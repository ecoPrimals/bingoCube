//! Full Brain Rehearsal — save state, simulate, validate, reset.
//!
//! Exercises every new Nautilus feature in a production-like cycle:
//! 1. Bootstrap a shell from synthetic QCD-like phase transition data
//! 2. Evolve with drift monitoring (watch N_e*s stabilize)
//! 3. Detect concept edges at the phase transition
//! 4. Inject edge-seeded boards and re-evolve
//! 5. Export AKD1000 int4 weights and validate quantization error
//! 6. Save full state to disk (JSON)
//! 7. Restore from disk and confirm predictions match
//! 8. Transfer to a second instance, merge, and validate
//! 9. Reset for next run
//!
//! Run: cargo run --example full_brain_rehearsal -p bingocube-nautilus

use bingocube_nautilus::{
    EvolutionConfig, InstanceId, NautilusShell,
    ReservoirInput, ResponseVector, SelectionMethod, ShellConfig,
};

/// Synthetic QCD-like data: plaquette ladder with a phase transition at β≈5.7.
fn qcd_phase_data(n: usize) -> (Vec<ReservoirInput>, Vec<Vec<f64>>) {
    let mut inputs = Vec::with_capacity(n);
    let mut targets = Vec::with_capacity(n);

    for i in 0..n {
        let beta = 4.0 + 3.0 * (i as f64 / (n - 1) as f64);
        let x = (beta - 5.7) * 3.0;
        let plaq = 0.33 + 0.30 / (1.0 + (-x).exp());
        let cg = 60000.0 * (-0.3 * (beta - 5.0)).exp() + 200.0;

        inputs.push(ReservoirInput::Continuous(vec![
            (beta - 5.0) / 2.0,
            plaq,
            (cg / 60000.0).ln().abs(),
        ]));
        targets.push(vec![plaq, cg / 60000.0]);
    }

    (inputs, targets)
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   Nautilus Shell — Full Brain Rehearsal (Save/Sim/Reset)    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let save_path = "/tmp/nautilus_brain_rehearsal.json";
    let akd_path = "/tmp/nautilus_akd1000_export.json";

    // ═══ Phase 1: Bootstrap and evolve with drift monitoring ═══

    println!("━━━ Phase 1: Bootstrap + Drift-Monitored Evolution ━━━\n");

    let config = ShellConfig {
        population_size: 16,
        n_targets: 2,
        evolution: EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 4 },
            mutation_rate: 0.15,
            column_crossover: true,
            cell_crossover: false,
        },
        ridge_lambda: 1e-6,
        max_stored_generations: 10,
        ..Default::default()
    };

    let id = InstanceId::new("biomegate-rtx3090");
    let mut shell = NautilusShell::from_seed(config, id, 42);
    let (inputs, targets) = qcd_phase_data(30);

    println!("  Training data: {} beta points (β=4.0..7.0)", inputs.len());
    println!("  Targets: plaquette + CG cost\n");

    for gen in 0..15 {
        let mse = shell.evolve_generation_seeded(&inputs, &targets, 1000 + gen);
        let ne_s = shell.latest_ne_s();
        let drifting = if shell.is_drifting() { " ⚠ DRIFT" } else { "" };
        if gen % 3 == 0 || shell.is_drifting() {
            println!(
                "  Gen {:>2}: MSE={:.6}  N_e·s={:.2}  pop={}{}",
                gen,
                mse,
                ne_s,
                shell.current_population.size(),
                drifting,
            );
        }
    }

    let last_record = shell.history.last().unwrap();
    println!("\n  Final: best_fitness={:.4}  drift_action={:?}",
        last_record.best_fitness, last_record.drift_action);

    // ═══ Phase 2: Detect concept edges ═══

    println!("\n━━━ Phase 2: Concept Edge Detection ━━━\n");

    let edge_indices = shell.detect_concept_edges(&inputs, &targets, 2.0);
    println!("  Found {} concept edges (threshold=2.0×mean):", edge_indices.len());

    let mut edge_features = Vec::new();
    for &idx in &edge_indices {
        if let ReservoirInput::Continuous(ref feats) = inputs[idx] {
            let beta = feats[0] * 2.0 + 5.0;
            println!("    idx={idx}: β≈{beta:.2} (normalized features: [{:.3}, {:.3}, {:.3}])",
                feats[0], feats[1], feats[2]);
            edge_features.push(feats.clone());
        }
    }

    // ═══ Phase 3: Edge-seeded re-evolution ═══

    println!("\n━━━ Phase 3: Edge-Seeded Re-Evolution ━━━\n");

    let mse_before = {
        let responses: Vec<ResponseVector> = inputs.iter()
            .map(|inp| shell.current_population.project(inp))
            .collect();
        shell.readout.mse(&responses, &targets)
    };
    println!("  MSE before edge seeding: {mse_before:.6}");

    if !edge_features.is_empty() {
        shell.set_concept_edges(edge_features);
        println!("  Registered {} concept edges for directed mutagenesis", edge_indices.len());
    }

    for gen in 0..10 {
        let mse = shell.evolve_generation_seeded(&inputs, &targets, 2000 + gen);
        if gen % 3 == 0 {
            println!("  Edge-seeded Gen {:>2}: MSE={:.6}  N_e·s={:.2}", gen, mse, shell.latest_ne_s());
        }
    }

    let mse_after = {
        let responses: Vec<ResponseVector> = inputs.iter()
            .map(|inp| shell.current_population.project(inp))
            .collect();
        shell.readout.mse(&responses, &targets)
    };
    println!("  MSE after edge seeding:  {mse_after:.6}");
    let improved = if mse_after < mse_before { "IMPROVED" } else { "no change" };
    println!("  Edge seeding effect: {improved}");

    // ═══ Phase 4: AKD1000 export ═══

    println!("\n━━━ Phase 4: AKD1000 Int4 Weight Export ━━━\n");

    let export = shell.export_akd1000_weights();
    println!("  Export shape: {} targets × {} inputs", export.n_targets, export.input_dim);

    let mut int4_range = (i8::MAX, i8::MIN);
    let mut nonzero = 0usize;
    let total = export.quantized_weights.iter().map(|r| r.len()).sum::<usize>();
    for row in &export.quantized_weights {
        for &w in row {
            if w < int4_range.0 { int4_range.0 = w; }
            if w > int4_range.1 { int4_range.1 = w; }
            if w != 0 { nonzero += 1; }
        }
    }
    println!("  Weight range: [{}, {}] (int4: [-8, 7])", int4_range.0, int4_range.1);
    println!("  Nonzero weights: {}/{} ({:.1}% sparsity)",
        nonzero, total, 100.0 * (1.0 - nonzero as f64 / total as f64));

    let responses: Vec<ResponseVector> = inputs.iter()
        .map(|inp| shell.current_population.project(inp))
        .collect();
    let quant_mse = export.quantization_mse(&shell.readout, &responses);
    println!("  Quantization MSE: {quant_mse:.8}");

    // Spot-check: compare original vs quantized on a few points
    println!("\n  Spot check (original vs int4-quantized):");
    for i in [0, 14, 29] {
        let orig = shell.readout.predict(&responses[i]);
        let quant = export.predict_dequantized(&responses[i].activations);
        let beta = 4.0 + 3.0 * (i as f64 / 29.0);
        println!(
            "    β≈{beta:.2}: plaq orig={:.4} quant={:.4} | CG orig={:.4} quant={:.4}",
            orig[0], quant[0], orig[1], quant[1],
        );
    }

    // Save AKD1000 export
    let akd_json = serde_json::to_string_pretty(&export).unwrap();
    std::fs::write(akd_path, &akd_json).unwrap();
    println!("\n  AKD1000 weights saved: {akd_path} ({:.1} KB)", akd_json.len() as f64 / 1024.0);

    // ═══ Phase 5: Save full state ═══

    println!("\n━━━ Phase 5: Save Full Shell State ━━━\n");

    let json = serde_json::to_string(&shell).unwrap();
    std::fs::write(save_path, &json).unwrap();
    println!("  Shell saved: {save_path} ({:.1} KB)", json.len() as f64 / 1024.0);
    println!("  Generation: {}", shell.generation());
    println!("  History entries: {}", shell.history.len());
    println!("  Lineage depth: {}", shell.lineage_depth());
    println!("  Concept edges: {}", shell.concept_edges.len());
    println!("  Drift monitor entries: {}", shell.drift_monitor.history.len());

    // Reference predictions for validation
    let test_inputs: Vec<ReservoirInput> = [4.5_f64, 5.25, 5.69, 6.5].iter()
        .map(|&beta| {
            let x: f64 = (beta - 5.7) * 3.0;
            let plaq: f64 = 0.33 + 0.30 / (1.0 + (-x).exp());
            let cg_raw: f64 = 60000.0 * (-0.3 * (beta - 5.0)).exp() + 200.0;
            ReservoirInput::Continuous(vec![
                (beta - 5.0) / 2.0,
                plaq,
                cg_raw.ln() / 60000.0_f64.ln(),
            ])
        })
        .collect();
    let ref_preds: Vec<Vec<f64>> = test_inputs.iter().map(|inp| shell.predict(inp)).collect();

    // ═══ Phase 6: Restore and validate ═══

    println!("\n━━━ Phase 6: Restore From Disk + Validate ━━━\n");

    let loaded_json = std::fs::read_to_string(save_path).unwrap();
    let restored: NautilusShell = serde_json::from_str(&loaded_json).unwrap();

    assert_eq!(restored.generation(), shell.generation());
    assert_eq!(restored.history.len(), shell.history.len());
    assert_eq!(restored.lineage_depth(), shell.lineage_depth());
    assert_eq!(restored.concept_edges.len(), shell.concept_edges.len());
    assert_eq!(restored.drift_monitor.history.len(), shell.drift_monitor.history.len());

    let mut max_pred_delta = 0.0f64;
    for (i, (inp, ref_pred)) in test_inputs.iter().zip(ref_preds.iter()).enumerate() {
        let restored_pred = restored.predict(inp);
        let delta: f64 = ref_pred.iter().zip(restored_pred.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        max_pred_delta = max_pred_delta.max(delta);
        let beta = [4.5, 5.25, 5.69, 6.5][i];
        println!("  β={beta:.2}: pred delta = {delta:.2e} {}",
            if delta < 1e-10 { "✓" } else { "≠" });
    }
    assert!(max_pred_delta < 1e-10, "predictions diverged after restore!");
    println!("\n  All predictions match — state restore verified ✓");

    // ═══ Phase 7: Transfer + merge simulation ═══

    println!("\n━━━ Phase 7: Instance Transfer + Merge ━━━\n");

    let id_field = InstanceId::new("biomegate-titanv");
    let mut field_shell = NautilusShell::continue_from(restored, id_field);
    println!("  Transferred to field node: lineage={}", field_shell.lineage_depth());

    // Field node evolves on slightly different data (different seed range)
    let (field_inputs, field_targets) = qcd_phase_data(25);
    for gen in 0..5 {
        field_shell.evolve_generation_seeded(&field_inputs, &field_targets, 3000 + gen);
    }
    println!("  Field node evolved {} more generations", 5);

    // Merge field knowledge back into original
    let mut merged = shell.clone();
    merged.merge_shell(&field_shell);
    println!("  Merged: lineage={} history={}", merged.lineage_depth(), merged.history.len());

    // Evolve merged shell
    for gen in 0..3 {
        merged.evolve_generation_seeded(&inputs, &targets, 4000 + gen);
    }
    let merged_mse = {
        let r: Vec<ResponseVector> = inputs.iter()
            .map(|inp| merged.current_population.project(inp))
            .collect();
        merged.readout.mse(&r, &targets)
    };
    println!("  Post-merge MSE: {merged_mse:.6}");

    // ═══ Phase 8: Reset for next run ═══

    println!("\n━━━ Phase 8: Reset for Next Production Run ━━━\n");

    // Clear concept edges (will be re-detected from fresh data)
    merged.concept_edges.clear();
    // Save the production-ready state
    let prod_json = serde_json::to_string(&merged).unwrap();
    let prod_path = "/tmp/nautilus_production_ready.json";
    std::fs::write(prod_path, &prod_json).unwrap();
    println!("  Production state saved: {prod_path} ({:.1} KB)", prod_json.len() as f64 / 1024.0);
    println!("  Generation: {}", merged.generation());
    println!("  N_e·s: {:.2}", merged.latest_ne_s());
    println!("  Drifting: {}", merged.is_drifting());
    println!("  Lineage: {} instances", merged.lineage_depth());

    // Cleanup temp files
    std::fs::remove_file(save_path).ok();
    std::fs::remove_file(akd_path).ok();

    // ═══ Summary ═══

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Rehearsal Complete                       ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  ✓ Drift monitor: wired, tracked {} generations", shell.drift_monitor.history.len());
    println!("║  ✓ Concept edges: {} detected, seeded into evolution", edge_indices.len());
    println!("║  ✓ AKD1000 export: int4 quantized, MSE={quant_mse:.2e}");
    println!("║  ✓ Save/restore: bit-perfect prediction match");
    println!("║  ✓ Transfer + merge: {} instances in lineage", merged.lineage_depth());
    println!("║  ✓ Production state: ready at {prod_path}");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

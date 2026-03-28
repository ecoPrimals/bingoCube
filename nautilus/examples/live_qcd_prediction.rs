//! Live QCD Trajectory Prediction via Nautilus Shell
//!
//! Replays actual dynamical QCD trajectory data from hotSpring Exp 024 + 028
//! through the Nautilus Shell, evolving the board population in real time.
//!
//! The shell predicts CG solver cost and plaquette from (β, acceptance, δH)
//! and evolves its boards to specialize on the lattice QCD landscape.
//!
//! Run from the hotSpring root (needs access to results/):
//!   cd /path/to/hotSpring
//!   cargo run --example live_qcd_prediction -p bingocube-nautilus
//!
//! Or provide the data path explicitly:
//!   DATA_DIR=/path/to/hotSpring/results cargo run --example live_qcd_prediction -p bingocube-nautilus

use bingocube_nautilus::{
    EvolutionConfig, InstanceId, NautilusShell, ReservoirInput, SelectionMethod, ShellConfig,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct TrajectoryRecord {
    beta: f64,
    plaquette: f64,
    accepted: bool,
    cg_iters: u64,
    delta_h: f64,
    phase: String,
    #[allow(dead_code)]
    traj_idx: u64,
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║   Nautilus Shell — Live QCD Trajectory Prediction              ║");
    println!("║   Data: hotSpring Exp 024 + 028 (1,336 measurement records)   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // ─── Load trajectory data ───

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "results".to_string());
    let files = [
        format!("{}/exp024_production_8x8.jsonl", data_dir),
        format!("{}/exp028_brain_production_8x8.jsonl", data_dir),
    ];

    let mut all_records = Vec::new();
    for path in &files {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut count = 0;
                for line in content.lines() {
                    if let Ok(rec) = serde_json::from_str::<TrajectoryRecord>(line) {
                        all_records.push(rec);
                        count += 1;
                    }
                }
                println!("  Loaded {count} records from {path}");
            }
            Err(e) => {
                eprintln!("  Warning: could not read {path}: {e}");
            }
        }
    }

    // Filter to measurement phase
    let measurements: Vec<&TrajectoryRecord> = all_records
        .iter()
        .filter(|r| r.phase == "measurement")
        .collect();

    println!("  Total measurement records: {}", measurements.len());

    if measurements.is_empty() {
        eprintln!("\nNo measurement records found. Run from hotSpring root or set DATA_DIR.");
        return;
    }

    // ─── Aggregate per-beta summaries ───

    let mut beta_data: BTreeMap<String, Vec<&TrajectoryRecord>> = BTreeMap::new();
    for rec in &measurements {
        let key = format!("{:.3}", rec.beta);
        beta_data.entry(key).or_default().push(rec);
    }

    println!("  Unique β values: {}", beta_data.len());
    println!();

    // Build feature vectors and targets from per-beta aggregates
    // Features: (β, mean_plaq, acceptance_rate, mean_|δH|, log_mean_cg)
    // Targets: (predicted_cg_cost_normalized, predicted_plaquette)

    let mut inputs = Vec::new();
    let mut targets = Vec::new();
    let mut beta_labels = Vec::new();

    let max_cg: f64 = measurements
        .iter()
        .map(|r| r.cg_iters as f64)
        .fold(0.0, f64::max);

    for (beta_key, recs) in &beta_data {
        let n = recs.len() as f64;
        let beta: f64 = recs[0].beta;
        let mean_plaq = recs.iter().map(|r| r.plaquette).sum::<f64>() / n;
        let acc_rate = recs.iter().filter(|r| r.accepted).count() as f64 / n;
        let mean_abs_dh = recs.iter().map(|r| r.delta_h.abs()).sum::<f64>() / n;
        let mean_cg = recs.iter().map(|r| r.cg_iters as f64).sum::<f64>() / n;

        inputs.push(ReservoirInput::Continuous(vec![
            beta / 7.0,                  // normalized β
            mean_plaq,                   // already [0, 1]
            acc_rate,                    // already [0, 1]
            mean_abs_dh.min(1.0),        // clamp
            (mean_cg + 1.0).ln() / 12.0, // log-normalized CG
        ]));

        targets.push(vec![
            mean_cg / max_cg, // normalized CG cost
            mean_plaq,        // plaquette (self-prediction as sanity check)
        ]);

        beta_labels.push((beta_key.clone(), beta, mean_cg, mean_plaq, acc_rate));
    }

    // ─── Create Nautilus Shell ───

    let config = ShellConfig {
        population_size: 24,
        n_targets: 2,
        evolution: EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 6 },
            mutation_rate: 0.12,
            column_crossover: true,
            cell_crossover: false,
        },
        ridge_lambda: 1e-4,
        max_stored_generations: 50,
        ..Default::default()
    };

    let instance = InstanceId::new("hotspring-northgate");
    let mut shell = NautilusShell::from_seed(config, instance, 42);

    println!("  Nautilus Shell initialized:");
    println!(
        "    Population: {} boards (5×5)",
        shell.current_population.size()
    );
    println!(
        "    Response dim: {}",
        shell.current_population.response_dim()
    );
    println!("    Targets: CG cost (normalized), plaquette");
    println!();

    // ─── Evolve and predict ───

    let n_generations = 40;

    println!(
        "━━━ Evolution: {} generations on {} β-point summaries ━━━\n",
        n_generations,
        inputs.len()
    );
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}",
        "Gen", "MSE", "Mean Fit", "Best Fit"
    );
    println!("  {:─>4}  {:─>10}  {:─>10}  {:─>10}", "", "", "", "");

    for gen in 0..n_generations {
        let mse = shell.evolve_generation_seeded(&inputs, &targets, 1000 + gen);
        let traj = shell.fitness_trajectory();
        let last = traj.last().unwrap();
        if gen % 5 == 0 || gen == n_generations - 1 {
            println!(
                "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}",
                last.0, mse, last.1, last.2
            );
        }
    }

    // ─── Per-β predictions ───

    println!(
        "\n━━━ Per-β Predictions (after {} generations) ━━━\n",
        n_generations
    );
    println!(
        "  {:>7}  {:>10}  {:>10}  {:>8}  {:>10}  {:>10}",
        "β", "CG actual", "CG pred", "Acc%", "Plaq act", "Plaq pred"
    );
    println!(
        "  {:─>7}  {:─>10}  {:─>10}  {:─>8}  {:─>10}  {:─>10}",
        "", "", "", "", "", ""
    );

    let mut total_cg_err = 0.0;
    let mut total_plaq_err = 0.0;

    for (i, (beta_key, _beta, mean_cg, mean_plaq, acc_rate)) in beta_labels.iter().enumerate() {
        let pred = shell.predict(&inputs[i]);
        let pred_cg = pred[0] * max_cg;
        let pred_plaq = pred[1];

        let cg_err = (pred_cg - mean_cg).abs() / mean_cg;
        let plaq_err = (pred_plaq - mean_plaq).abs();
        total_cg_err += cg_err;
        total_plaq_err += plaq_err;

        let marker = if cg_err > 0.15 { " !" } else { "" };

        println!(
            "  {:>7}  {:>10.0}  {:>10.0}  {:>7.1}%  {:>10.4}  {:>10.4}{}",
            beta_key,
            mean_cg,
            pred_cg,
            acc_rate * 100.0,
            mean_plaq,
            pred_plaq,
            marker
        );
    }

    let n_betas = beta_labels.len() as f64;
    println!(
        "\n  Mean CG relative error: {:.1}%",
        total_cg_err / n_betas * 100.0
    );
    println!(
        "  Mean plaquette absolute error: {:.4}",
        total_plaq_err / n_betas
    );

    // ─── Leave-one-out cross-validation ───

    println!("\n━━━ Leave-One-Out Cross-Validation ━━━\n");
    println!(
        "  {:>7}  {:>10}  {:>10}  {:>8}",
        "β (held)", "CG actual", "CG pred", "Rel Err"
    );
    println!("  {:─>7}  {:─>10}  {:─>10}  {:─>8}", "", "", "", "");

    let mut loo_total_err = 0.0;
    for hold_out in 0..inputs.len() {
        let train_inputs: Vec<ReservoirInput> = inputs
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != hold_out)
            .map(|(_, x)| x.clone())
            .collect();
        let train_targets: Vec<Vec<f64>> = targets
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != hold_out)
            .map(|(_, t)| t.clone())
            .collect();

        let loo_config = ShellConfig {
            population_size: 16,
            n_targets: 2,
            evolution: EvolutionConfig {
                selection: SelectionMethod::Elitism { survivors: 4 },
                mutation_rate: 0.12,
                column_crossover: true,
                cell_crossover: false,
            },
            ridge_lambda: 1e-3,
            max_stored_generations: 20,
            ..Default::default()
        };

        let loo_instance = InstanceId::new("loo-validator");
        let mut loo_shell =
            NautilusShell::from_seed(loo_config, loo_instance, 42 + hold_out as u64);

        for gen in 0..20 {
            loo_shell.evolve_generation_seeded(&train_inputs, &train_targets, 2000 + gen);
        }

        let pred = loo_shell.predict(&inputs[hold_out]);
        let pred_cg = pred[0] * max_cg;
        let actual_cg = beta_labels[hold_out].2;
        let rel_err = (pred_cg - actual_cg).abs() / actual_cg;
        loo_total_err += rel_err;

        let marker = if rel_err > 0.20 { " !" } else { "" };
        println!(
            "  {:>7}  {:>10.0}  {:>10.0}  {:>7.1}%{}",
            beta_labels[hold_out].0,
            actual_cg,
            pred_cg,
            rel_err * 100.0,
            marker
        );
    }

    println!(
        "\n  Mean LOO CG relative error: {:.1}%",
        loo_total_err / inputs.len() as f64 * 100.0
    );

    // ─── Shell summary ───

    let serialized = serde_json::to_string(&shell).unwrap();
    println!("\n━━━ Shell Summary ━━━\n");
    println!("  Generations evolved: {}", shell.generation());
    println!("  Shell size: {:.1} KB", serialized.len() as f64 / 1024.0);
    println!(
        "  Lineage: {:?}",
        shell.lineage.iter().map(|l| l.name()).collect::<Vec<_>>()
    );
    println!("  History layers: {}", shell.history.len());
    println!(
        "  Best fitness achieved: {:.4} (gen {})",
        shell
            .history
            .iter()
            .map(|r| r.best_fitness)
            .fold(0.0_f64, f64::max),
        shell
            .history
            .iter()
            .max_by(|a, b| a.best_fitness.partial_cmp(&b.best_fitness).unwrap())
            .map(|r| r.generation)
            .unwrap_or(0),
    );
    println!("\n  This shell can be serialized and shipped to another instance");
    println!("  (e.g. a field AKD1000 NPU) to continue evolving on new data.\n");
}

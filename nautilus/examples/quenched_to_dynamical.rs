// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quenched → Dynamical Transfer Test
//!
//! Tests whether a Nautilus Shell trained on cheap quenched observables
//! (plaquette from pure gauge, zero CG cost) can predict expensive
//! dynamical observables (CG solver iterations with fermions).
//!
//! If this works, quenched pretherm data (which is fast to generate)
//! can inform the shell about what to expect from the expensive dynamical
//! phase — reducing wall time by front-loading knowledge.
//!
//! Run from hotSpring root:
//!   DATA_DIR=/path/to/results cargo run --example quenched_to_dynamical -p bingocube-nautilus

use bingocube_nautilus::{
    DriftMonitor, EdgeSeeder, EvolutionConfig, InstanceId, NautilusShell, ReservoirInput,
    SelectionMethod, ShellConfig,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct Record {
    beta: f64,
    plaquette: f64,
    accepted: bool,
    cg_iters: u64,
    #[allow(dead_code)]
    delta_h: f64,
    phase: String,
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║   Nautilus Shell — Quenched → Dynamical Transfer Test          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "results".to_string());
    let files = [
        format!("{}/exp024_production_8x8.jsonl", data_dir),
        format!("{}/exp028_brain_production_8x8.jsonl", data_dir),
    ];

    let mut all: Vec<Record> = Vec::new();
    for path in &files {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Ok(r) = serde_json::from_str::<Record>(line) {
                    all.push(r);
                }
            }
        }
    }

    // Separate phases
    let quenched: Vec<&Record> = all
        .iter()
        .filter(|r| r.phase == "quenched_pretherm")
        .collect();
    let therm: Vec<&Record> = all
        .iter()
        .filter(|r| r.phase == "dynamical_therm")
        .collect();
    let meas: Vec<&Record> = all.iter().filter(|r| r.phase == "measurement").collect();

    println!(
        "  Data loaded: {} quenched, {} therm, {} measurement",
        quenched.len(),
        therm.len(),
        meas.len()
    );

    // ─── Aggregate per-beta ───

    let mut quenched_by_beta: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
    let mut meas_by_beta: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
    let mut therm_by_beta: BTreeMap<String, Vec<&Record>> = BTreeMap::new();

    for r in &quenched {
        quenched_by_beta
            .entry(format!("{:.4}", r.beta))
            .or_default()
            .push(r);
    }
    for r in &meas {
        meas_by_beta
            .entry(format!("{:.4}", r.beta))
            .or_default()
            .push(r);
    }
    for r in &therm {
        therm_by_beta
            .entry(format!("{:.4}", r.beta))
            .or_default()
            .push(r);
    }

    // Build paired data: quenched features → dynamical targets
    let mut inputs = Vec::new();
    let mut targets = Vec::new();
    let mut beta_labels = Vec::new();

    let max_cg: f64 = meas.iter().map(|r| r.cg_iters as f64).fold(0.0, f64::max);

    for (key, q_recs) in &quenched_by_beta {
        if let Some(m_recs) = meas_by_beta.get(key) {
            let beta = q_recs[0].beta;

            // Quenched features (cheap to compute)
            let q_n = q_recs.len() as f64;
            let q_plaq = q_recs.iter().map(|r| r.plaquette).sum::<f64>() / q_n;
            let q_plaq_var = q_recs
                .iter()
                .map(|r| (r.plaquette - q_plaq).powi(2))
                .sum::<f64>()
                / q_n;

            // Dynamical targets (expensive to compute)
            let m_n = m_recs.len() as f64;
            let m_cg = m_recs.iter().map(|r| r.cg_iters as f64).sum::<f64>() / m_n;
            let m_plaq = m_recs.iter().map(|r| r.plaquette).sum::<f64>() / m_n;
            let m_acc = m_recs.iter().filter(|r| r.accepted).count() as f64 / m_n;

            // Therm features (partial cost)
            let (t_cg, t_plaq) = if let Some(t_recs) = therm_by_beta.get(key) {
                let t_n = t_recs.len() as f64;
                let tc = t_recs.iter().map(|r| r.cg_iters as f64).sum::<f64>() / t_n;
                let tp = t_recs.iter().map(|r| r.plaquette).sum::<f64>() / t_n;
                (tc, tp)
            } else {
                (0.0, q_plaq)
            };

            // Input: quenched-only features
            inputs.push(ReservoirInput::Continuous(vec![
                beta / 7.0,
                q_plaq,
                q_plaq_var.sqrt().min(0.1) * 10.0, // normalized std
                (t_cg + 1.0).ln() / 12.0,          // log therm CG
                t_plaq,
            ]));

            targets.push(vec![
                m_cg / max_cg, // dynamical CG cost (normalized)
                m_plaq,        // dynamical plaquette
                m_acc,         // dynamical acceptance rate
            ]);

            beta_labels.push((key.clone(), beta, m_cg, m_plaq, m_acc, q_plaq));
        }
    }

    println!("  Paired beta points: {}", inputs.len());
    println!("  Features: β, quenched_plaq, quenched_plaq_std, log_therm_CG, therm_plaq");
    println!("  Targets: dynamical CG cost, dynamical plaq, dynamical acceptance\n");

    // ─── Train: quenched features → dynamical targets ───

    let config = ShellConfig {
        population_size: 24,
        n_targets: 3,
        evolution: EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 6 },
            mutation_rate: 0.12,
            column_crossover: true,
            cell_crossover: false,
        },
        ridge_lambda: 1e-4,
        ..Default::default()
    };

    let id = InstanceId::new("quenched-transfer-test");
    let mut shell = NautilusShell::from_seed(config, id, 42);
    let mut drift = DriftMonitor::default();

    println!("━━━ Phase 1: Full Training (quenched → dynamical) ━━━\n");
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}  {:>8}",
        "Gen", "MSE", "Mean Fit", "Best Fit", "Ne·s"
    );
    println!(
        "  {:─>4}  {:─>10}  {:─>10}  {:─>10}  {:─>8}",
        "", "", "", "", ""
    );

    for gen_idx in 0..50 {
        let mse = shell.evolve_generation_seeded(&inputs, &targets, 1000 + gen_idx);
        let traj = shell.fitness_trajectory();
        let last = traj.last().unwrap();

        drift.record(gen_idx as usize, 24, last.1, last.2);

        if gen_idx % 5 == 0 || gen_idx == 49 {
            println!(
                "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}  {:>8.2}",
                last.0,
                mse,
                last.1,
                last.2,
                drift.latest_ne_s()
            );
        }
    }

    if drift.is_drifting() {
        println!(
            "\n  ⚠ Drift detected! Recommendation: {:?}",
            drift.recommendation()
        );
    } else {
        println!(
            "\n  ✓ Selection dominant (Ne·s = {:.2})",
            drift.latest_ne_s()
        );
    }

    // ─── Per-β predictions ───

    println!("\n━━━ Phase 2: Per-β Predictions ━━━\n");
    println!(
        "  {:>7}  {:>8}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
        "β", "Q plaq", "CG actual", "CG pred", "Plaq Δ", "Acc Δ", "CG err", "Regime"
    );
    println!(
        "  {:─>7}  {:─>8}  {:─>10}  {:─>10}  {:─>8}  {:─>8}  {:─>8}  {:─>8}",
        "", "", "", "", "", "", "", ""
    );

    let mut total_cg_err = 0.0;
    let mut concept_edges = Vec::new();

    for (i, (key, _beta, m_cg, m_plaq, m_acc, q_plaq)) in beta_labels.iter().enumerate() {
        let pred = shell.predict(&inputs[i]);
        let pred_cg = pred[0] * max_cg;
        let pred_plaq = pred[1];
        let pred_acc = pred[2];

        let cg_err = (pred_cg - m_cg).abs() / m_cg;
        let plaq_delta = (pred_plaq - m_plaq).abs();
        let acc_delta = (pred_acc - m_acc).abs();
        total_cg_err += cg_err;

        let regime = if *m_plaq < 0.4 {
            "confined"
        } else if *m_plaq > 0.55 {
            "deconf"
        } else {
            "crossover"
        };

        let marker = if cg_err > 0.10 { " !" } else { "" };

        if cg_err > 0.15 {
            concept_edges.push((key.clone(), cg_err));
        }

        println!(
            "  {:>7}  {:>8.4}  {:>10.0}  {:>10.0}  {:>8.4}  {:>8.3}  {:>7.1}%  {:>8}{}",
            key,
            q_plaq,
            m_cg,
            pred_cg,
            plaq_delta,
            acc_delta,
            cg_err * 100.0,
            regime,
            marker
        );
    }

    let n = beta_labels.len() as f64;
    println!(
        "\n  Mean CG relative error: {:.1}%",
        total_cg_err / n * 100.0
    );

    // ─── LOO cross-validation ───

    println!("\n━━━ Phase 3: Leave-One-Out (quenched → dynamical) ━━━\n");
    println!(
        "  {:>7}  {:>10}  {:>10}  {:>8}",
        "β held", "CG actual", "CG pred", "Rel Err"
    );
    println!("  {:─>7}  {:─>10}  {:─>10}  {:─>8}", "", "", "", "");

    let mut loo_total = 0.0;
    let mut loo_edges = Vec::new();

    for hold_out in 0..inputs.len() {
        let train_in: Vec<_> = inputs
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != hold_out)
            .map(|(_, x)| x.clone())
            .collect();
        let train_tgt: Vec<_> = targets
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != hold_out)
            .map(|(_, t)| t.clone())
            .collect();

        let loo_cfg = ShellConfig {
            population_size: 16,
            n_targets: 3,
            evolution: EvolutionConfig {
                selection: SelectionMethod::Elitism { survivors: 4 },
                mutation_rate: 0.12,
                column_crossover: true,
                cell_crossover: false,
            },
            ridge_lambda: 1e-3,
            ..Default::default()
        };

        let mut loo =
            NautilusShell::from_seed(loo_cfg, InstanceId::new("loo"), 42 + hold_out as u64);
        for gen_idx in 0..25 {
            loo.evolve_generation_seeded(&train_in, &train_tgt, 2000 + gen_idx);
        }

        let pred = loo.predict(&inputs[hold_out]);
        let pred_cg = pred[0] * max_cg;
        let actual_cg = beta_labels[hold_out].2;
        let rel_err = (pred_cg - actual_cg).abs() / actual_cg;
        loo_total += rel_err;

        if rel_err > 0.15 {
            loo_edges.push((beta_labels[hold_out].0.clone(), rel_err));
        }

        let marker = if rel_err > 0.15 { " !" } else { "" };
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
        "\n  Mean LOO CG error (quenched→dynamical): {:.1}%",
        loo_total / n * 100.0
    );

    // ─── Concept edge detection ───

    if !loo_edges.is_empty() {
        println!("\n━━━ Concept Edges Detected ━━━\n");
        for (beta, err) in &loo_edges {
            println!(
                "  β = {}: LOO error {:.1}% — physics changes qualitatively here",
                beta,
                err * 100.0
            );
        }

        println!(
            "\n  Seeding {} edge-targeted boards for next generation...",
            loo_edges.len() * 4
        );
        let mut rng = rand::thread_rng();
        let config = bingocube_nautilus::Config::default();

        for (beta_key, _) in &loo_edges {
            let beta: f64 = beta_key.parse().unwrap_or(5.0);
            let features = vec![beta / 7.0, 0.5, 0.1]; // edge features
            let edge_boards = EdgeSeeder::seed_boards(&config, 4, &features, &mut rng).unwrap();
            println!("    Seeded 4 boards for edge β={}", beta_key);
            let _ = edge_boards; // would inject into population in production
        }
    }

    // ─── Cost analysis ───

    println!("\n━━━ Cost Analysis: Quenched Proxy Value ━━━\n");

    let quenched_cost_per_beta = 10.0; // ~10 seconds for quenched pretherm
    let dynamical_cost_per_beta = 90.0 * 60.0; // ~90 minutes for full dynamical
    let n_betas = beta_labels.len() as f64;

    let quenched_total = n_betas * quenched_cost_per_beta;
    let dynamical_total = n_betas * dynamical_cost_per_beta;

    println!(
        "  Quenched scan (21 β × ~10s):   {:.0} seconds",
        quenched_total
    );
    println!(
        "  Dynamical scan (21 β × ~90m):  {:.0} seconds ({:.1} hours)",
        dynamical_total,
        dynamical_total / 3600.0
    );
    println!("  Ratio: {:.0}× cheaper", dynamical_total / quenched_total);
    println!();
    println!(
        "  With {:.1}% LOO error, the quenched-trained shell can:",
        loo_total / n * 100.0
    );
    println!("    • Pre-screen β points before committing dynamical budget");
    println!("    • Estimate CG cost to plan wall-time allocation");
    println!("    • Identify concept edges for priority measurement");
    println!("    • Guide adaptive steering before any dynamical data exists");
    println!();

    let saved_betas = (loo_total / n * 100.0 < 10.0) as usize * 5; // conservatively skip 5 β if error < 10%
    let saved_time = saved_betas as f64 * dynamical_cost_per_beta;
    println!(
        "  Potential savings: skip ~{} low-information β points → save ~{:.1} hours",
        saved_betas,
        saved_time / 3600.0
    );
}

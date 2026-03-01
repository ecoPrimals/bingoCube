//! Predict Live Exp 029 Data Using Shell Trained on Historical Data
//!
//! Trains the Nautilus Shell on Exp 024+028 (historical), then makes
//! blind predictions on Exp 029 (live production run) — true out-of-sample
//! validation on data the shell has never seen.
//!
//! Run from the hotSpring root:
//!   cd /path/to/hotSpring
//!   cargo run --release --example predict_live_exp029 -p bingocube-nautilus

use bingocube_nautilus::{
    EvolutionConfig, InstanceId, NautilusShell, ReservoirInput, SelectionMethod, ShellConfig,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct TrajectoryRecord {
    beta: f64,
    plaquette: f64,
    #[serde(default)]
    accepted: bool,
    #[serde(default)]
    cg_iters: u64,
    #[serde(default)]
    delta_h: f64,
    #[serde(default)]
    phase: String,
    #[serde(default)]
    acceptance: f64,
}

struct BetaSummary {
    beta: f64,
    mean_plaq: f64,
    acc_rate: f64,
    mean_abs_dh: f64,
    mean_cg: f64,
    n_trajs: usize,
}

fn load_and_aggregate(paths: &[String]) -> Vec<BetaSummary> {
    let mut all_records = Vec::new();
    for path in paths {
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
            Err(e) => eprintln!("  Warning: {path}: {e}"),
        }
    }

    let measurements: Vec<&TrajectoryRecord> = all_records
        .iter()
        .filter(|r| r.phase == "measurement")
        .collect();

    let mut beta_data: BTreeMap<String, Vec<&TrajectoryRecord>> = BTreeMap::new();
    for rec in &measurements {
        let key = format!("{:.4}", rec.beta);
        beta_data.entry(key).or_default().push(rec);
    }

    beta_data
        .values()
        .map(|recs| {
            let n = recs.len() as f64;
            let beta = recs[0].beta;
            let mean_plaq = recs.iter().map(|r| r.plaquette).sum::<f64>() / n;
            let acc_rate = if recs[0].acceptance > 0.0 {
                recs[0].acceptance
            } else {
                recs.iter().filter(|r| r.accepted).count() as f64 / n
            };
            let mean_abs_dh = recs.iter().map(|r| r.delta_h.abs()).sum::<f64>() / n;
            let mean_cg = recs.iter().map(|r| r.cg_iters as f64).sum::<f64>() / n;
            BetaSummary {
                beta,
                mean_plaq,
                acc_rate,
                mean_abs_dh,
                mean_cg,
                n_trajs: recs.len(),
            }
        })
        .collect()
}

fn make_inputs(summaries: &[BetaSummary]) -> Vec<ReservoirInput> {
    summaries
        .iter()
        .map(|s| {
            ReservoirInput::Continuous(vec![
                s.beta / 7.0,
                s.mean_plaq,
                s.acc_rate,
                s.mean_abs_dh.min(1.0),
                (s.mean_cg + 1.0).ln() / 12.0,
            ])
        })
        .collect()
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Nautilus Shell — Blind Prediction on Live Exp 029 Data        ║");
    println!("║  Train: Exp 024+028 (historical)  Test: Exp 029 (live)        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let data_dir = std::env::var("DATA_DIR")
        .unwrap_or_else(|_| "results".to_string());

    // ─── Load training data (historical) ───
    println!("━━━ Training Data (Exp 024 + 028) ━━━\n");
    let train_summaries = load_and_aggregate(&[
        format!("{}/exp024_production_8x8.jsonl", data_dir),
        format!("{}/exp028_brain_production_8x8.jsonl", data_dir),
    ]);

    if train_summaries.is_empty() {
        eprintln!("\nNo training data. Run from hotSpring root or set DATA_DIR.");
        return;
    }

    let max_cg: f64 = train_summaries
        .iter()
        .map(|s| s.mean_cg)
        .fold(0.0, f64::max);

    let train_inputs = make_inputs(&train_summaries);
    let train_targets: Vec<Vec<f64>> = train_summaries
        .iter()
        .map(|s| vec![s.mean_cg / max_cg, s.mean_plaq])
        .collect();

    println!("  Training β points: {}", train_summaries.len());
    println!("  Max CG for normalization: {:.0}\n", max_cg);

    // ─── Load test data (live Exp 029) ───
    println!("━━━ Test Data (Exp 029 — Live Production) ━━━\n");
    let test_summaries = load_and_aggregate(&[
        format!("{}/exp029_npu_steering_8x8.jsonl", data_dir),
    ]);

    if test_summaries.is_empty() {
        eprintln!("\nNo Exp 029 data yet. Is the run active?");
        return;
    }

    let test_inputs = make_inputs(&test_summaries);

    println!("  Test β points: {}\n", test_summaries.len());

    // ─── Create and train Nautilus Shell ───
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

    let instance = InstanceId::new("hotspring-biomeGate");
    let mut shell = NautilusShell::from_seed(config, instance, 42);

    println!("━━━ Evolution: 40 generations on {} training points ━━━\n", train_inputs.len());
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}",
        "Gen", "MSE", "Mean Fit", "Best Fit"
    );
    println!(
        "  {:─>4}  {:─>10}  {:─>10}  {:─>10}",
        "", "", "", ""
    );

    for gen in 0..40 {
        let mse = shell.evolve_generation_seeded(&train_inputs, &train_targets, 1000 + gen);
        let traj = shell.fitness_trajectory();
        let last = traj.last().unwrap();
        if gen % 10 == 0 || gen == 39 {
            println!(
                "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}",
                last.0, mse, last.1, last.2
            );
        }
    }

    // ─── Training fit (sanity check) ───
    println!("\n━━━ Training Fit (sanity check) ━━━\n");
    println!(
        "  {:>7}  {:>8}  {:>10}  {:>10}  {:>8}",
        "β", "Trajs", "CG actual", "CG pred", "Rel Err"
    );
    println!(
        "  {:─>7}  {:─>8}  {:─>10}  {:─>10}  {:─>8}",
        "", "", "", "", ""
    );

    let mut train_err_total = 0.0;
    for (i, s) in train_summaries.iter().enumerate() {
        let pred = shell.predict(&train_inputs[i]);
        let pred_cg = pred[0] * max_cg;
        let rel_err = (pred_cg - s.mean_cg).abs() / s.mean_cg;
        train_err_total += rel_err;
        println!(
            "  {:>7.4}  {:>8}  {:>10.0}  {:>10.0}  {:>7.1}%",
            s.beta, s.n_trajs, s.mean_cg, pred_cg, rel_err * 100.0
        );
    }
    println!(
        "\n  Mean training CG error: {:.1}%\n",
        train_err_total / train_summaries.len() as f64 * 100.0
    );

    // ─── BLIND PREDICTIONS on Exp 029 ───
    println!("━━━ BLIND Predictions on Exp 029 (never seen during training) ━━━\n");
    println!(
        "  {:>7}  {:>8}  {:>10}  {:>10}  {:>8}  {:>10}  {:>10}",
        "β", "Trajs", "CG actual", "CG pred", "CG Err", "P actual", "P pred"
    );
    println!(
        "  {:─>7}  {:─>8}  {:─>10}  {:─>10}  {:─>8}  {:─>10}  {:─>10}",
        "", "", "", "", "", "", ""
    );

    let mut blind_cg_err = 0.0;
    let mut blind_plaq_err = 0.0;
    let mut n_test = 0;

    for (i, s) in test_summaries.iter().enumerate() {
        let pred = shell.predict(&test_inputs[i]);
        let pred_cg = pred[0] * max_cg;
        let pred_plaq = pred[1];

        let cg_err = if s.mean_cg > 0.0 {
            (pred_cg - s.mean_cg).abs() / s.mean_cg
        } else {
            0.0
        };
        let plaq_err = (pred_plaq - s.mean_plaq).abs();

        if s.mean_cg > 0.0 {
            blind_cg_err += cg_err;
            n_test += 1;
        }
        blind_plaq_err += plaq_err;

        let marker = if cg_err > 0.15 { " !" } else { "" };
        println!(
            "  {:>7.4}  {:>8}  {:>10.0}  {:>10.0}  {:>7.1}%  {:>10.4}  {:>10.4}{}",
            s.beta, s.n_trajs, s.mean_cg, pred_cg,
            cg_err * 100.0, s.mean_plaq, pred_plaq, marker
        );
    }

    if n_test > 0 {
        println!(
            "\n  Mean BLIND CG relative error: {:.1}%",
            blind_cg_err / n_test as f64 * 100.0
        );
    }
    println!(
        "  Mean BLIND plaquette absolute error: {:.4}",
        blind_plaq_err / test_summaries.len() as f64
    );

    // ─── Overlap analysis ───
    println!("\n━━━ Overlap Analysis ━━━\n");
    let train_betas: std::collections::HashSet<String> = train_summaries
        .iter()
        .map(|s| format!("{:.4}", s.beta))
        .collect();
    let test_betas: std::collections::HashSet<String> = test_summaries
        .iter()
        .map(|s| format!("{:.4}", s.beta))
        .collect();
    let overlap: Vec<&String> = train_betas.intersection(&test_betas).collect();
    let novel: Vec<&String> = test_betas.difference(&train_betas).collect();

    println!("  Training β points: {}", train_betas.len());
    println!("  Test β points: {}", test_betas.len());
    println!("  Overlapping: {} {:?}", overlap.len(), overlap);
    println!("  Novel (true extrapolation): {} {:?}", novel.len(), novel);

    // ─── Shell state ───
    let serialized = serde_json::to_string(&shell).unwrap();
    println!("\n━━━ Shell State ━━━\n");
    println!("  Generations: {}", shell.generation());
    println!("  Size: {:.1} KB", serialized.len() as f64 / 1024.0);
    println!("  Population: {} boards", shell.current_population.size());
    println!(
        "  Best fitness: {:.4}",
        shell
            .history
            .iter()
            .map(|r| r.best_fitness)
            .fold(0.0_f64, f64::max)
    );
    println!("\n  The shell was trained ONLY on Exp 024+028.");
    println!("  Exp 029 predictions are completely blind.\n");
}

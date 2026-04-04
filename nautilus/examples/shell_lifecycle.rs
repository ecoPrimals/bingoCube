// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nautilus Shell Lifecycle Demo
//!
//! Demonstrates:
//! 1. Within-instance evolution (homelab trains on physics data)
//! 2. Serialization (ship shell to another machine)
//! 3. Between-instance continuation (field node continues evolving)
//! 4. Shell merge (homelab absorbs field knowledge)
//!
//! Run: cargo run --example shell_lifecycle -p bingocube-nautilus

use bingocube_nautilus::{
    EvolutionConfig, InstanceId, NautilusShell, ReservoirInput, SelectionMethod, ShellConfig,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║        Nautilus Shell — Evolutionary Reservoir Demo         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ─── Phase 1: Homelab trains on synthetic physics data ───

    println!("━━━ Phase 1: Within-Instance Evolution (Homelab) ━━━\n");

    let config = ShellConfig {
        population_size: 16,
        n_targets: 2, // predict two observables
        evolution: EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 4 },
            mutation_rate: 0.15,
            column_crossover: true,
            cell_crossover: false,
        },
        ridge_lambda: 1e-4,
        ..Default::default()
    };

    let homelab_id = InstanceId::new("homelab-northgate");
    let mut homelab_shell = NautilusShell::from_seed(config.clone(), homelab_id, 42);

    println!("  Instance: {}", homelab_shell.origin.0);
    println!(
        "  Population: {} boards",
        homelab_shell.current_population.size()
    );
    println!(
        "  Response dim: {} ({}×{} cells × {} boards)",
        homelab_shell.current_population.response_dim(),
        homelab_shell.config.board_config.grid_size,
        homelab_shell.config.board_config.grid_size,
        homelab_shell.config.population_size,
    );
    println!();

    // Synthetic "physics" data: predict sin(β) and β² from features
    let (inputs, targets) = generate_physics_data(100);

    println!(
        "  Training on {} samples, 2 target observables",
        inputs.len()
    );
    println!("  Evolving for 20 generations...\n");

    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}",
        "Gen", "MSE", "Mean Fit", "Best Fit"
    );
    println!("  {:─>4}  {:─>10}  {:─>10}  {:─>10}", "", "", "", "");

    for gen_idx in 0..20 {
        let mse = homelab_shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        let traj = homelab_shell.fitness_trajectory();
        let last = traj.last().unwrap();
        if gen_idx % 2 == 0 || gen_idx == 19 {
            println!(
                "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}",
                last.0, mse, last.1, last.2
            );
        }
    }

    // Test prediction
    let test_input = ReservoirInput::Continuous(vec![0.5, 0.5_f64.sin(), 0.5_f64.cos()]);
    let pred = homelab_shell.predict(&test_input);
    let expected = [0.5_f64.sin(), 0.25]; // sin(0.5), 0.5²
    println!("\n  Prediction at β=0.5:");
    println!(
        "    Target 0 (sin β): predicted={:.4}, actual={:.4}",
        pred[0], expected[0]
    );
    println!(
        "    Target 1 (β²):    predicted={:.4}, actual={:.4}",
        pred[1], expected[1]
    );

    // ─── Phase 2: Serialize and ship to field node ───

    println!("\n━━━ Phase 2: Instance Transfer (Homelab → Field Node) ━━━\n");

    let serialized = serde_json::to_string(&homelab_shell).unwrap();
    let shell_size = serialized.len();
    println!(
        "  Serialized shell: {} bytes ({:.1} KB)",
        shell_size,
        shell_size as f64 / 1024.0
    );
    println!(
        "  Contains {} generations of heritage",
        homelab_shell.history.len()
    );

    // Simulate network transfer
    let received: NautilusShell = serde_json::from_str(&serialized).unwrap();
    println!("  ✓ Deserialized on receiving end");

    // ─── Phase 3: Field node continues evolving ───

    println!("\n━━━ Phase 3: Between-Instance Continuation (Field Node) ━━━\n");

    let field_id = InstanceId::new("field-strandgate");
    let mut field_shell = NautilusShell::continue_from(received, field_id);

    println!("  New instance: {}", field_shell.origin.0);
    println!("  Inherited generation: {}", field_shell.generation());
    println!("  Lineage depth: {} instances", field_shell.lineage_depth());
    println!(
        "  Lineage: {:?}",
        field_shell
            .lineage
            .iter()
            .map(|l| l.name())
            .collect::<Vec<_>>()
    );

    // Field node sees slightly different data (different regime)
    let (field_inputs, field_targets) = generate_field_data(80);

    println!("\n  Field node evolving for 10 more generations on local data...\n");
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}",
        "Gen", "MSE", "Mean Fit", "Best Fit"
    );
    println!("  {:─>4}  {:─>10}  {:─>10}  {:─>10}", "", "", "", "");

    for gen_idx in 0..10 {
        let mse = field_shell.evolve_generation_seeded(&field_inputs, &field_targets, 300 + gen_idx);
        let traj = field_shell.fitness_trajectory();
        let last = traj.last().unwrap();
        if gen_idx % 2 == 0 || gen_idx == 9 {
            println!(
                "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}",
                last.0, mse, last.1, last.2
            );
        }
    }

    // ─── Phase 4: Merge field knowledge back into homelab ───

    println!("\n━━━ Phase 4: Shell Merge (Field → Homelab) ━━━\n");

    // Homelab gets the field shell back
    let field_serialized = serde_json::to_string(&field_shell).unwrap();
    let field_received: NautilusShell = serde_json::from_str(&field_serialized).unwrap();

    println!(
        "  Homelab shell: gen {}, {} boards",
        homelab_shell.generation(),
        homelab_shell.current_population.size()
    );
    println!(
        "  Field shell:   gen {}, {} boards",
        field_received.generation(),
        field_received.current_population.size()
    );

    homelab_shell.merge_shell(&field_received);

    println!("  After merge:");
    println!(
        "    Population: {} boards (best from both)",
        homelab_shell.current_population.size()
    );
    println!("    Lineage depth: {}", homelab_shell.lineage_depth());
    println!("    Total history records: {}", homelab_shell.history.len());
    println!(
        "    Combined lineage: {:?}",
        homelab_shell
            .lineage
            .iter()
            .map(|l| l.name())
            .collect::<Vec<_>>()
    );

    // Continue evolving after merge
    println!("\n  Post-merge evolution (5 generations)...\n");
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>10}",
        "Gen", "MSE", "Mean Fit", "Best Fit"
    );
    println!("  {:─>4}  {:─>10}  {:─>10}  {:─>10}", "", "", "", "");

    for gen_idx in 0..5 {
        let mse = homelab_shell.evolve_generation_seeded(&inputs, &targets, 400 + gen_idx);
        let traj = homelab_shell.fitness_trajectory();
        let last = traj.last().unwrap();
        println!(
            "  {:>4}  {:>10.6}  {:>10.4}  {:>10.4}",
            last.0, mse, last.1, last.2
        );
    }

    // ─── Summary ───

    println!("\n━━━ Summary ━━━\n");
    println!("  Full fitness trajectory:");
    println!(
        "  {:>4}  {:>10}  {:>10}  {:>12}",
        "Gen", "Mean Fit", "Best Fit", "Origin"
    );
    println!("  {:─>4}  {:─>10}  {:─>10}  {:─>12}", "", "", "", "");
    for r in &homelab_shell.history {
        println!(
            "  {:>4}  {:>10.4}  {:>10.4}  {:>12}",
            r.generation,
            r.mean_fitness,
            r.best_fitness,
            r.origin_instance.name(),
        );
    }

    println!(
        "\n  The nautilus shell has {} layers of heritage",
        homelab_shell.history.len()
    );
    println!("  spanning {} instances.", homelab_shell.lineage_depth());
    println!("  Each layer wraps the previous, preserving heritage while adding adaptation.\n");
}

/// Generate synthetic "physics" data: predict sin(β) and β² from (β, sin(β), cos(β)).
fn generate_physics_data(n: usize) -> (Vec<ReservoirInput>, Vec<Vec<f64>>) {
    let inputs: Vec<ReservoirInput> = (0..n)
        .map(|i| {
            let beta = i as f64 / n as f64;
            ReservoirInput::Continuous(vec![beta, beta.sin(), beta.cos()])
        })
        .collect();

    let targets: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let beta = i as f64 / n as f64;
            vec![beta.sin(), beta * beta]
        })
        .collect();

    (inputs, targets)
}

/// Generate field data with a shifted regime.
fn generate_field_data(n: usize) -> (Vec<ReservoirInput>, Vec<Vec<f64>>) {
    let inputs: Vec<ReservoirInput> = (0..n)
        .map(|i| {
            let beta = 0.5 + i as f64 / (2.0 * n as f64); // β ∈ [0.5, 1.0]
            ReservoirInput::Continuous(vec![beta, beta.sin(), beta.cos()])
        })
        .collect();

    let targets: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let beta = 0.5 + i as f64 / (2.0 * n as f64);
            vec![beta.sin(), beta * beta]
        })
        .collect();

    (inputs, targets)
}

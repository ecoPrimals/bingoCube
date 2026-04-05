// SPDX-License-Identifier: AGPL-3.0-or-later
//! One generation of shell evolution: project, train readout, fitness, drift, edge seed, breed.

use rand::Rng;

use crate::constraints::{DriftAction, EdgeSeeder};
use crate::evolution::{Evolution, SelectionMethod};
use crate::response::{ReservoirInput, ResponseVector};
use crate::shell::NautilusShell;
use crate::snapshot::GenerationRecord;

/// Run one full evolution cycle using the given RNG.
///
/// Returns the MSE of the readout after training.
pub fn evolve_one_generation<R: Rng>(
    shell: &mut NautilusShell,
    inputs: &[ReservoirInput],
    targets: &[Vec<f64>],
    rng: &mut R,
) -> f64 {
    assert_eq!(inputs.len(), targets.len());

    // 1. Project all inputs through current population
    let responses: Vec<ResponseVector> = inputs
        .iter()
        .map(|inp| shell.current_population.project(inp))
        .collect();

    // 2. Train readout
    shell.readout.train(&responses, targets);
    let mse = shell.readout.mse(&responses, targets);

    // 3. Evaluate board fitness
    shell.current_population.evaluate_fitness(inputs, targets);

    // 4. Drift monitor: record and apply
    let mean_fit = shell.current_population.mean_fitness();
    let best_fit = shell.current_population.best_fitness();
    let pop_size = shell.current_population.size();
    shell.drift_monitor.record(
        shell.current_population.generation,
        pop_size,
        mean_fit,
        best_fit,
    );
    let drift_action = shell.drift_monitor.recommendation();
    let mut evo_config = shell.config.evolution.clone();

    match &drift_action {
        DriftAction::IncreaseSelection => match &mut evo_config.selection {
            SelectionMethod::Elitism { survivors } => {
                *survivors = (*survivors / 2).max(1);
            }
            SelectionMethod::Tournament { tournament_size } => {
                *tournament_size = (*tournament_size + 2).min(pop_size);
            }
            SelectionMethod::Roulette => {}
        },
        DriftAction::IncreasePop { factor } => {
            let new_size = (pop_size as f64 * factor).ceil() as usize;
            let deficit = new_size.saturating_sub(pop_size);
            if deficit > 0 {
                for _ in 0..deficit {
                    let board = bingocube_core::Board::generate(&shell.config.board_config, rng)
                        .expect("valid config");
                    shell.current_population.boards.push(board);
                }
            }
        }
        DriftAction::Continue => {}
    }

    // 5. Edge seeding: replace worst boards with edge-biased boards
    if !shell.concept_edges.is_empty() {
        let n_edge = (pop_size / 4).max(1).min(shell.concept_edges.len());
        let ranked = shell.current_population.ranked_boards();
        let worst_indices: Vec<usize> = ranked.iter().rev().take(n_edge).map(|&(i, _)| i).collect();

        for (slot, edge_features) in worst_indices.iter().zip(shell.concept_edges.iter()) {
            if let Ok(mut seeded) =
                EdgeSeeder::seed_boards(&shell.config.board_config, 1, edge_features, rng)
            {
                if let Some(board) = seeded.pop() {
                    shell.current_population.boards[*slot] = board;
                }
            }
        }
    }

    let ne_s = shell.drift_monitor.latest_ne_s();

    // 6. Record generation
    shell.history.push(GenerationRecord {
        generation: shell.current_population.generation,
        mean_fitness: mean_fit,
        best_fitness: best_fit,
        population_size: shell.current_population.size(),
        origin_instance: shell.origin.clone(),
        n_training_samples: inputs.len(),
        ne_s,
        drift_action: drift_action.clone(),
    });

    // 7. Breed next generation
    let next = Evolution::next_generation(&shell.current_population, &evo_config, rng)
        .expect("evolution should succeed");
    shell.current_population = next;

    mse
}

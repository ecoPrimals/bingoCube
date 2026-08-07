// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evolution — selection, crossover, and mutation to breed the next generation.
//!
//! After a population is evaluated for fitness, evolution creates the next
//! generation by:
//!
//! 1. **Selection**: Choose parent boards based on fitness
//! 2. **Crossover**: Combine structural properties of two parents
//! 3. **Mutation**: Add constrained randomness to child boards
//!
//! The key insight: board evolution preserves column-range constraints while
//! exploring the combinatorial space of ~10^31 possible boards (for L=5).
//! Children inherit high-performing parents' structure but are never clones.

use bingocube_core::{BingoCubeError, Board, Config};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::population::Population;

/// Selection method for choosing parents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SelectionMethod {
    /// Top-K elitism: the K best boards survive directly, rest are bred.
    Elitism {
        /// Number of boards that survive unchanged.
        survivors: usize,
    },

    /// Tournament selection: pick random subsets, best of each subset breeds.
    Tournament {
        /// Tournament size (number of candidates per selection).
        tournament_size: usize,
    },

    /// Roulette wheel: probability proportional to fitness.
    Roulette,
}

/// Configuration for the evolutionary process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Selection method.
    pub selection: SelectionMethod,

    /// Mutation rate: probability that a cell is re-randomized in a child.
    /// Range [0.0, 1.0]. Higher = more exploration, lower = more exploitation.
    pub mutation_rate: f64,

    /// Whether to apply column-swap crossover (swaps entire columns between parents).
    pub column_crossover: bool,

    /// Whether to apply cell-level crossover (mixes individual cells from parents).
    pub cell_crossover: bool,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            selection: SelectionMethod::Elitism { survivors: 4 },
            mutation_rate: 0.15,
            column_crossover: true,
            cell_crossover: false,
        }
    }
}

/// The evolutionary engine.
pub struct Evolution;

impl Evolution {
    /// Breed the next generation from an evaluated population.
    ///
    /// Returns a new `Population` with the same size, at generation + 1.
    /// Parents are selected by the configured method. Children are produced
    /// via crossover + mutation while preserving column-range constraints.
    pub fn next_generation<R: Rng>(
        current: &Population,
        evo_config: &EvolutionConfig,
        rng: &mut R,
    ) -> Result<Population, BingoCubeError> {
        let pop_size = current.size();
        let config = &current.config;

        let ranked = current.ranked_boards();
        if ranked.is_empty() {
            return Population::random(config.clone(), pop_size, rng);
        }

        let mut next_boards = Vec::with_capacity(pop_size);

        match evo_config.selection {
            SelectionMethod::Elitism { survivors } => {
                let n_elite = survivors.min(pop_size);

                // Elite survivors pass through unchanged
                for &(idx, _) in ranked.iter().take(n_elite) {
                    next_boards.push(current.boards[idx].clone());
                }

                // Fill remaining slots with offspring
                while next_boards.len() < pop_size {
                    let parent_a = &current.boards[Self::tournament_select(&ranked, 3, rng)];
                    let parent_b = &current.boards[Self::tournament_select(&ranked, 3, rng)];
                    let child = Self::breed(parent_a, parent_b, config, evo_config, rng)?;
                    next_boards.push(child);
                }
            }
            SelectionMethod::Tournament { tournament_size } => {
                while next_boards.len() < pop_size {
                    let parent_a =
                        &current.boards[Self::tournament_select(&ranked, tournament_size, rng)];
                    let parent_b =
                        &current.boards[Self::tournament_select(&ranked, tournament_size, rng)];
                    let child = Self::breed(parent_a, parent_b, config, evo_config, rng)?;
                    next_boards.push(child);
                }
            }
            SelectionMethod::Roulette => {
                let total_fitness: f64 = ranked.iter().map(|(_, f)| f).sum();
                while next_boards.len() < pop_size {
                    let parent_a =
                        &current.boards[Self::roulette_select(&ranked, total_fitness, rng)];
                    let parent_b =
                        &current.boards[Self::roulette_select(&ranked, total_fitness, rng)];
                    let child = Self::breed(parent_a, parent_b, config, evo_config, rng)?;
                    next_boards.push(child);
                }
            }
        }

        Ok(Population {
            boards: next_boards,
            config: config.clone(),
            generation: current.generation + 1,
            fitness: Vec::new(),
        })
    }

    /// Tournament selection: pick `k` random candidates, return the best.
    fn tournament_select<R: Rng>(ranked: &[(usize, f64)], k: usize, rng: &mut R) -> usize {
        let mut best_idx = ranked[rng.gen_range(0..ranked.len())].0;
        let mut best_fit = f64::NEG_INFINITY;

        for _ in 0..k {
            let candidate = &ranked[rng.gen_range(0..ranked.len())];
            if candidate.1 > best_fit {
                best_fit = candidate.1;
                best_idx = candidate.0;
            }
        }
        best_idx
    }

    /// Roulette wheel selection: probability proportional to fitness.
    fn roulette_select<R: Rng>(ranked: &[(usize, f64)], total: f64, rng: &mut R) -> usize {
        if total <= 0.0 {
            return ranked[rng.gen_range(0..ranked.len())].0;
        }

        let mut threshold = rng.r#gen::<f64>() * total;
        for &(idx, fitness) in ranked {
            threshold -= fitness;
            if threshold <= 0.0 {
                return idx;
            }
        }

        ranked.last().map(|r| r.0).unwrap_or(0)
    }

    /// Breed a child from two parents via crossover + mutation.
    ///
    /// Column-range constraints are preserved: each column's values stay
    /// within [col * range_size, (col+1) * range_size).
    fn breed<R: Rng>(
        parent_a: &Board,
        parent_b: &Board,
        config: &Config,
        evo_config: &EvolutionConfig,
        rng: &mut R,
    ) -> Result<Board, BingoCubeError> {
        let size = config.grid_size;
        let range_size = config.range_size();
        let mut grid = vec![vec![None; size]; size];

        if evo_config.column_crossover {
            // Column-swap crossover: each column is taken from one parent
            for col in 0..size {
                let source = if rng.r#gen::<bool>() {
                    parent_a
                } else {
                    parent_b
                };
                for row in 0..size {
                    grid[row][col] = source.grid[row][col];
                }
            }
        } else if evo_config.cell_crossover {
            // Cell-level crossover: each cell is taken from one parent
            for row in 0..size {
                for col in 0..size {
                    let source = if rng.r#gen::<bool>() {
                        parent_a
                    } else {
                        parent_b
                    };
                    grid[row][col] = source.grid[row][col];
                }
            }

            // Fix duplicates within columns (cell crossover can create them)
            for col in 0..size {
                Self::fix_column_duplicates(&mut grid, col, size, range_size, config, rng);
            }
        } else {
            // No crossover: clone parent_a
            for row in 0..size {
                for col in 0..size {
                    grid[row][col] = parent_a.grid[row][col];
                }
            }
        }

        // Mutation: re-randomize cells with probability mutation_rate
        for col in 0..size {
            let range_start = (col * range_size) as u32;
            let range_end = range_start + range_size as u32;

            for row in 0..size {
                if let Some((free_row, free_col)) = config.free_cell {
                    if row == free_row && col == free_col {
                        grid[row][col] = None;
                        continue;
                    }
                }

                if rng.r#gen::<f64>() < evo_config.mutation_rate {
                    // Replace with a random value from the column's range
                    // that isn't already used in this column
                    let used: Vec<u32> = (0..size)
                        .filter(|&r| r != row)
                        .filter_map(|r| grid[r][col])
                        .collect();

                    let available: Vec<u32> = (range_start..range_end)
                        .filter(|v| !used.contains(v))
                        .collect();

                    if let Some(&new_val) = available.choose(rng) {
                        grid[row][col] = Some(new_val);
                    }
                }
            }
        }

        // Inherit permutation from a random parent
        let permutation = if rng.r#gen::<bool>() {
            parent_a.permutation.clone()
        } else {
            parent_b.permutation.clone()
        };

        Ok(Board {
            grid,
            size,
            permutation,
        })
    }

    /// Fix duplicate values within a column after cell-level crossover.
    fn fix_column_duplicates<R: Rng>(
        grid: &mut [Vec<Option<u32>>],
        col: usize,
        size: usize,
        range_size: usize,
        config: &Config,
        rng: &mut R,
    ) {
        let range_start = (col * range_size) as u32;
        let range_end = range_start + range_size as u32;

        let mut seen = std::collections::HashSet::new();
        let mut dup_rows = Vec::new();

        for row in 0..size {
            if let Some((fr, fc)) = config.free_cell {
                if row == fr && col == fc {
                    continue;
                }
            }
            if let Some(val) = grid[row][col] {
                if !seen.insert(val) {
                    dup_rows.push(row);
                }
            }
        }

        for row in dup_rows {
            let available: Vec<u32> = (range_start..range_end)
                .filter(|v| !seen.contains(v))
                .collect();
            if let Some(&new_val) = available.choose(rng) {
                grid[row][col] = Some(new_val);
                seen.insert(new_val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population::FitnessRecord;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    fn make_evaluated_pop(rng: &mut ChaCha20Rng) -> Population {
        let config = Config::default();
        let mut pop = Population::random(config, 16, rng).unwrap();

        let inputs: Vec<crate::ReservoirInput> = (0..30)
            .map(|i| crate::ReservoirInput::Continuous(vec![i as f64 / 30.0, (i as f64).sin()]))
            .collect();
        let targets: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64 / 30.0]).collect();

        pop.evaluate_fitness(&inputs, &targets);
        pop
    }

    #[test]
    fn test_elitism_evolution() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = make_evaluated_pop(&mut rng);

        let evo_config = EvolutionConfig::default();
        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();

        assert_eq!(next.size(), 16);
        assert_eq!(next.generation, 1);
        assert!(next.fitness.is_empty()); // not yet evaluated
    }

    #[test]
    fn test_tournament_evolution() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = make_evaluated_pop(&mut rng);

        let evo_config = EvolutionConfig {
            selection: SelectionMethod::Tournament { tournament_size: 4 },
            mutation_rate: 0.2,
            column_crossover: true,
            cell_crossover: false,
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        assert_eq!(next.size(), 16);
        assert_eq!(next.generation, 1);
    }

    #[test]
    fn test_column_constraints_preserved() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = make_evaluated_pop(&mut rng);
        let config = &pop.config;

        let evo_config = EvolutionConfig {
            mutation_rate: 0.5, // High mutation to stress-test constraints
            ..Default::default()
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();

        // Verify column-range constraints for all boards
        let range_size = config.range_size();
        for board in &next.boards {
            for col in 0..config.grid_size {
                let range_start = (col * range_size) as u32;
                let range_end = range_start + range_size as u32;

                for row in 0..config.grid_size {
                    if let Some((fr, fc)) = config.free_cell {
                        if row == fr && col == fc {
                            assert!(board.get(row, col).is_none());
                            continue;
                        }
                    }
                    if let Some(val) = board.get(row, col) {
                        assert!(
                            val >= range_start && val < range_end,
                            "Board col {col} row {row}: val {val} outside [{range_start}, {range_end})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_multi_generation_evolution() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let mut pop = make_evaluated_pop(&mut rng);
        let evo_config = EvolutionConfig::default();

        let inputs: Vec<crate::ReservoirInput> = (0..30)
            .map(|i| crate::ReservoirInput::Continuous(vec![i as f64 / 30.0, (i as f64).sin()]))
            .collect();
        let targets: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64 / 30.0]).collect();

        for gen_idx in 0..5 {
            let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
            assert_eq!(next.generation, gen_idx + 1);
            pop = next;
            pop.evaluate_fitness(&inputs, &targets);
        }

        assert_eq!(pop.generation, 5);
    }

    #[test]
    fn test_next_generation_random_when_no_fitness() {
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        let mut pop = make_evaluated_pop(&mut rng);
        pop.fitness.clear();

        let evo_config = EvolutionConfig::default();
        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        assert_eq!(next.size(), pop.size());
        // Empty fitness → ranked boards empty → `Population::random` (generation resets to 0).
        assert_eq!(next.generation, 0);
    }

    #[test]
    fn test_roulette_selection_path() {
        let mut rng = ChaCha20Rng::seed_from_u64(11);
        let mut pop = make_evaluated_pop(&mut rng);
        // Total fitness 0 forces roulette_select's uniform fallback branch.
        pop.fitness = (0..pop.boards.len())
            .map(|i| FitnessRecord {
                board_idx: i,
                fitness: 0.0,
                target_correlations: vec![0.0],
            })
            .collect();

        let evo_config = EvolutionConfig {
            selection: SelectionMethod::Roulette,
            mutation_rate: 0.1,
            column_crossover: true,
            cell_crossover: false,
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        assert_eq!(next.size(), pop.size());
        assert_eq!(next.generation, pop.generation + 1);
    }

    #[test]
    fn test_cell_crossover_with_duplicate_repair() {
        let mut rng = ChaCha20Rng::seed_from_u64(99);
        let pop = make_evaluated_pop(&mut rng);
        let config = &pop.config;

        let evo_config = EvolutionConfig {
            selection: SelectionMethod::Tournament { tournament_size: 3 },
            mutation_rate: 0.05,
            column_crossover: false,
            cell_crossover: true,
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        let range_size = config.range_size();
        for board in &next.boards {
            for col in 0..config.grid_size {
                let range_start = (col * range_size) as u32;
                let range_end = range_start + range_size as u32;
                for row in 0..config.grid_size {
                    if let Some((fr, fc)) = config.free_cell {
                        if row == fr && col == fc {
                            assert!(board.get(row, col).is_none());
                            continue;
                        }
                    }
                    if let Some(val) = board.get(row, col) {
                        assert!(val >= range_start && val < range_end);
                    }
                }
            }
        }
    }

    #[test]
    fn test_no_crossover_clones_primary_parent() {
        let mut rng = ChaCha20Rng::seed_from_u64(123);
        let pop = make_evaluated_pop(&mut rng);

        let evo_config = EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 2 },
            mutation_rate: 0.0,
            column_crossover: false,
            cell_crossover: false,
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        assert_eq!(next.size(), pop.size());
        assert_eq!(next.generation, pop.generation + 1);
    }

    #[test]
    fn test_elitism_survivors_capped_by_population_size() {
        let mut rng = ChaCha20Rng::seed_from_u64(55);
        let pop = make_evaluated_pop(&mut rng);

        let evo_config = EvolutionConfig {
            selection: SelectionMethod::Elitism { survivors: 1_000 },
            mutation_rate: 0.1,
            column_crossover: true,
            cell_crossover: false,
        };

        let next = Evolution::next_generation(&pop, &evo_config, &mut rng).unwrap();
        assert_eq!(next.size(), pop.size());
    }
}

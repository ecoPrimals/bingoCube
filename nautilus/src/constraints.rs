//! Constraint variants for board generation and evolution.
//!
//! Column-range constraints are the baseline "type system" for boards.
//! Additional constraints further prune the viable board space, smoothing
//! the NK fitness landscape and accelerating specialization.
//!
//! Each constraint reduces effective K (epistatic interactions between cells),
//! making the landscape less rugged and easier for evolution to navigate.

use bingocube_core::{Board, Config, BingoCubeError};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Constraint level applied to board generation and evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintLevel {
    /// Column-range only (baseline): column k draws from [k*R, (k+1)*R).
    /// Effective K ≈ L - 1 (cells interact only within their column).
    ColumnRange,

    /// Column-range + row uniqueness (sudoku-like).
    /// No value appears twice in any row across different column ranges.
    /// Effective K ≈ 2L - 2 (column + row, still sub-quadratic).
    Sudoku,
}

/// N_e * s drift monitor for evolutionary populations.
/// Tracks whether selection is dominating drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMonitor {
    /// History of (generation, N_e_s_ratio) measurements.
    pub history: Vec<(usize, f64)>,

    /// Threshold below which drift dominates.
    pub drift_threshold: f64,

    /// Number of consecutive generations below threshold.
    pub consecutive_drift: usize,
}

impl Default for DriftMonitor {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            drift_threshold: 1.0,
            consecutive_drift: 0,
        }
    }
}

impl DriftMonitor {
    /// Record a generation's fitness statistics and compute N_e * s.
    ///
    /// N_e = effective population size (we approximate as pop_size)
    /// s = selection coefficient ≈ (best_fitness - mean_fitness) / mean_fitness
    pub fn record(&mut self, generation: usize, pop_size: usize, mean_fitness: f64, best_fitness: f64) {
        let s = if mean_fitness > 1e-10 {
            (best_fitness - mean_fitness) / mean_fitness
        } else {
            0.0
        };

        let ne_s = pop_size as f64 * s;
        self.history.push((generation, ne_s));

        if ne_s < self.drift_threshold {
            self.consecutive_drift += 1;
        } else {
            self.consecutive_drift = 0;
        }
    }

    /// Whether the population is currently drifting (N_e * s < threshold).
    pub fn is_drifting(&self) -> bool {
        self.consecutive_drift >= 3
    }

    /// Recommended action when drifting.
    pub fn recommendation(&self) -> DriftAction {
        if !self.is_drifting() {
            return DriftAction::Continue;
        }

        if self.consecutive_drift >= 10 {
            DriftAction::IncreasePop { factor: 2.0 }
        } else {
            DriftAction::IncreaseSelection
        }
    }

    /// Latest N_e * s ratio.
    pub fn latest_ne_s(&self) -> f64 {
        self.history.last().map(|h| h.1).unwrap_or(0.0)
    }
}

/// Action recommended by the drift monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriftAction {
    /// Selection is working, continue normally.
    Continue,
    /// Increase selection pressure (more elites, larger tournaments).
    IncreaseSelection,
    /// Increase population size by the given factor.
    IncreasePop {
        /// Multiplication factor for population size.
        factor: f64,
    },
}

/// Concept-edge board seeder.
///
/// When the shell identifies a region of parameter space where predictions
/// fail (concept edge), this generates boards with column values concentrated
/// in the relevant ranges — directed mutagenesis to potentiate the population
/// for learning at discontinuities.
pub struct EdgeSeeder;

impl EdgeSeeder {
    /// Generate boards targeted at a concept edge.
    ///
    /// `edge_beta` is the normalized parameter value (0..1) where the
    /// prediction breaks down. Boards are generated with column values
    /// biased toward the edge region.
    pub fn seed_boards<R: Rng>(
        config: &Config,
        n_boards: usize,
        edge_features: &[f64],
        rng: &mut R,
    ) -> Result<Vec<Board>, BingoCubeError> {
        config.validate()?;

        let size = config.grid_size;
        let range_size = config.range_size();
        let mut boards = Vec::with_capacity(n_boards);

        for _ in 0..n_boards {
            let mut grid = vec![vec![None; size]; size];

            for col in 0..size {
                let range_start = col * range_size;
                let range_end = range_start + range_size;

                // Bias value selection toward a region informed by edge features.
                // Use the edge features to pick a "center of mass" within
                // the column's range, then sample preferentially nearby.
                let feature_idx = col % edge_features.len();
                let feature_val = edge_features[feature_idx].clamp(0.0, 1.0);
                let center = range_start + (feature_val * range_size as f64) as usize;
                let center = center.min(range_end - 1);

                // Generate values with Gaussian-like bias around center
                let mut values: Vec<u32> = (range_start..range_end).map(|v| v as u32).collect();
                values.sort_by(|a, b| {
                    let da = (*a as i64 - center as i64).unsigned_abs();
                    let db = (*b as i64 - center as i64).unsigned_abs();
                    da.cmp(&db)
                });

                // Take the closest values, then shuffle among them for variety
                let n_cells = if config.free_cell.map_or(false, |(_, fc)| fc == col) {
                    size - 1
                } else {
                    size
                };
                let pool_size = (n_cells * 2).min(range_size);
                let pool = &mut values[..pool_size];
                pool.shuffle(rng);

                let mut value_idx = 0;
                for row in 0..size {
                    if let Some((free_row, free_col)) = config.free_cell {
                        if row == free_row && col == free_col {
                            grid[row][col] = None;
                            continue;
                        }
                    }
                    if value_idx < pool.len() {
                        grid[row][col] = Some(pool[value_idx]);
                        value_idx += 1;
                    }
                }
            }

            let mut permutation: Vec<usize> = (0..size).collect();
            permutation.shuffle(rng);

            boards.push(Board {
                grid,
                size,
                permutation,
            });
        }

        Ok(boards)
    }
}

/// Check if a board satisfies a given constraint level.
pub fn board_satisfies(board: &Board, config: &Config, level: ConstraintLevel) -> bool {
    let size = config.grid_size;
    let range_size = config.range_size();

    // Column-range check (always required)
    for col in 0..size {
        let range_start = (col * range_size) as u32;
        let range_end = range_start + range_size as u32;
        let mut seen = std::collections::HashSet::new();

        for row in 0..size {
            if let Some((fr, fc)) = config.free_cell {
                if row == fr && col == fc {
                    continue;
                }
            }
            if let Some(val) = board.get(row, col) {
                if val < range_start || val >= range_end {
                    return false;
                }
                if !seen.insert(val) {
                    return false; // duplicate in column
                }
            }
        }
    }

    if level == ConstraintLevel::Sudoku {
        // Row uniqueness: no two cells in the same row should map to
        // the same value modulo range_size (position within their column's range).
        for row in 0..size {
            let mut positions = std::collections::HashSet::new();
            for col in 0..size {
                if let Some((fr, fc)) = config.free_cell {
                    if row == fr && col == fc {
                        continue;
                    }
                }
                if let Some(val) = board.get(row, col) {
                    let pos_in_range = val as usize % range_size;
                    if !positions.insert(pos_in_range) {
                        return false;
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_drift_monitor_detects_drift() {
        let mut monitor = DriftMonitor::default();

        // Strong selection
        for gen in 0..5 {
            monitor.record(gen, 24, 0.5, 0.8);
        }
        assert!(!monitor.is_drifting());
        assert!(monitor.latest_ne_s() > 1.0);

        // Weak selection (best ≈ mean)
        for gen in 5..15 {
            monitor.record(gen, 24, 0.5, 0.502);
        }
        assert!(monitor.is_drifting());
        assert_eq!(monitor.recommendation(), DriftAction::IncreasePop { factor: 2.0 });
    }

    #[test]
    fn test_edge_seeder_generates_valid_boards() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let edge_features = vec![0.85, 0.3, 0.5]; // concept edge at high-β
        let boards = EdgeSeeder::seed_boards(&config, 8, &edge_features, &mut rng).unwrap();

        assert_eq!(boards.len(), 8);
        for board in &boards {
            assert!(board_satisfies(board, &config, ConstraintLevel::ColumnRange));
        }
    }

    #[test]
    fn test_column_range_constraint_check() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let board = Board::generate(&config, &mut rng).unwrap();

        assert!(board_satisfies(&board, &config, ConstraintLevel::ColumnRange));
    }
}

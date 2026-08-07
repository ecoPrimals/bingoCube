// SPDX-License-Identifier: AGPL-3.0-or-later
//! Board response — how input data flows through a board as a reservoir projection.
//!
//! A bingo board is a structured random network. When input values stream through
//! as the "caller," each cell either matches or doesn't. The binary match pattern
//! IS the reservoir response — a deterministic random projection of the input into
//! a high-dimensional space.
//!
//! For continuous-valued inputs (like plaquette measurements), we use scalar field
//! projections: each input value is hashed with the board cell to produce a
//! continuous activation, giving richer responses than binary matching.

use bingocube_core::{Board, Config};
use serde::{Deserialize, Serialize};

/// Input to the reservoir. Can be discrete (bingo caller values) or continuous
/// (physics observables).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReservoirInput {
    /// Discrete caller values — classic bingo. Each value either matches a cell or not.
    Discrete(Vec<u32>),

    /// Continuous feature vector — physics observables. Each feature is projected
    /// through the board's scalar field.
    Continuous(Vec<f64>),
}

/// The response vector from a single board. This is the "reservoir state" for
/// one board in the ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseVector {
    /// The activation values, one per board cell (L×L flattened row-major).
    pub activations: Vec<f64>,
}

impl ResponseVector {
    /// Dimensionality of the response.
    pub fn dim(&self) -> usize {
        self.activations.len()
    }
}

/// Computes the response of a board to an input stream.
pub struct BoardResponse;

impl BoardResponse {
    /// Project input through a board, producing a response vector.
    ///
    /// For **discrete** input: binary match pattern. Each cell activates (1.0) if
    /// any input value matches the cell's value, else 0.0.
    ///
    /// For **continuous** input: scalar projection. Each cell's value is combined
    /// with the input features via BLAKE3 to produce a bounded activation in [0, 1].
    /// This gives each board a unique, deterministic, nonlinear projection of the
    /// continuous input — exactly what reservoir computing needs.
    pub fn project(board: &Board, config: &Config, input: &ReservoirInput) -> ResponseVector {
        let size = config.grid_size;
        let mut activations = Vec::with_capacity(size * size);

        match input {
            ReservoirInput::Discrete(values) => {
                for row in 0..size {
                    for col in 0..size {
                        let act = match board.get(row, col) {
                            Some(cell_val) => {
                                if values.contains(&cell_val) {
                                    1.0
                                } else {
                                    0.0
                                }
                            }
                            None => 0.5, // free cell: always half-active
                        };
                        activations.push(act);
                    }
                }
            }
            ReservoirInput::Continuous(features) => {
                for row in 0..size {
                    for col in 0..size {
                        let cell_val = board.get(row, col).unwrap_or(0);
                        let act = Self::scalar_projection(row, col, cell_val, features);
                        activations.push(act);
                    }
                }
            }
        }

        ResponseVector { activations }
    }

    /// Project an ensemble of boards, concatenating their response vectors.
    /// The result is a single fat vector: [board₀ response | board₁ response | ...].
    pub fn project_ensemble(
        boards: &[Board],
        config: &Config,
        input: &ReservoirInput,
    ) -> ResponseVector {
        let mut all_activations = Vec::new();
        for board in boards {
            let resp = Self::project(board, config, input);
            all_activations.extend(resp.activations);
        }
        ResponseVector {
            activations: all_activations,
        }
    }

    /// Scalar projection of continuous features through a single cell.
    ///
    /// Uses BLAKE3 to hash (row, col, cell_value, feature_bytes) → [0, 1].
    /// This is the core nonlinear random projection that makes each board unique.
    fn scalar_projection(row: usize, col: usize, cell_val: u32, features: &[f64]) -> f64 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"NAUTILUS_PROJ");
        hasher.update(&row.to_le_bytes());
        hasher.update(&col.to_le_bytes());
        hasher.update(&cell_val.to_le_bytes());

        for &f in features {
            hasher.update(&f.to_le_bytes());
        }

        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        let raw = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        // Map to [0, 1]
        raw as f64 / u64::MAX as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_core::Config;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_discrete_response_deterministic() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let board = Board::generate(&config, &mut rng).unwrap();

        let input = ReservoirInput::Discrete(vec![3, 17, 42, 81]);
        let r1 = BoardResponse::project(&board, &config, &input);
        let r2 = BoardResponse::project(&board, &config, &input);

        assert_eq!(r1.activations, r2.activations);
        assert_eq!(r1.dim(), 25); // 5×5
    }

    #[test]
    fn test_continuous_response_varies_with_input() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let board = Board::generate(&config, &mut rng).unwrap();

        let input_a = ReservoirInput::Continuous(vec![0.1, 0.2, 0.3]);
        let input_b = ReservoirInput::Continuous(vec![0.9, 0.8, 0.7]);

        let ra = BoardResponse::project(&board, &config, &input_a);
        let rb = BoardResponse::project(&board, &config, &input_b);

        // Different inputs must produce different responses
        assert_ne!(ra.activations, rb.activations);
    }

    #[test]
    fn test_ensemble_projection_concatenates() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let boards: Vec<Board> = (0..4)
            .map(|_| Board::generate(&config, &mut rng).unwrap())
            .collect();

        let input = ReservoirInput::Continuous(vec![0.5, 0.6]);
        let ensemble = BoardResponse::project_ensemble(&boards, &config, &input);

        assert_eq!(ensemble.dim(), 4 * 25); // 4 boards × 25 cells
    }

    #[test]
    fn test_different_boards_give_different_projections() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let board_a = Board::generate(&config, &mut rng).unwrap();
        let board_b = Board::generate(&config, &mut rng).unwrap();

        let input = ReservoirInput::Continuous(vec![0.5, 0.6, 0.7]);
        let ra = BoardResponse::project(&board_a, &config, &input);
        let rb = BoardResponse::project(&board_b, &config, &input);

        assert_ne!(ra.activations, rb.activations);
    }
}

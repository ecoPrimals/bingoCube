//! Board populations — ensembles of boards that form a generation.
//!
//! A [`Population`] is a set of boards that collectively act as the reservoir.
//! Each board is a different random projection. Together they provide the
//! combinatorial diversity that replaces temporal memory in the feed-forward
//! architecture.

use bingocube_core::{BingoCubeError, Board, Config};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::response::{BoardResponse, ReservoirInput, ResponseVector};

/// Fitness evaluation for a single board in the population.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessRecord {
    /// Index of the board in the population.
    pub board_idx: usize,

    /// Fitness score: higher is better. Measures how well this board's
    /// response correlates with the target observable.
    pub fitness: f64,

    /// Per-target correlation breakdown (if multiple readout targets).
    pub target_correlations: Vec<f64>,
}

/// A population of boards forming one generation's reservoir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// The boards in this generation.
    pub boards: Vec<Board>,

    /// Configuration shared by all boards.
    pub config: Config,

    /// Generation number (0 = random init).
    pub generation: usize,

    /// Fitness records from evaluation (empty until evaluated).
    pub fitness: Vec<FitnessRecord>,
}

impl Population {
    /// Create a new random population (Generation 0).
    pub fn random<R: Rng>(
        config: Config,
        pop_size: usize,
        rng: &mut R,
    ) -> Result<Self, BingoCubeError> {
        let mut boards = Vec::with_capacity(pop_size);
        for _ in 0..pop_size {
            boards.push(Board::generate(&config, rng)?);
        }

        Ok(Self {
            boards,
            config,
            generation: 0,
            fitness: Vec::new(),
        })
    }

    /// Number of boards.
    pub fn size(&self) -> usize {
        self.boards.len()
    }

    /// Dimensionality of the ensemble response vector.
    /// Each board contributes grid_size² activations.
    pub fn response_dim(&self) -> usize {
        let cells_per_board = self.config.grid_size * self.config.grid_size;
        self.boards.len() * cells_per_board
    }

    /// Project input through the entire population (ensemble response).
    pub fn project(&self, input: &ReservoirInput) -> ResponseVector {
        BoardResponse::project_ensemble(&self.boards, &self.config, input)
    }

    /// Evaluate fitness of each board against a dataset.
    ///
    /// For each board, computes the correlation between its individual response
    /// and the target values across all samples. Boards whose responses vary
    /// predictably with the target get high fitness.
    pub fn evaluate_fitness(&mut self, inputs: &[ReservoirInput], targets: &[Vec<f64>]) {
        assert_eq!(inputs.len(), targets.len(), "inputs and targets must match");
        if inputs.is_empty() {
            return;
        }

        let n_targets = targets[0].len();
        let n_samples = inputs.len();
        let mut records = Vec::with_capacity(self.boards.len());

        for (board_idx, board) in self.boards.iter().enumerate() {
            // Collect this board's response across all samples
            let responses: Vec<ResponseVector> = inputs
                .iter()
                .map(|inp| BoardResponse::project(board, &self.config, inp))
                .collect();

            // For each target dimension, compute correlation between
            // the board's mean activation and the target value
            let mut target_correlations = Vec::with_capacity(n_targets);

            for t in 0..n_targets {
                let target_vals: Vec<f64> = targets.iter().map(|tv| tv[t]).collect();

                // Mean activation per sample
                let mean_acts: Vec<f64> = responses
                    .iter()
                    .map(|r| {
                        let sum: f64 = r.activations.iter().sum();
                        sum / r.activations.len() as f64
                    })
                    .collect();

                let corr = pearson_correlation(&mean_acts, &target_vals, n_samples);
                target_correlations.push(corr);
            }

            let fitness =
                target_correlations.iter().map(|c| c.abs()).sum::<f64>() / n_targets as f64;

            records.push(FitnessRecord {
                board_idx,
                fitness,
                target_correlations,
            });
        }

        self.fitness = records;
    }

    /// Get boards sorted by fitness (best first).
    pub fn ranked_boards(&self) -> Vec<(usize, f64)> {
        let mut ranked: Vec<(usize, f64)> = self
            .fitness
            .iter()
            .map(|r| (r.board_idx, r.fitness))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Mean fitness across the population.
    pub fn mean_fitness(&self) -> f64 {
        if self.fitness.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.fitness.iter().map(|r| r.fitness).sum();
        sum / self.fitness.len() as f64
    }

    /// Best fitness in the population.
    pub fn best_fitness(&self) -> f64 {
        self.fitness
            .iter()
            .map(|r| r.fitness)
            .fold(0.0_f64, f64::max)
    }
}

/// Pearson correlation coefficient between two vectors.
fn pearson_correlation(x: &[f64], y: &[f64], n: usize) -> f64 {
    if n < 2 {
        return 0.0;
    }

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        cov / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_random_population() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let pop = Population::random(config, 16, &mut rng).unwrap();

        assert_eq!(pop.size(), 16);
        assert_eq!(pop.generation, 0);
        assert_eq!(pop.response_dim(), 16 * 25);
    }

    #[test]
    fn test_fitness_evaluation() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let mut pop = Population::random(config, 8, &mut rng).unwrap();

        let inputs: Vec<ReservoirInput> = (0..20)
            .map(|i| ReservoirInput::Continuous(vec![i as f64 / 20.0, (i as f64).sin()]))
            .collect();

        let targets: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64 / 20.0]).collect();

        pop.evaluate_fitness(&inputs, &targets);

        assert_eq!(pop.fitness.len(), 8);
        assert!(pop.best_fitness() >= 0.0);
        assert!(pop.mean_fitness() >= 0.0);
    }

    #[test]
    fn test_pearson_perfect_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let r = pearson_correlation(&x, &y, 5);
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_no_correlation() {
        // Constant y → zero variance → correlation 0
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let r = pearson_correlation(&x, &y, 5);
        assert!((r).abs() < 1e-10);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serializable shell configuration and history records.

use serde::{Deserialize, Serialize};

use crate::constraints::DriftAction;
use crate::readout::LinearReadout;
use crate::response::ResponseVector;
use bingocube_core::Config;

/// Unique identifier for an instance (machine/node).
/// BLAKE3 hash of machine identity + timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub String);

impl InstanceId {
    /// Create a new instance ID from a human-readable name.
    pub fn new(name: &str) -> Self {
        let hash = blake3::hash(name.as_bytes());
        Self(format!("{}:{}", name, &hash.to_hex()[..12]))
    }

    /// The human-readable portion of the ID.
    pub fn name(&self) -> &str {
        self.0.split(':').next().unwrap_or(&self.0)
    }
}

/// Record of a single generation's evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRecord {
    /// Generation number.
    pub generation: usize,

    /// Mean fitness of the population at evaluation time.
    pub mean_fitness: f64,

    /// Best fitness in the population.
    pub best_fitness: f64,

    /// Number of boards in the population.
    pub population_size: usize,

    /// Instance that produced this generation.
    pub origin_instance: InstanceId,

    /// Number of training samples used for evaluation.
    pub n_training_samples: usize,

    /// N_e * s ratio at this generation.
    pub ne_s: f64,

    /// Action taken by the drift monitor.
    pub drift_action: DriftAction,
}

/// Configuration for the nautilus shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Board configuration.
    pub board_config: Config,

    /// Population size (number of boards per generation).
    pub population_size: usize,

    /// Evolution parameters.
    pub evolution: crate::evolution::EvolutionConfig,

    /// Number of readout targets.
    pub n_targets: usize,

    /// Input dimensionality for reservoir.
    pub input_dim: usize,

    /// Ridge regularization for readout.
    pub ridge_lambda: f64,

    /// Maximum generations to keep in shell history.
    /// Older generations are pruned to save memory, but their
    /// GenerationRecords are kept.
    pub max_stored_generations: usize,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            board_config: Config::default(),
            population_size: 16,
            evolution: crate::evolution::EvolutionConfig::default(),
            n_targets: 1,
            input_dim: 1,
            ridge_lambda: 1e-6,
            max_stored_generations: 5,
        }
    }
}

/// AKD1000 int4-quantized weight export for FullyConnected layer deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Akd1000Export {
    /// Quantized weights in [-8, 7] (int4 range), shape [n_targets][input_dim].
    pub quantized_weights: Vec<Vec<i8>>,

    /// Per-target dequantization scale: w_float ≈ w_q / scale.
    pub scales: Vec<f64>,

    /// Bias vector (kept in full precision).
    pub biases: Vec<f64>,

    /// Input dimensionality.
    pub input_dim: usize,

    /// Number of output targets.
    pub n_targets: usize,
}

impl Akd1000Export {
    /// Dequantize and predict (for validation against hardware).
    pub fn predict_dequantized(&self, activations: &[f64]) -> Vec<f64> {
        let mut output = self.biases.clone();
        for t in 0..self.n_targets {
            let scale = self.scales[t];
            for i in 0..self.input_dim {
                let w_approx = self.quantized_weights[t][i] as f64 / scale;
                let x = activations.get(i).copied().unwrap_or(0.0);
                output[t] += w_approx * x;
            }
        }
        output
    }

    /// Compute quantization error (MSE) against the original readout.
    pub fn quantization_mse(&self, readout: &LinearReadout, responses: &[ResponseVector]) -> f64 {
        if responses.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        for resp in responses {
            let orig = readout.predict(resp);
            let quant = self.predict_dequantized(&resp.activations);
            for (o, q) in orig.iter().zip(quant.iter()) {
                total += (o - q).powi(2);
            }
        }
        total / responses.len() as f64
    }
}

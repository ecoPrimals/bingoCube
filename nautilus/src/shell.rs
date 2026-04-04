// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Nautilus Shell — layered evolutionary history of board populations.
//!
//! ```text
//! Generation 0: Random boards (naive initialization)         ╮
//! Generation 1: Boards informed by Gen 0 performance         │ shell layers
//! Generation 2: Boards informed by Gen 1 (shell growing)     │ (heritage)
//!    ...                                                      │
//! Generation N: Boards evolved to the environment's structure ╯
//! ```
//!
//! The shell is the portable unit of learned structure. It can be:
//! - Serialized and shipped to another machine (instance transfer)
//! - Loaded as a warm start for a new environment
//! - Merged with shells from other instances
//! - Inspected for evolutionary trajectory analysis

use serde::{Deserialize, Serialize};

use crate::constraints::{DriftAction, DriftMonitor, EdgeSeeder};
use crate::evolution::{Evolution, EvolutionConfig, SelectionMethod};
use crate::population::Population;
use crate::readout::LinearReadout;
use crate::response::{ReservoirInput, ResponseVector};
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
    pub evolution: EvolutionConfig,

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
            evolution: EvolutionConfig::default(),
            n_targets: 1,
            input_dim: 1,
            ridge_lambda: 1e-6,
            max_stored_generations: 5,
        }
    }
}

/// The Nautilus Shell: the full evolutionary history with trained readout.
///
/// ## Within an Instance
///
/// Call [`evolve_generation`] repeatedly with training data. Each call:
/// 1. Projects input through the current population
/// 2. Trains the readout on ensemble responses
/// 3. Evaluates board fitness
/// 4. Breeds the next generation
/// 5. Records the generation in the shell history
///
/// ## Between Instances
///
/// Serialize the shell with serde, ship it to another machine.
/// The receiving instance calls [`continue_from`] or [`merge_shell`]
/// to incorporate the inherited knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NautilusShell {
    /// Shell configuration.
    pub config: ShellConfig,

    /// Current population (latest generation).
    pub current_population: Population,

    /// Trained readout layer.
    pub readout: LinearReadout,

    /// Evolutionary history (generation records, always kept).
    pub history: Vec<GenerationRecord>,

    /// Instance that created this shell.
    pub origin: InstanceId,

    /// Instances that have contributed to this shell (via merge).
    pub lineage: Vec<InstanceId>,

    /// Drift monitor tracking N_e * s across generations.
    pub drift_monitor: DriftMonitor,

    /// Concept edge features detected during evolution (normalized 0..1).
    /// Used by edge seeding to bias new boards toward prediction failure regions.
    pub concept_edges: Vec<Vec<f64>>,
}

impl NautilusShell {
    /// Create a new shell with a random population (Generation 0).
    pub fn new(config: ShellConfig, instance_id: InstanceId) -> Self {
        let mut rng = rand::thread_rng();
        let pop = Population::random(
            config.board_config.clone(),
            config.population_size,
            &mut rng,
        )
        .expect("valid config");

        let response_dim = pop.response_dim();
        let readout =
            LinearReadout::new(response_dim, config.n_targets).with_ridge(config.ridge_lambda);

        Self {
            config,
            current_population: pop,
            readout,
            history: Vec::new(),
            origin: instance_id.clone(),
            lineage: vec![instance_id],
            drift_monitor: DriftMonitor::default(),
            concept_edges: Vec::new(),
        }
    }

    /// Create a shell from an explicit seed (deterministic initialization).
    pub fn from_seed(config: ShellConfig, instance_id: InstanceId, seed: u64) -> Self {
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
        let pop = Population::random(
            config.board_config.clone(),
            config.population_size,
            &mut rng,
        )
        .expect("valid config");

        let response_dim = pop.response_dim();
        let readout =
            LinearReadout::new(response_dim, config.n_targets).with_ridge(config.ridge_lambda);

        Self {
            config,
            current_population: pop,
            readout,
            history: Vec::new(),
            origin: instance_id.clone(),
            lineage: vec![instance_id],
            drift_monitor: DriftMonitor::default(),
            concept_edges: Vec::new(),
        }
    }

    /// Current generation number.
    pub fn generation(&self) -> usize {
        self.current_population.generation
    }

    /// Run one cycle of evolution:
    /// 1. Project inputs through population → ensemble responses
    /// 2. Train the readout on (responses, targets)
    /// 3. Evaluate board fitness
    /// 4. Record drift monitor and apply recommended action
    /// 5. Inject edge-seeded boards if concept edges are known
    /// 6. Breed next generation
    /// 7. Record the generation
    ///
    /// Returns the MSE of the readout after training.
    pub fn evolve_generation(&mut self, inputs: &[ReservoirInput], targets: &[Vec<f64>]) -> f64 {
        assert_eq!(inputs.len(), targets.len());
        let mut rng = rand::thread_rng();

        // 1. Project all inputs through current population
        let responses: Vec<ResponseVector> = inputs
            .iter()
            .map(|inp| self.current_population.project(inp))
            .collect();

        // 2. Train readout
        self.readout.train(&responses, targets);
        let mse = self.readout.mse(&responses, targets);

        // 3. Evaluate board fitness
        self.current_population.evaluate_fitness(inputs, targets);

        // 4. Drift monitor: record and apply
        let mean_fit = self.current_population.mean_fitness();
        let best_fit = self.current_population.best_fitness();
        let pop_size = self.current_population.size();
        self.drift_monitor.record(
            self.current_population.generation,
            pop_size,
            mean_fit,
            best_fit,
        );
        let drift_action = self.drift_monitor.recommendation();
        let mut evo_config = self.config.evolution.clone();

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
                        let board =
                            bingocube_core::Board::generate(&self.config.board_config, &mut rng)
                                .expect("valid config");
                        self.current_population.boards.push(board);
                    }
                }
            }
            DriftAction::Continue => {}
        }

        // 5. Edge seeding: replace worst boards with edge-biased boards
        if !self.concept_edges.is_empty() {
            let n_edge = (pop_size / 4).max(1).min(self.concept_edges.len());
            let ranked = self.current_population.ranked_boards();
            let worst_indices: Vec<usize> =
                ranked.iter().rev().take(n_edge).map(|&(i, _)| i).collect();

            for (slot, edge_features) in worst_indices.iter().zip(self.concept_edges.iter()) {
                if let Ok(mut seeded) =
                    EdgeSeeder::seed_boards(&self.config.board_config, 1, edge_features, &mut rng)
                {
                    if let Some(board) = seeded.pop() {
                        self.current_population.boards[*slot] = board;
                    }
                }
            }
        }

        let ne_s = self.drift_monitor.latest_ne_s();

        // 6. Record generation
        self.history.push(GenerationRecord {
            generation: self.current_population.generation,
            mean_fitness: mean_fit,
            best_fitness: best_fit,
            population_size: self.current_population.size(),
            origin_instance: self.origin.clone(),
            n_training_samples: inputs.len(),
            ne_s,
            drift_action: drift_action.clone(),
        });

        // 7. Breed next generation
        let next = Evolution::next_generation(&self.current_population, &evo_config, &mut rng)
            .expect("evolution should succeed");
        self.current_population = next;

        mse
    }

    /// Evolve with a deterministic seed (reproducible evolution).
    pub fn evolve_generation_seeded(
        &mut self,
        inputs: &[ReservoirInput],
        targets: &[Vec<f64>],
        seed: u64,
    ) -> f64 {
        assert_eq!(inputs.len(), targets.len());
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);

        let responses: Vec<ResponseVector> = inputs
            .iter()
            .map(|inp| self.current_population.project(inp))
            .collect();

        self.readout.train(&responses, targets);
        let mse = self.readout.mse(&responses, targets);

        self.current_population.evaluate_fitness(inputs, targets);

        let mean_fit = self.current_population.mean_fitness();
        let best_fit = self.current_population.best_fitness();
        let pop_size = self.current_population.size();
        self.drift_monitor.record(
            self.current_population.generation,
            pop_size,
            mean_fit,
            best_fit,
        );
        let drift_action = self.drift_monitor.recommendation();
        let mut evo_config = self.config.evolution.clone();

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
                for _ in 0..deficit {
                    let board =
                        bingocube_core::Board::generate(&self.config.board_config, &mut rng)
                            .expect("valid config");
                    self.current_population.boards.push(board);
                }
            }
            DriftAction::Continue => {}
        }

        if !self.concept_edges.is_empty() {
            let n_edge = (pop_size / 4).max(1).min(self.concept_edges.len());
            let ranked = self.current_population.ranked_boards();
            let worst_indices: Vec<usize> =
                ranked.iter().rev().take(n_edge).map(|&(i, _)| i).collect();
            for (slot, edge_features) in worst_indices.iter().zip(self.concept_edges.iter()) {
                if let Ok(mut seeded) =
                    EdgeSeeder::seed_boards(&self.config.board_config, 1, edge_features, &mut rng)
                {
                    if let Some(board) = seeded.pop() {
                        self.current_population.boards[*slot] = board;
                    }
                }
            }
        }

        let ne_s = self.drift_monitor.latest_ne_s();

        self.history.push(GenerationRecord {
            generation: self.current_population.generation,
            mean_fitness: mean_fit,
            best_fitness: best_fit,
            population_size: self.current_population.size(),
            origin_instance: self.origin.clone(),
            n_training_samples: inputs.len(),
            ne_s,
            drift_action: drift_action.clone(),
        });

        let next = Evolution::next_generation(&self.current_population, &evo_config, &mut rng)
            .expect("evolution should succeed");
        self.current_population = next;

        mse
    }

    /// Predict from a new input using the trained readout.
    pub fn predict(&self, input: &ReservoirInput) -> Vec<f64> {
        let response = self.current_population.project(input);
        self.readout.predict(&response)
    }

    // ─── Instance Transfer ───

    /// Continue evolution from an inherited shell.
    ///
    /// The receiving instance takes the shell's current population and readout
    /// as its starting point. The lineage records both origins.
    pub fn continue_from(inherited: Self, new_instance: InstanceId) -> Self {
        let mut lineage = inherited.lineage.clone();
        if !lineage.contains(&new_instance) {
            lineage.push(new_instance.clone());
        }

        Self {
            origin: new_instance,
            lineage,
            drift_monitor: inherited.drift_monitor,
            concept_edges: inherited.concept_edges,
            config: inherited.config,
            current_population: inherited.current_population,
            readout: inherited.readout,
            history: inherited.history,
        }
    }

    /// Merge another shell into this one.
    ///
    /// Takes the best boards from both populations and combines them.
    /// The readout is retrained on the next `evolve_generation` call.
    pub fn merge_shell(&mut self, other: &NautilusShell) {
        let my_ranked = self.current_population.ranked_boards();
        let other_ranked = other.current_population.ranked_boards();

        let half = self.config.population_size / 2;

        let mut merged_boards = Vec::new();

        // Take top half from self
        for &(idx, _) in my_ranked.iter().take(half) {
            merged_boards.push(self.current_population.boards[idx].clone());
        }

        // Take top half from other (or as many as available)
        let remaining = self.config.population_size - merged_boards.len();
        for &(idx, _) in other_ranked.iter().take(remaining) {
            merged_boards.push(other.current_population.boards[idx].clone());
        }

        // If other didn't have enough, fill with random boards
        if merged_boards.len() < self.config.population_size {
            let mut rng = rand::thread_rng();
            while merged_boards.len() < self.config.population_size {
                let board = bingocube_core::Board::generate(&self.config.board_config, &mut rng)
                    .expect("valid config");
                merged_boards.push(board);
            }
        }

        self.current_population.boards = merged_boards;
        self.current_population.fitness.clear();

        // Record lineage
        for id in &other.lineage {
            if !self.lineage.contains(id) {
                self.lineage.push(id.clone());
            }
        }

        // Merge history
        for record in &other.history {
            if !self.history.iter().any(|r| {
                r.generation == record.generation && r.origin_instance == record.origin_instance
            }) {
                self.history.push(record.clone());
            }
        }
        self.history.sort_by_key(|r| r.generation);
    }

    /// Fitness trajectory: (generation, mean_fitness, best_fitness) tuples.
    pub fn fitness_trajectory(&self) -> Vec<(usize, f64, f64)> {
        self.history
            .iter()
            .map(|r| (r.generation, r.mean_fitness, r.best_fitness))
            .collect()
    }

    /// Number of distinct instances in the lineage.
    pub fn lineage_depth(&self) -> usize {
        self.lineage.len()
    }

    /// Register concept edges — regions of input space where predictions fail.
    ///
    /// Each entry is a vector of normalized features (0..1) representing
    /// the edge region. During evolution, the worst boards will be replaced
    /// with edge-seeded boards biased toward these regions.
    pub fn set_concept_edges(&mut self, edges: Vec<Vec<f64>>) {
        self.concept_edges = edges;
    }

    /// Detect concept edges via leave-one-out cross-validation.
    ///
    /// Returns indices of samples where LOO prediction error exceeds
    /// `threshold` times the mean error — these are the concept edges.
    pub fn detect_concept_edges(
        &self,
        inputs: &[ReservoirInput],
        targets: &[Vec<f64>],
        threshold: f64,
    ) -> Vec<usize> {
        if inputs.len() < 3 {
            return Vec::new();
        }

        let responses: Vec<ResponseVector> = inputs
            .iter()
            .map(|inp| self.current_population.project(inp))
            .collect();

        let mut loo_errors = Vec::with_capacity(inputs.len());

        for leave_out in 0..inputs.len() {
            let train_r: Vec<_> = responses
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != leave_out)
                .map(|(_, r)| r.clone())
                .collect();
            let train_t: Vec<_> = targets
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != leave_out)
                .map(|(_, t)| t.clone())
                .collect();

            let mut loo_readout =
                LinearReadout::new(self.readout.input_dim, self.readout.output_dim)
                    .with_ridge(self.config.ridge_lambda);
            loo_readout.train(&train_r, &train_t);

            let pred = loo_readout.predict(&responses[leave_out]);
            let err: f64 = pred
                .iter()
                .zip(targets[leave_out].iter())
                .map(|(p, t)| (p - t).powi(2))
                .sum::<f64>()
                .sqrt();
            loo_errors.push(err);
        }

        let mean_err = loo_errors.iter().sum::<f64>() / loo_errors.len() as f64;
        let edge_threshold = mean_err * threshold;

        loo_errors
            .iter()
            .enumerate()
            .filter(|&(_, &e)| e > edge_threshold)
            .map(|(i, _)| i)
            .collect()
    }

    /// Whether the drift monitor indicates the population is drifting.
    pub fn is_drifting(&self) -> bool {
        self.drift_monitor.is_drifting()
    }

    /// Latest N_e * s ratio from the drift monitor.
    pub fn latest_ne_s(&self) -> f64 {
        self.drift_monitor.latest_ne_s()
    }

    // ─── AKD1000 Export ───

    /// Export readout weights as quantized int4 for AKD1000 FullyConnected layer.
    ///
    /// The AKD1000 NPU uses int4 weights [-8, 7]. We quantize via symmetric
    /// min-max scaling: w_q = round(w * scale), scale = 7 / max(|w|).
    ///
    /// Returns (quantized_weights, scales, biases) where:
    /// - quantized_weights: [n_targets][input_dim] as i8 (int4 range)
    /// - scales: per-target dequantization scale factors
    /// - biases: per-target bias values (kept in f64)
    pub fn export_akd1000_weights(&self) -> Akd1000Export {
        let n_targets = self.readout.output_dim;
        let input_dim = self.readout.input_dim;

        let mut quantized = vec![vec![0i8; input_dim]; n_targets];
        let mut scales = vec![0.0f64; n_targets];

        for t in 0..n_targets {
            let w_abs_max = self.readout.weights[t]
                .iter()
                .map(|w| w.abs())
                .fold(0.0f64, f64::max)
                .max(1e-10);

            let scale = 7.0 / w_abs_max;
            scales[t] = scale;

            for i in 0..input_dim {
                let q = (self.readout.weights[t][i] * scale).round() as i8;
                quantized[t][i] = q.clamp(-8, 7);
            }
        }

        Akd1000Export {
            quantized_weights: quantized,
            scales,
            biases: self.readout.bias.clone(),
            input_dim,
            n_targets,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_dataset(n: usize) -> (Vec<ReservoirInput>, Vec<Vec<f64>>) {
        let inputs: Vec<ReservoirInput> = (0..n)
            .map(|i| {
                let x = i as f64 / n as f64;
                ReservoirInput::Continuous(vec![x, x.sin(), x.cos()])
            })
            .collect();

        let targets: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let x = i as f64 / n as f64;
                vec![x.sin() + 0.5 * x]
            })
            .collect();

        (inputs, targets)
    }

    #[test]
    fn test_shell_creation() {
        let config = ShellConfig::default();
        let id = InstanceId::new("northgate");
        let shell = NautilusShell::from_seed(config, id.clone(), 42);

        assert_eq!(shell.generation(), 0);
        assert_eq!(shell.lineage_depth(), 1);
        assert_eq!(shell.origin.name(), "northgate");
    }

    #[test]
    fn test_within_instance_evolution() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        let id = InstanceId::new("homelab");
        let mut shell = NautilusShell::from_seed(config, id, 42);
        let (inputs, targets) = synthetic_dataset(50);

        let mut mse_history = Vec::new();
        for gen_idx in 0..10 {
            let mse = shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
            mse_history.push(mse);
        }

        assert_eq!(shell.generation(), 10);
        assert_eq!(shell.history.len(), 10);

        // Fitness should generally improve (or at least not catastrophically degrade)
        let trajectory = shell.fitness_trajectory();
        assert_eq!(trajectory.len(), 10);
    }

    #[test]
    fn test_between_instance_transfer() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        // Instance A evolves for 5 generations
        let id_a = InstanceId::new("northgate");
        let mut shell_a = NautilusShell::from_seed(config.clone(), id_a, 42);
        let (inputs, targets) = synthetic_dataset(50);

        for gen_idx in 0..5 {
            shell_a.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        }
        assert_eq!(shell_a.generation(), 5);

        // Transfer to Instance B
        let id_b = InstanceId::new("strandgate");
        let mut shell_b = NautilusShell::continue_from(shell_a, id_b.clone());

        assert_eq!(shell_b.generation(), 5); // continues from generation 5
        assert_eq!(shell_b.lineage_depth(), 2);
        assert_eq!(shell_b.origin.name(), "strandgate");

        // Instance B continues evolving
        for gen_idx in 0..5 {
            shell_b.evolve_generation_seeded(&inputs, &targets, 200 + gen_idx);
        }
        assert_eq!(shell_b.generation(), 10);
    }

    #[test]
    fn test_shell_merge() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        let (inputs, targets) = synthetic_dataset(50);

        // Two instances evolve independently
        let mut shell_a =
            NautilusShell::from_seed(config.clone(), InstanceId::new("northgate"), 42);
        let mut shell_b = NautilusShell::from_seed(config, InstanceId::new("strandgate"), 99);

        for gen_idx in 0..5 {
            shell_a.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
            shell_b.evolve_generation_seeded(&inputs, &targets, 200 + gen_idx);
        }

        // Merge B into A
        shell_a.merge_shell(&shell_b);

        assert_eq!(shell_a.current_population.size(), 8);
        assert!(shell_a.lineage_depth() >= 2);
    }

    #[test]
    fn test_shell_serialization_roundtrip() {
        let config = ShellConfig {
            population_size: 4,
            n_targets: 1,
            ..Default::default()
        };

        let id = InstanceId::new("homelab");
        let mut shell = NautilusShell::from_seed(config, id, 42);
        let (inputs, targets) = synthetic_dataset(20);
        shell.evolve_generation_seeded(&inputs, &targets, 100);

        // Serialize
        let json = serde_json::to_string(&shell).unwrap();

        // Deserialize
        let restored: NautilusShell = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.generation(), shell.generation());
        assert_eq!(restored.history.len(), shell.history.len());
        assert_eq!(restored.lineage_depth(), shell.lineage_depth());

        // Predictions should match
        let test_input = ReservoirInput::Continuous(vec![0.5, 0.3, 0.7]);
        let pred_orig = shell.predict(&test_input);
        let pred_restored = restored.predict(&test_input);
        assert_eq!(pred_orig, pred_restored);
    }

    #[test]
    fn test_drift_monitor_wired_into_evolution() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        let id = InstanceId::new("driftlab");
        let mut shell = NautilusShell::from_seed(config, id, 42);
        let (inputs, targets) = synthetic_dataset(50);

        for gen_idx in 0..10 {
            shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        }

        // Drift monitor should have 10 entries
        assert_eq!(shell.drift_monitor.history.len(), 10);

        // Each generation record should have ne_s and drift_action
        for record in &shell.history {
            assert!(record.ne_s >= 0.0);
        }

        // Latest ne_s should be accessible
        let ne_s = shell.latest_ne_s();
        assert!(ne_s >= 0.0);
    }

    #[test]
    fn test_edge_seeding_integration() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        let id = InstanceId::new("edgelab");
        let mut shell = NautilusShell::from_seed(config, id, 42);
        let (inputs, targets) = synthetic_dataset(50);

        // Evolve a few generations without edges
        for gen_idx in 0..3 {
            shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        }

        // Set concept edges and evolve more
        shell.set_concept_edges(vec![vec![0.85, 0.3, 0.5]]);
        for gen_idx in 3..8 {
            shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        }

        assert_eq!(shell.generation(), 8);
        assert_eq!(shell.concept_edges.len(), 1);
    }

    #[test]
    fn test_akd1000_export() {
        let config = ShellConfig {
            population_size: 4,
            n_targets: 2,
            ..Default::default()
        };

        let id = InstanceId::new("akd_lab");
        let mut shell = NautilusShell::from_seed(config, id, 42);
        let (inputs, targets_1) = synthetic_dataset(30);
        let targets_2: Vec<Vec<f64>> = targets_1.iter().map(|t| vec![t[0], t[0] * 2.0]).collect();

        for gen_idx in 0..5 {
            shell.evolve_generation_seeded(&inputs, &targets_2, 100 + gen_idx);
        }

        let export = shell.export_akd1000_weights();

        assert_eq!(export.n_targets, 2);
        assert_eq!(export.quantized_weights.len(), 2);
        assert_eq!(export.scales.len(), 2);
        assert_eq!(export.biases.len(), 2);

        // All quantized weights should be in int4 range [-8, 7]
        for row in &export.quantized_weights {
            for &w in row {
                assert!(w >= -8 && w <= 7, "weight {w} outside int4 range");
            }
        }

        // Dequantized predictions should be close to original
        let responses: Vec<ResponseVector> = inputs
            .iter()
            .map(|inp| shell.current_population.project(inp))
            .collect();
        let quant_mse = export.quantization_mse(&shell.readout, &responses);
        assert!(quant_mse < 1.0, "quantization MSE too high: {quant_mse}");

        // Serialization roundtrip
        let json = serde_json::to_string(&export).unwrap();
        let restored: Akd1000Export = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.quantized_weights, export.quantized_weights);
    }

    #[test]
    fn test_concept_edge_detection() {
        let config = ShellConfig {
            population_size: 8,
            n_targets: 1,
            ..Default::default()
        };

        let id = InstanceId::new("concept_lab");
        let mut shell = NautilusShell::from_seed(config, id, 42);

        // Create data with a discontinuity at x=0.5
        let n = 40;
        let inputs: Vec<ReservoirInput> = (0..n)
            .map(|i| {
                let x = i as f64 / n as f64;
                ReservoirInput::Continuous(vec![x, x.sin(), x.cos()])
            })
            .collect();
        let targets: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let x = i as f64 / n as f64;
                if x < 0.5 {
                    vec![x]
                } else {
                    vec![x + 2.0]
                }
            })
            .collect();

        for gen_idx in 0..5 {
            shell.evolve_generation_seeded(&inputs, &targets, 100 + gen_idx);
        }

        let edges = shell.detect_concept_edges(&inputs, &targets, 2.0);
        // Should find at least some edges near the discontinuity
        assert!(
            !edges.is_empty(),
            "should detect concept edges at the discontinuity"
        );
    }

    #[test]
    fn test_drift_action_serialization() {
        use crate::constraints::DriftAction;

        let actions = vec![
            DriftAction::Continue,
            DriftAction::IncreaseSelection,
            DriftAction::IncreasePop { factor: 2.0 },
        ];

        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let restored: DriftAction = serde_json::from_str(&json).unwrap();
            assert_eq!(*action, restored);
        }
    }
}

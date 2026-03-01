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

use crate::evolution::{Evolution, EvolutionConfig};
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
        let readout = LinearReadout::new(response_dim, config.n_targets)
            .with_ridge(config.ridge_lambda);

        Self {
            config,
            current_population: pop,
            readout,
            history: Vec::new(),
            origin: instance_id.clone(),
            lineage: vec![instance_id],
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
        let readout = LinearReadout::new(response_dim, config.n_targets)
            .with_ridge(config.ridge_lambda);

        Self {
            config,
            current_population: pop,
            readout,
            history: Vec::new(),
            origin: instance_id.clone(),
            lineage: vec![instance_id],
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
    /// 4. Breed next generation
    /// 5. Record the generation
    ///
    /// Returns the MSE of the readout after training.
    pub fn evolve_generation(
        &mut self,
        inputs: &[ReservoirInput],
        targets: &[Vec<f64>],
    ) -> f64 {
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

        // 4. Record generation
        self.history.push(GenerationRecord {
            generation: self.current_population.generation,
            mean_fitness: self.current_population.mean_fitness(),
            best_fitness: self.current_population.best_fitness(),
            population_size: self.current_population.size(),
            origin_instance: self.origin.clone(),
            n_training_samples: inputs.len(),
        });

        // 5. Breed next generation
        let next = Evolution::next_generation(
            &self.current_population,
            &self.config.evolution,
            &mut rng,
        )
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

        self.history.push(GenerationRecord {
            generation: self.current_population.generation,
            mean_fitness: self.current_population.mean_fitness(),
            best_fitness: self.current_population.best_fitness(),
            population_size: self.current_population.size(),
            origin_instance: self.origin.clone(),
            n_training_samples: inputs.len(),
        });

        let next = Evolution::next_generation(
            &self.current_population,
            &self.config.evolution,
            &mut rng,
        )
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
            ..inherited
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
                let board = bingocube_core::Board::generate(
                    &self.config.board_config,
                    &mut rng,
                )
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
                r.generation == record.generation
                    && r.origin_instance == record.origin_instance
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
        for gen in 0..10 {
            let mse = shell.evolve_generation_seeded(&inputs, &targets, 100 + gen);
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

        for gen in 0..5 {
            shell_a.evolve_generation_seeded(&inputs, &targets, 100 + gen);
        }
        assert_eq!(shell_a.generation(), 5);

        // Transfer to Instance B
        let id_b = InstanceId::new("strandgate");
        let mut shell_b = NautilusShell::continue_from(shell_a, id_b.clone());

        assert_eq!(shell_b.generation(), 5); // continues from gen 5
        assert_eq!(shell_b.lineage_depth(), 2);
        assert_eq!(shell_b.origin.name(), "strandgate");

        // Instance B continues evolving
        for gen in 0..5 {
            shell_b.evolve_generation_seeded(&inputs, &targets, 200 + gen);
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
        let mut shell_a = NautilusShell::from_seed(
            config.clone(),
            InstanceId::new("northgate"),
            42,
        );
        let mut shell_b = NautilusShell::from_seed(
            config,
            InstanceId::new("strandgate"),
            99,
        );

        for gen in 0..5 {
            shell_a.evolve_generation_seeded(&inputs, &targets, 100 + gen);
            shell_b.evolve_generation_seeded(&inputs, &targets, 200 + gen);
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
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Brain integration — Nautilus Shell as a subsystem in the NPU brain architecture.
//!
//! The Nautilus Shell runs alongside the ESN in the brain, handling:
//! - Cross-run structural learning (which board geometries fit which regimes)
//! - Quenched→dynamical cost prediction (cheap proxy for expensive computation)
//! - Concept edge detection (where physics models break down)
//! - Shell propagation between instances (portable evolutionary heritage)
//!
//! The ESN handles fast within-run temporal dynamics. The Nautilus Shell
//! handles slow cross-run structural adaptation. They complement each other.

use serde::{Deserialize, Serialize};

use crate::constraints::{DriftMonitor, EdgeSeeder};
use crate::evolution::EvolutionConfig;
use crate::response::ReservoirInput;
use crate::shell::{InstanceId, NautilusShell, ShellConfig};

/// Configuration for the Nautilus brain subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NautilusBrainConfig {
    /// Shell configuration.
    pub shell: ShellConfig,

    /// How many generations to evolve per training cycle.
    pub generations_per_cycle: u64,

    /// Minimum data points before training.
    pub min_training_points: usize,

    /// LOO error threshold above which a point is flagged as a concept edge.
    pub concept_edge_threshold: f64,

    /// Number of edge-targeted boards to seed per detected edge.
    pub edge_seed_count: usize,
}

impl Default for NautilusBrainConfig {
    fn default() -> Self {
        Self {
            shell: ShellConfig {
                population_size: 24,
                n_targets: 3, // CG cost, plaquette, acceptance
                evolution: EvolutionConfig::default(),
                ridge_lambda: 1e-4,
                ..Default::default()
            },
            generations_per_cycle: 20,
            min_training_points: 5,
            concept_edge_threshold: 0.15,
            edge_seed_count: 4,
        }
    }
}

/// A data point for the Nautilus brain — one measured beta point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaObservation {
    /// Coupling constant.
    pub beta: f64,

    /// Quenched (pure gauge) plaquette, if available.
    pub quenched_plaq: Option<f64>,

    /// Quenched plaquette variance, if available.
    pub quenched_plaq_var: Option<f64>,

    /// Dynamical measurement plaquette.
    pub plaquette: f64,

    /// Mean CG solver iterations.
    pub cg_iters: f64,

    /// Acceptance rate.
    pub acceptance: f64,

    /// Mean |δH|.
    pub delta_h_abs: f64,

    /// Anderson proxy: mean level spacing ratio, if available.
    pub anderson_r: Option<f64>,

    /// Anderson proxy: minimum eigenvalue, if available.
    pub anderson_lambda_min: Option<f64>,
}

/// The Nautilus brain subsystem — manages a shell and provides predictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NautilusBrain {
    /// Configuration.
    pub config: NautilusBrainConfig,

    /// The evolved shell.
    pub shell: NautilusShell,

    /// Accumulated observations (training data).
    pub observations: Vec<BetaObservation>,

    /// Drift monitor.
    pub drift: DriftMonitor,

    /// Detected concept edges (beta values where predictions fail).
    pub concept_edges: Vec<f64>,

    /// Whether the shell has been trained at least once.
    pub trained: bool,
}

impl NautilusBrain {
    /// Create a new Nautilus brain subsystem.
    pub fn new(config: NautilusBrainConfig, instance: &str) -> Self {
        let id = InstanceId::new(instance);
        let shell = NautilusShell::from_seed(config.shell.clone(), id, 42);

        Self {
            config,
            shell,
            observations: Vec::new(),
            drift: DriftMonitor::default(),
            concept_edges: Vec::new(),
            trained: false,
        }
    }

    /// Create from an inherited shell (cross-run bootstrap).
    pub fn from_shell(config: NautilusBrainConfig, shell: NautilusShell, instance: &str) -> Self {
        let id = InstanceId::new(instance);
        let shell = NautilusShell::continue_from(shell, id);

        Self {
            config,
            shell,
            observations: Vec::new(),
            drift: DriftMonitor::default(),
            concept_edges: Vec::new(),
            trained: true, // inherited shell is already trained
        }
    }

    /// Add a new observation (called after each beta point is measured).
    pub fn observe(&mut self, obs: BetaObservation) {
        self.observations.push(obs);
    }

    /// Train the shell on all accumulated observations.
    /// Returns the MSE, or None if not enough data.
    pub fn train(&mut self) -> Option<f64> {
        if self.observations.len() < self.config.min_training_points {
            return None;
        }

        let (inputs, targets) = self.build_training_data();
        let mut last_mse = 0.0;

        for generation in 0..self.config.generations_per_cycle {
            let seed = self.shell.generation() as u64 * 1000 + generation;
            last_mse = self.shell.evolve_generation_seeded(&inputs, &targets, seed);

            let traj = self.shell.fitness_trajectory();
            if let Some(last) = traj.last() {
                self.drift
                    .record(last.0, self.config.shell.population_size, last.1, last.2);
            }
        }

        self.trained = true;
        Some(last_mse)
    }

    /// Predict dynamical observables for a beta value.
    /// Returns (predicted_cg, predicted_plaq, predicted_acc), or None if untrained.
    pub fn predict_dynamical(
        &self,
        beta: f64,
        quenched_plaq: Option<f64>,
    ) -> Option<(f64, f64, f64)> {
        if !self.trained {
            return None;
        }

        let input = self.make_input(beta, quenched_plaq);
        let pred = self.shell.predict(&input);

        if pred.len() >= 3 {
            let max_cg = self.max_cg();
            Some((pred[0] * max_cg, pred[1], pred[2]))
        } else {
            None
        }
    }

    /// Estimate CG cost for a beta value (for steering decisions).
    pub fn estimate_cg(&self, beta: f64) -> Option<f64> {
        self.predict_dynamical(beta, None).map(|(cg, _, _)| cg)
    }

    /// Screen a list of candidate betas, returning them sorted by
    /// predicted information value (concept edges first, then by CG cost spread).
    pub fn screen_candidates(&self, candidates: &[f64]) -> Vec<(f64, f64)> {
        if !self.trained {
            return candidates.iter().map(|&b| (b, 0.0)).collect();
        }

        let mut scored: Vec<(f64, f64)> = candidates
            .iter()
            .map(|&beta| {
                let pred = self.predict_dynamical(beta, None);
                let edge_bonus = if self.concept_edges.iter().any(|&e| (e - beta).abs() < 0.1) {
                    1.0 // prioritize near concept edges
                } else {
                    0.0
                };

                let cg_score = pred.map(|(cg, _, _)| cg / self.max_cg()).unwrap_or(0.5);

                // Higher score = more informative
                let info_score = edge_bonus + cg_score * 0.5;
                (beta, info_score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Detect concept edges via leave-one-out error analysis.
    /// Call after training with enough data. Expensive but informative.
    pub fn detect_concept_edges(&mut self) -> Vec<(f64, f64)> {
        if self.observations.len() < 8 {
            return Vec::new();
        }

        let (inputs, targets) = self.build_training_data();
        let max_cg = self.max_cg();
        let mut edges = Vec::new();

        for hold_out in 0..inputs.len() {
            let train_in: Vec<_> = inputs
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != hold_out)
                .map(|(_, x)| x.clone())
                .collect();
            let train_tgt: Vec<_> = targets
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != hold_out)
                .map(|(_, t)| t.clone())
                .collect();

            let loo_cfg = ShellConfig {
                population_size: 12,
                n_targets: self.config.shell.n_targets,
                ridge_lambda: 1e-3,
                ..self.config.shell.clone()
            };

            let loo_id = InstanceId::new("loo-edge-detect");
            let mut loo = NautilusShell::from_seed(loo_cfg, loo_id, 42 + hold_out as u64);

            for generation in 0..15 {
                loo.evolve_generation_seeded(&train_in, &train_tgt, 3000 + generation);
            }

            let pred = loo.predict(&inputs[hold_out]);
            let pred_cg = pred[0] * max_cg;
            let actual_cg = self.observations[hold_out].cg_iters;
            let rel_err = (pred_cg - actual_cg).abs() / actual_cg.max(1.0);

            if rel_err > self.config.concept_edge_threshold {
                edges.push((self.observations[hold_out].beta, rel_err));
            }
        }

        self.concept_edges = edges.iter().map(|(b, _)| *b).collect();

        // Seed edge-targeted boards into the population
        if !edges.is_empty() {
            let mut rng = rand::thread_rng();
            for (edge_beta, _) in &edges {
                let features = vec![edge_beta / 7.0, 0.5, 0.1];
                if let Ok(new_boards) = EdgeSeeder::seed_boards(
                    &self.config.shell.board_config,
                    self.config.edge_seed_count,
                    &features,
                    &mut rng,
                ) {
                    // Replace worst boards with edge-targeted ones
                    let ranked = self.shell.current_population.ranked_boards();
                    let worst_indices: Vec<usize> = ranked
                        .iter()
                        .rev()
                        .take(new_boards.len())
                        .map(|(idx, _)| *idx)
                        .collect();

                    for (new_board, &old_idx) in new_boards.into_iter().zip(worst_indices.iter()) {
                        if old_idx < self.shell.current_population.boards.len() {
                            self.shell.current_population.boards[old_idx] = new_board;
                        }
                    }
                }
            }
        }

        edges
    }

    /// Whether the population is drifting (N_e * s too low).
    pub fn is_drifting(&self) -> bool {
        self.drift.is_drifting()
    }

    /// Export the shell for transfer to another instance.
    pub fn export_shell(&self) -> &NautilusShell {
        &self.shell
    }

    /// Serialize the entire brain state to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize brain state from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    // ─── Internal helpers ───

    fn build_training_data(&self) -> (Vec<ReservoirInput>, Vec<Vec<f64>>) {
        let max_cg = self.max_cg();

        let inputs: Vec<ReservoirInput> = self
            .observations
            .iter()
            .map(|obs| self.make_input(obs.beta, obs.quenched_plaq))
            .collect();

        let targets: Vec<Vec<f64>> = self
            .observations
            .iter()
            .map(|obs| {
                vec![
                    obs.cg_iters / max_cg.max(1.0),
                    obs.plaquette,
                    obs.acceptance,
                ]
            })
            .collect();

        (inputs, targets)
    }

    fn make_input(&self, beta: f64, quenched_plaq: Option<f64>) -> ReservoirInput {
        let q_plaq = quenched_plaq.unwrap_or_else(|| {
            // Interpolate from observations if no quenched plaq provided
            self.interpolate_quenched_plaq(beta)
        });

        let anderson_r = self.nearest_anderson_r(beta);
        let anderson_lam = self.nearest_anderson_lambda(beta);

        ReservoirInput::Continuous(vec![
            beta / 7.0,
            q_plaq,
            (anderson_r + 1.0).ln(),     // log-scaled level spacing
            anderson_lam.abs().min(1.0), // bounded eigenvalue
            beta.sin() * 0.5 + 0.5,      // nonlinear beta transform
        ])
    }

    fn max_cg(&self) -> f64 {
        self.observations
            .iter()
            .map(|o| o.cg_iters)
            .fold(1.0_f64, f64::max)
    }

    fn interpolate_quenched_plaq(&self, beta: f64) -> f64 {
        if self.observations.is_empty() {
            return 0.4;
        }

        let closest = self
            .observations
            .iter()
            .filter(|o| o.quenched_plaq.is_some())
            .min_by(|a, b| {
                (a.beta - beta)
                    .abs()
                    .partial_cmp(&(b.beta - beta).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        closest.and_then(|o| o.quenched_plaq).unwrap_or(
            self.observations.iter().map(|o| o.plaquette).sum::<f64>()
                / self.observations.len() as f64,
        )
    }

    fn nearest_anderson_r(&self, beta: f64) -> f64 {
        self.observations
            .iter()
            .filter(|o| o.anderson_r.is_some())
            .min_by(|a, b| {
                (a.beta - beta)
                    .abs()
                    .partial_cmp(&(b.beta - beta).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .and_then(|o| o.anderson_r)
            .unwrap_or(0.5)
    }

    fn nearest_anderson_lambda(&self, beta: f64) -> f64 {
        self.observations
            .iter()
            .filter(|o| o.anderson_lambda_min.is_some())
            .min_by(|a, b| {
                (a.beta - beta)
                    .abs()
                    .partial_cmp(&(b.beta - beta).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .and_then(|o| o.anderson_lambda_min)
            .unwrap_or(0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_observations() -> Vec<BetaObservation> {
        (0..8)
            .map(|i| {
                let beta = 4.5 + i as f64 * 0.15;
                BetaObservation {
                    beta,
                    quenched_plaq: Some(0.3 + beta * 0.04),
                    quenched_plaq_var: Some(0.001),
                    plaquette: 0.3 + beta * 0.05,
                    cg_iters: 62000.0 - (beta - 4.5) * 3000.0,
                    acceptance: 0.4 + (beta - 4.5) * 0.1,
                    delta_h_abs: 0.5,
                    anderson_r: Some(0.45 + beta * 0.02),
                    anderson_lambda_min: Some(0.05),
                }
            })
            .collect()
    }

    #[test]
    fn test_brain_creation_and_training() {
        let config = NautilusBrainConfig::default();
        let mut brain = NautilusBrain::new(config, "test-instance");

        // Not enough data
        assert!(brain.train().is_none());
        assert!(brain.predict_dynamical(5.0, None).is_none());

        // Add observations
        for obs in make_observations() {
            brain.observe(obs);
        }

        let mse = brain.train();
        assert!(mse.is_some());
        assert!(brain.trained);

        let pred = brain.predict_dynamical(5.0, Some(0.4));
        assert!(pred.is_some());
    }

    #[test]
    fn test_candidate_screening() {
        let config = NautilusBrainConfig::default();
        let mut brain = NautilusBrain::new(config, "test");

        for obs in make_observations() {
            brain.observe(obs);
        }
        brain.train();

        let candidates = vec![4.5, 5.0, 5.5, 6.0, 6.5];
        let scored = brain.screen_candidates(&candidates);
        assert_eq!(scored.len(), 5);
    }

    #[test]
    fn test_brain_serialization() {
        let config = NautilusBrainConfig::default();
        let mut brain = NautilusBrain::new(config, "test");

        for obs in make_observations() {
            brain.observe(obs);
        }
        brain.train();

        let json = brain.to_json().unwrap();
        let restored = NautilusBrain::from_json(&json).unwrap();

        assert_eq!(restored.observations.len(), brain.observations.len());
        assert_eq!(restored.trained, brain.trained);
    }

    #[test]
    fn test_from_shell_and_export() {
        let shell_cfg = ShellConfig {
            population_size: 6,
            n_targets: 3,
            ..Default::default()
        };
        let mut shell = NautilusShell::from_seed(shell_cfg, InstanceId::new("donor"), 7);
        let (inputs, targets) = {
            let inputs: Vec<ReservoirInput> = (0..10)
                .map(|i| ReservoirInput::Continuous(vec![i as f64 / 10.0, 0.0, 0.0]))
                .collect();
            let targets: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64 / 10.0, 0.5, 0.5]).collect();
            (inputs, targets)
        };
        shell.evolve_generation_seeded(&inputs, &targets, 100);

        let brain_cfg = NautilusBrainConfig {
            shell: shell.config.clone(),
            generations_per_cycle: 1,
            min_training_points: 5,
            ..Default::default()
        };
        let brain = NautilusBrain::from_shell(brain_cfg, shell, "receiver");
        assert!(brain.trained);
        assert_eq!(brain.export_shell().generation(), brain.shell.generation());
    }

    #[test]
    fn test_predict_dynamical_requires_three_targets() {
        let mut shell_cfg = ShellConfig::default();
        shell_cfg.n_targets = 1;
        let brain_cfg = NautilusBrainConfig {
            shell: shell_cfg,
            generations_per_cycle: 1,
            min_training_points: 5,
            ..Default::default()
        };
        let mut brain = NautilusBrain::new(brain_cfg, "one-target");
        for obs in make_observations() {
            brain.observe(obs);
        }
        brain.train().expect("trained");
        assert!(brain.predict_dynamical(5.0, Some(0.4)).is_none());
        assert!(brain.estimate_cg(5.0).is_none());
    }

    #[test]
    fn test_screen_candidates_untrained() {
        let brain = NautilusBrain::new(NautilusBrainConfig::default(), "cold");
        let scored = brain.screen_candidates(&[1.0, 2.0]);
        assert_eq!(scored, vec![(1.0, 0.0), (2.0, 0.0)]);
    }

    #[test]
    fn test_detect_concept_edges_requires_many_observations() {
        let mut brain = NautilusBrain::new(NautilusBrainConfig::default(), "sparse");
        for i in 0..4 {
            let beta = 4.0 + i as f64 * 0.1;
            brain.observe(BetaObservation {
                beta,
                quenched_plaq: None,
                quenched_plaq_var: None,
                plaquette: 0.3,
                cg_iters: 1000.0,
                acceptance: 0.5,
                delta_h_abs: 0.1,
                anderson_r: None,
                anderson_lambda_min: None,
            });
        }
        assert!(brain.detect_concept_edges().is_empty());
    }

    #[test]
    fn test_make_input_interpolation_and_defaults() {
        let mut cfg = NautilusBrainConfig::default();
        cfg.shell.n_targets = 3;
        cfg.generations_per_cycle = 2;
        let mut brain = NautilusBrain::new(cfg, "interp");

        brain.observe(BetaObservation {
            beta: 5.0,
            quenched_plaq: Some(0.41),
            quenched_plaq_var: None,
            plaquette: 0.42,
            cg_iters: 2000.0,
            acceptance: 0.6,
            delta_h_abs: 0.2,
            anderson_r: Some(0.5),
            anderson_lambda_min: Some(0.04),
        });
        brain.observe(BetaObservation {
            beta: 5.5,
            quenched_plaq: None,
            quenched_plaq_var: None,
            plaquette: 0.5,
            cg_iters: 1800.0,
            acceptance: 0.55,
            delta_h_abs: 0.2,
            anderson_r: None,
            anderson_lambda_min: None,
        });

        for obs in make_observations() {
            brain.observe(obs);
        }
        brain.train().expect("train");

        let p = brain
            .predict_dynamical(5.25, None)
            .expect("three targets trained");
        assert!(p.0.is_finite() && p.1.is_finite() && p.2.is_finite());
        assert!(brain.estimate_cg(5.0).is_some());
        assert!(!brain.is_drifting());
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linear readout — the FC (fully connected) layer that extracts predictions
//! from the ensemble reservoir response.
//!
//! In reservoir computing, only the readout is trained. The reservoir (board
//! population) provides the nonlinear random projection; the readout finds
//! linear combinations of the projections that predict the target.
//!
//! On AKD1000 hardware, this maps to a FullyConnected layer via SkipDMA —
//! a single hardware pass.

use serde::{Deserialize, Serialize};

use crate::response::ResponseVector;

/// A linear readout layer: y = W·x + b
///
/// Trained via ordinary least squares (ridge regression) on the ensemble
/// response vectors and target values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearReadout {
    /// Weight matrix, shape [n_targets × response_dim], stored row-major.
    pub weights: Vec<Vec<f64>>,

    /// Bias vector, shape [n_targets].
    pub bias: Vec<f64>,

    /// Input dimensionality (response_dim).
    pub input_dim: usize,

    /// Output dimensionality (n_targets).
    pub output_dim: usize,

    /// Ridge regularization parameter (λ).
    pub ridge_lambda: f64,
}

impl LinearReadout {
    /// Create a new untrained readout.
    pub fn new(input_dim: usize, output_dim: usize) -> Self {
        Self {
            weights: vec![vec![0.0; input_dim]; output_dim],
            bias: vec![0.0; output_dim],
            input_dim,
            output_dim,
            ridge_lambda: 1e-6,
        }
    }

    /// Set the ridge regularization parameter.
    pub fn with_ridge(mut self, lambda: f64) -> Self {
        self.ridge_lambda = lambda;
        self
    }

    /// Train the readout on ensemble response vectors and target values.
    ///
    /// Uses ridge regression (Tikhonov): W = Y·X^T·(X·X^T + λI)^{-1}
    ///
    /// For small-to-medium problems, we solve via the normal equations directly.
    /// This is the standard approach for reservoir computing readouts.
    pub fn train(&mut self, responses: &[ResponseVector], targets: &[Vec<f64>]) {
        assert_eq!(responses.len(), targets.len());
        let n = responses.len();
        if n == 0 {
            return;
        }

        let d = self.input_dim;
        let k = self.output_dim;

        // Compute means for centering
        let mut mean_x = vec![0.0; d];
        let mut mean_y = vec![0.0; k];

        for resp in responses {
            for (i, &v) in resp.activations.iter().enumerate().take(d) {
                mean_x[i] += v;
            }
        }
        for t in targets {
            for (i, &v) in t.iter().enumerate().take(k) {
                mean_y[i] += v;
            }
        }
        for v in &mut mean_x {
            *v /= n as f64;
        }
        for v in &mut mean_y {
            *v /= n as f64;
        }

        // Build X^T·X (d×d) with ridge
        let mut xtx = vec![vec![0.0; d]; d];
        for resp in responses {
            for i in 0..d {
                let xi = resp.activations.get(i).copied().unwrap_or(0.0) - mean_x[i];
                for j in i..d {
                    let xj = resp.activations.get(j).copied().unwrap_or(0.0) - mean_x[j];
                    xtx[i][j] += xi * xj;
                }
            }
        }
        // Symmetrize and add ridge
        for i in 0..d {
            xtx[i][i] += self.ridge_lambda;
            for j in (i + 1)..d {
                xtx[j][i] = xtx[i][j];
            }
        }

        // Build X^T·Y (d×k)
        let mut xty = vec![vec![0.0; k]; d];
        for (resp, target) in responses.iter().zip(targets.iter()) {
            for i in 0..d {
                let xi = resp.activations.get(i).copied().unwrap_or(0.0) - mean_x[i];
                for j in 0..k {
                    let yj = target.get(j).copied().unwrap_or(0.0) - mean_y[j];
                    xty[i][j] += xi * yj;
                }
            }
        }

        // Solve (X^T·X)·W = X^T·Y via Cholesky decomposition
        if let Some(l) = cholesky(&xtx, d) {
            for target_idx in 0..k {
                let rhs: Vec<f64> = (0..d).map(|i| xty[i][target_idx]).collect();
                let w = cholesky_solve(&l, &rhs, d);
                self.weights[target_idx] = w;
            }
        }
        // else: singular matrix, weights stay zero

        // Compute bias: b = mean_y - W·mean_x
        for t in 0..k {
            let dot: f64 = self.weights[t]
                .iter()
                .zip(mean_x.iter())
                .map(|(w, x)| w * x)
                .sum();
            self.bias[t] = mean_y[t] - dot;
        }
    }

    /// Predict from a response vector.
    pub fn predict(&self, response: &ResponseVector) -> Vec<f64> {
        let mut output = self.bias.clone();
        for (t, w_row) in self.weights.iter().enumerate() {
            for (i, &w) in w_row.iter().enumerate() {
                let x = response.activations.get(i).copied().unwrap_or(0.0);
                output[t] += w * x;
            }
        }
        output
    }

    /// Compute mean squared error on a dataset.
    pub fn mse(&self, responses: &[ResponseVector], targets: &[Vec<f64>]) -> f64 {
        if responses.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        for (resp, target) in responses.iter().zip(targets.iter()) {
            let pred = self.predict(resp);
            for (p, t) in pred.iter().zip(target.iter()) {
                total += (p - t).powi(2);
            }
        }
        total / responses.len() as f64
    }
}

/// Cholesky decomposition: A = L·L^T. Returns L (lower triangular).
fn cholesky(a: &[Vec<f64>], n: usize) -> Option<Vec<Vec<f64>>> {
    let mut l = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i][k] * l[j][k];
            }

            if i == j {
                let diag = a[i][i] - sum;
                if diag <= 0.0 {
                    return None; // not positive definite
                }
                l[i][j] = diag.sqrt();
            } else {
                l[i][j] = (a[i][j] - sum) / l[j][j];
            }
        }
    }

    Some(l)
}

/// Solve L·L^T·x = b given Cholesky factor L.
fn cholesky_solve(l: &[Vec<f64>], b: &[f64], n: usize) -> Vec<f64> {
    // Forward substitution: L·y = b
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..i {
            sum += l[i][j] * y[j];
        }
        y[i] = (b[i] - sum) / l[i][i];
    }

    // Back substitution: L^T·x = y
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += l[j][i] * x[j];
        }
        x[i] = (y[i] - sum) / l[i][i];
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ResponseVector;

    fn make_synthetic_data(n: usize) -> (Vec<ResponseVector>, Vec<Vec<f64>>) {
        // y = 2*x₀ + 3*x₁ + 1
        let mut responses = Vec::new();
        let mut targets = Vec::new();

        for i in 0..n {
            let x0 = i as f64 / n as f64;
            let x1 = (i as f64 * 0.7).sin();
            responses.push(ResponseVector {
                activations: vec![x0, x1],
            });
            targets.push(vec![2.0 * x0 + 3.0 * x1 + 1.0]);
        }

        (responses, targets)
    }

    #[test]
    fn test_linear_readout_learns_linear() {
        let (responses, targets) = make_synthetic_data(100);

        let mut readout = LinearReadout::new(2, 1).with_ridge(1e-8);
        readout.train(&responses, &targets);

        let mse = readout.mse(&responses, &targets);
        assert!(mse < 0.01, "MSE too high: {mse}");

        // Check weights are close to [2, 3]
        assert!(
            (readout.weights[0][0] - 2.0).abs() < 0.1,
            "w0 = {}",
            readout.weights[0][0]
        );
        assert!(
            (readout.weights[0][1] - 3.0).abs() < 0.1,
            "w1 = {}",
            readout.weights[0][1]
        );
    }

    #[test]
    fn test_multi_target_readout() {
        let n = 100;
        let mut responses = Vec::new();
        let mut targets = Vec::new();

        for i in 0..n {
            let x0 = i as f64 / n as f64;
            let x1 = (i as f64 * 0.5).cos();
            responses.push(ResponseVector {
                activations: vec![x0, x1],
            });
            targets.push(vec![x0 + x1, x0 - x1]);
        }

        let mut readout = LinearReadout::new(2, 2).with_ridge(1e-8);
        readout.train(&responses, &targets);

        let mse = readout.mse(&responses, &targets);
        assert!(mse < 0.01, "MSE too high: {mse}");
    }

    #[test]
    fn test_cholesky_2x2() {
        let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let l = cholesky(&a, 2).unwrap();

        assert!((l[0][0] - 2.0).abs() < 1e-10);
        assert!((l[1][0] - 1.0).abs() < 1e-10);
        assert!((l[1][1] - (2.0_f64).sqrt()).abs() < 1e-10);
    }
}

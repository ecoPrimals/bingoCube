// SPDX-License-Identifier: AGPL-3.0-or-later
//! Wire types for crypto and reservoir IPC methods.

use bingocube_core::{BingoCube, Config, SubCube};
use bingocube_nautilus::{InstanceId, NautilusShell, ReservoirInput, ShellConfig};
use serde::{Deserialize, Serialize};

use crate::error::IpcError;

/// Named preset for [`Config`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigPreset {
    /// Small / default bingo configuration.
    Small,
    /// Medium grid configuration.
    Medium,
    /// Large grid configuration.
    Large,
}

impl ConfigPreset {
    /// Materialize the preset into a [`Config`].
    #[must_use]
    pub fn into_config(self) -> Config {
        match self {
            Self::Small => Config::default(),
            Self::Medium => Config::medium(),
            Self::Large => Config::large(),
        }
    }
}

/// Board configuration accepted by crypto methods (preset name or full object).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigParam {
    /// Named preset (`medium`, `large`, etc.).
    Preset(ConfigPreset),
    /// Explicit configuration object.
    Full(Config),
}

impl ConfigParam {
    /// Resolve to a validated [`Config`].
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration is invalid.
    pub fn into_config(self) -> Result<Config, IpcError> {
        let config = match self {
            Self::Preset(preset) => preset.into_config(),
            Self::Full(config) => config,
        };
        config
            .validate()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?;
        Ok(config)
    }
}

/// Parameters for `crypto.commit`.
#[derive(Debug, Clone, Deserialize)]
pub struct CryptoCommitParams {
    /// Seed bytes (UTF-8 string or byte array).
    pub seed: SeedParam,
    /// Optional board configuration (defaults to small).
    #[serde(default)]
    pub config: Option<ConfigParam>,
}

/// Parameters for `crypto.reveal`.
#[derive(Debug, Clone, Deserialize)]
pub struct CryptoRevealParams {
    /// Seed bytes.
    pub seed: SeedParam,
    /// Reveal level x ∈ (0, 1].
    pub x: f64,
    /// Optional board configuration.
    #[serde(default)]
    pub config: Option<ConfigParam>,
}

/// Parameters for `crypto.verify`.
#[derive(Debug, Clone, Deserialize)]
pub struct CryptoVerifyParams {
    /// Seed bytes.
    pub seed: SeedParam,
    /// Reveal level x ∈ (0, 1].
    pub x: f64,
    /// Subcube payload.
    pub subcube: SubCubeWire,
    /// Optional board configuration.
    #[serde(default)]
    pub config: Option<ConfigParam>,
}

/// Seed accepted as UTF-8 string or JSON byte array.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SeedParam {
    /// UTF-8 seed string.
    Text(String),
    /// Raw seed bytes.
    Bytes(Vec<u8>),
}

impl SeedParam {
    /// Resolve seed to byte slice owner.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Text(s) => s.into_bytes(),
            Self::Bytes(b) => b,
        }
    }
}

/// JSON-serializable subcube representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubCubeWire {
    /// Grid dimension.
    pub size: usize,
    /// Reveal parameter.
    pub x: f64,
    /// Revealed cells as `[row, col, color]` triples.
    pub revealed: Vec<[u32; 3]>,
}

impl SubCubeWire {
    /// Convert from a core [`SubCube`].
    #[must_use]
    pub fn from_subcube(subcube: &SubCube) -> Self {
        let mut revealed: Vec<[u32; 3]> = subcube
            .revealed
            .iter()
            .map(|(&(row, col), &color)| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "grid indices are bounded by small bingo dimensions"
                )]
                let row_u = row as u32;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "grid indices are bounded by small bingo dimensions"
                )]
                let col_u = col as u32;
                [row_u, col_u, u32::from(color)]
            })
            .collect();
        revealed.sort_by_key(|cell| (cell[0], cell[1]));
        Self {
            size: subcube.size,
            x: subcube.x,
            revealed,
        }
    }

    /// Convert into a core [`SubCube`].
    ///
    /// # Errors
    ///
    /// Returns an error when cell coordinates cannot fit in `usize`.
    pub fn into_subcube(self) -> Result<SubCube, IpcError> {
        use std::collections::HashMap;

        let mut revealed = HashMap::new();
        for cell in self.revealed {
            let row = usize::try_from(cell[0])
                .map_err(|_| IpcError::invalid_params("invalid row index"))?;
            let col = usize::try_from(cell[1])
                .map_err(|_| IpcError::invalid_params("invalid column index"))?;
            let color = u8::try_from(cell[2])
                .map_err(|_| IpcError::invalid_params("invalid color value"))?;
            revealed.insert((row, col), color);
        }
        Ok(SubCube {
            size: self.size,
            revealed,
            x: self.x,
        })
    }
}

/// Parameters for `reservoir.create`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirCreateParams {
    /// Optional shell configuration (defaults applied when omitted).
    #[serde(default)]
    pub config: Option<ShellConfig>,
    /// Optional instance name for lineage tracking.
    #[serde(default)]
    pub instance_name: Option<String>,
}

/// Parameters for `reservoir.evolve`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservoirEvolveParams {
    /// Training inputs.
    pub inputs: Vec<ReservoirInput>,
    /// Training targets (one vector per input).
    pub targets: Vec<Vec<f64>>,
}

/// Parameters for `reservoir.predict`.
#[derive(Debug, Clone, Deserialize)]
pub struct ReservoirPredictParams {
    /// Input sample.
    pub input: ReservoirInput,
}

/// Resolve configuration from optional param, defaulting to small preset.
fn resolve_config(config: Option<ConfigParam>) -> Result<Config, IpcError> {
    match config {
        Some(param) => param.into_config(),
        None => {
            let cfg = Config::default();
            cfg.validate()
                .map_err(|e| IpcError::invalid_params(e.to_string()))?;
            Ok(cfg)
        }
    }
}

/// Generate a [`BingoCube`] from seed + optional config param.
///
/// # Errors
///
/// Returns an error when configuration or generation fails.
pub fn cube_from_params(
    seed: SeedParam,
    config: Option<ConfigParam>,
) -> Result<BingoCube, IpcError> {
    let seed_bytes = seed.into_bytes();
    let config = resolve_config(config)?;
    BingoCube::from_seed(&seed_bytes, config).map_err(|e| IpcError::handler(e.to_string()))
}

/// Compute BLAKE3 commitment hash over the color grid.
#[must_use]
pub fn commitment_hash(cube: &BingoCube) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"BINGOCUBE_COMMITMENT_V1");
    for row in cube.color_grid() {
        for &color in row {
            hasher.update(&[color]);
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// Create a new [`NautilusShell`] from IPC parameters.
///
/// # Errors
///
/// Returns an error when shell initialization fails.
pub(crate) fn create_shell(
    params: ReservoirCreateParams,
    instance_id: &InstanceId,
) -> Result<NautilusShell, IpcError> {
    let config = params.config.unwrap_or_default();
    NautilusShell::new(config, instance_id.clone()).map_err(|e| IpcError::handler(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_core::BingoCube;

    #[test]
    fn subcube_wire_roundtrip() {
        let cube = BingoCube::from_seed(b"wire-test", Config::default()).expect("cube");
        let sub = cube.subcube(0.5).expect("subcube");
        let wire = SubCubeWire::from_subcube(&sub);
        let back = wire.into_subcube().expect("back");
        assert_eq!(sub, back);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Domain handlers for IPC methods.

use bingocube_nautilus::InstanceId;
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::IpcError;
use crate::socket::PRIMAL_NAME;
use crate::state::ServerState;
use crate::types::{
    CryptoCommitParams, CryptoRevealParams, CryptoVerifyParams, ReservoirCreateParams,
    ReservoirEvolveParams, ReservoirPredictParams, SubCubeWire, commitment_hash, create_shell,
    cube_from_params,
};

/// Package version exposed over IPC.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// All JSON-RPC methods implemented by this primal.
pub const METHODS: &[&str] = &[
    "capabilities.list",
    "health.liveness",
    "health.check",
    "identity.get",
    "crypto.commit",
    "crypto.reveal",
    "crypto.verify",
    "reservoir.create",
    "reservoir.evolve",
    "reservoir.predict",
];

/// Response for `capabilities.list`.
#[derive(Debug, Serialize)]
pub(crate) struct CapabilityListResponse {
    /// Primal name.
    pub primal: &'static str,
    /// Crate version.
    pub version: &'static str,
    /// Supported method names.
    pub methods: &'static [&'static str],
}

/// Response for `health.liveness`.
#[derive(Debug, Serialize)]
pub(crate) struct LivenessResponse {
    /// Liveness status.
    pub status: &'static str,
}

/// Response for `health.check`.
#[derive(Debug, Serialize)]
pub(crate) struct HealthCheckResponse {
    /// Overall health status.
    pub status: &'static str,
    /// Per-component status map.
    pub components: HealthComponents,
}

/// Component health map.
#[derive(Debug, Serialize)]
pub(crate) struct HealthComponents {
    /// Core `BingoCube` engine status.
    pub core: &'static str,
    /// Nautilus reservoir status.
    pub nautilus: &'static str,
}

/// Response for `identity.get`.
#[derive(Debug, Serialize)]
pub(crate) struct IdentityResponse {
    /// Primal name.
    pub primal: &'static str,
    /// Instance identifier.
    pub instance_id: String,
}

/// Build `capabilities.list` result.
#[must_use]
pub(crate) fn handle_capabilities_list() -> CapabilityListResponse {
    CapabilityListResponse {
        primal: PRIMAL_NAME,
        version: VERSION,
        methods: METHODS,
    }
}

/// Build `health.liveness` result.
#[must_use]
pub(crate) fn handle_health_liveness() -> LivenessResponse {
    LivenessResponse { status: "alive" }
}

/// Build `health.check` result.
#[must_use]
pub(crate) fn handle_health_check() -> HealthCheckResponse {
    HealthCheckResponse {
        status: "healthy",
        components: HealthComponents {
            core: "ok",
            nautilus: "ok",
        },
    }
}

/// Build `identity.get` result.
#[must_use]
pub(crate) fn handle_identity_get(instance_id: &InstanceId) -> IdentityResponse {
    IdentityResponse {
        primal: PRIMAL_NAME,
        instance_id: instance_id.0.clone(),
    }
}

/// Handle `crypto.commit`.
///
/// # Errors
///
/// Returns an error when cube generation fails.
pub(crate) fn handle_crypto_commit(params: CryptoCommitParams) -> Result<Value, IpcError> {
    let cube = cube_from_params(params.seed, params.config)?;
    Ok(json!({
        "commitment_hash": commitment_hash(&cube),
        "grid_size": cube.config.grid_size,
    }))
}

/// Handle `crypto.reveal`.
///
/// # Errors
///
/// Returns an error when cube generation or reveal fails.
pub(crate) fn handle_crypto_reveal(params: CryptoRevealParams) -> Result<Value, IpcError> {
    let cube = cube_from_params(params.seed, params.config)?;
    let subcube = cube
        .subcube(params.x)
        .map_err(|e| IpcError::handler(e.to_string()))?;
    let wire = SubCubeWire::from_subcube(&subcube);
    serde_json::to_value(wire).map_err(IpcError::from)
}

/// Handle `crypto.verify`.
///
/// # Errors
///
/// Returns an error when parameters are invalid or verification cannot run.
pub(crate) fn handle_crypto_verify(params: CryptoVerifyParams) -> Result<Value, IpcError> {
    let cube = cube_from_params(params.seed, params.config)?;
    let subcube = params.subcube.into_subcube()?;
    let valid = cube.verify_subcube(&subcube, params.x);
    Ok(json!({ "valid": valid }))
}

/// Handle `reservoir.create`, mutating server state.
///
/// # Errors
///
/// Returns an error when shell creation fails.
pub(crate) fn handle_reservoir_create(
    state: &mut ServerState,
    params: ReservoirCreateParams,
) -> Result<Value, IpcError> {
    let instance_id = match params.instance_name.as_deref() {
        Some(name) => InstanceId::new(name),
        None => state.instance_id.clone(),
    };
    let shell = create_shell(params, &instance_id)?;
    let generation = shell.generation();
    state.set_reservoir(shell);
    Ok(json!({
        "status": "created",
        "generation": generation,
        "instance_id": instance_id.0,
    }))
}

/// Handle `reservoir.evolve`.
///
/// # Errors
///
/// Returns an error when no shell exists or evolution fails.
pub(crate) fn handle_reservoir_evolve(
    state: &mut ServerState,
    params: ReservoirEvolveParams,
) -> Result<Value, IpcError> {
    if params.inputs.len() != params.targets.len() {
        return Err(IpcError::invalid_params(
            "inputs and targets must have the same length",
        ));
    }
    let shell = state.reservoir_mut()?;
    let mse = shell
        .evolve_generation(&params.inputs, &params.targets)
        .map_err(|e| IpcError::handler(e.to_string()))?;
    Ok(json!({
        "mse": mse,
        "generation": shell.generation(),
    }))
}

/// Handle `reservoir.predict`.
///
/// # Errors
///
/// Returns an error when no shell exists.
pub(crate) fn handle_reservoir_predict(
    state: &ServerState,
    params: ReservoirPredictParams,
) -> Result<Value, IpcError> {
    let shell = state.reservoir()?;
    let prediction = shell.predict(&params.input);
    Ok(json!({ "prediction": prediction }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConfigPreset, SeedParam};

    #[test]
    fn crypto_commit_is_deterministic() {
        let params = CryptoCommitParams {
            seed: SeedParam::Text("test-seed".to_owned()),
            config: Some(crate::types::ConfigParam::Preset(ConfigPreset::Small)),
        };
        let a = handle_crypto_commit(params.clone()).expect("commit");
        let b = handle_crypto_commit(params).expect("commit");
        assert_eq!(a["commitment_hash"], b["commitment_hash"]);
    }
}

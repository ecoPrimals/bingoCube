// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC method dispatch.

use serde_json::Value;

use crate::error::IpcError;
use crate::service::{
    handle_capabilities_list, handle_crypto_commit, handle_crypto_reveal, handle_crypto_verify,
    handle_health_check, handle_health_liveness, handle_identity_get, handle_reservoir_create,
    handle_reservoir_evolve, handle_reservoir_predict,
};
use crate::state::ServerState;
use crate::types::{
    CryptoCommitParams, CryptoRevealParams, CryptoVerifyParams, ReservoirCreateParams,
    ReservoirEvolveParams, ReservoirPredictParams,
};

/// Extract typed parameters from a JSON-RPC `params` value.
///
/// Accepts either a JSON object or a single-element array (positional style).
fn extract_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, IpcError> {
    if params.is_null() {
        return Err(IpcError::invalid_params("missing params"));
    }
    if let Some(arr) = params.as_array() {
        if arr.len() != 1 {
            return Err(IpcError::invalid_params(
                "positional params must be a single-element array",
            ));
        }
        serde_json::from_value(arr[0].clone()).map_err(|e| IpcError::invalid_params(e.to_string()))
    } else {
        serde_json::from_value(params).map_err(|e| IpcError::invalid_params(e.to_string()))
    }
}

/// Route a JSON-RPC method to its handler.
///
/// # Errors
///
/// Returns an error for unknown methods, invalid params, or handler failures.
pub fn dispatch(method: &str, params: Value, state: &mut ServerState) -> Result<Value, IpcError> {
    match method {
        "capabilities.list" => serde_json::to_value(handle_capabilities_list())
            .map_err(|e| IpcError::Internal(e.to_string())),
        "health.liveness" => serde_json::to_value(handle_health_liveness())
            .map_err(|e| IpcError::Internal(e.to_string())),
        "health.check" => serde_json::to_value(handle_health_check())
            .map_err(|e| IpcError::Internal(e.to_string())),
        "identity.get" => serde_json::to_value(handle_identity_get(&state.instance_id))
            .map_err(|e| IpcError::Internal(e.to_string())),
        "crypto.commit" => {
            let p: CryptoCommitParams = extract_params(params)?;
            handle_crypto_commit(p)
        }
        "crypto.reveal" => {
            let p: CryptoRevealParams = extract_params(params)?;
            handle_crypto_reveal(p)
        }
        "crypto.verify" => {
            let p: CryptoVerifyParams = extract_params(params)?;
            handle_crypto_verify(p)
        }
        "reservoir.create" => {
            let p: ReservoirCreateParams = extract_params(params)?;
            handle_reservoir_create(state, p)
        }
        "reservoir.evolve" => {
            let p: ReservoirEvolveParams = extract_params(params)?;
            handle_reservoir_evolve(state, p)
        }
        "reservoir.predict" => {
            let p: ReservoirPredictParams = extract_params(params)?;
            handle_reservoir_predict(state, p)
        }
        other => Err(IpcError::MethodNotFound(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingocube_nautilus::InstanceId;
    use serde_json::json;

    fn test_state() -> ServerState {
        ServerState::new(InstanceId::new("test"))
    }

    #[test]
    fn dispatches_capabilities_list() {
        let mut state = test_state();
        let result = dispatch("capabilities.list", json!({}), &mut state).expect("dispatch");
        assert_eq!(result["primal"], "bingoCube");
        assert!(result["methods"].is_array());
    }

    #[test]
    fn unknown_method_returns_error() {
        let mut state = test_state();
        let err = dispatch("unknown.method", json!({}), &mut state).expect_err("err");
        assert!(matches!(err, IpcError::MethodNotFound(_)));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! tarpc 0.37 binary service (bincode framing) on a `.tarpc.sock` sibling socket.

use std::path::Path;

use bingocube_nautilus::ReservoirInput;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tarpc::server::{self, Channel};
use tarpc::tokio_serde::formats::Bincode;
use tokio::sync::watch;

use crate::error::IpcError;
use crate::service::{
    handle_capabilities_list, handle_crypto_commit, handle_crypto_reveal, handle_crypto_verify,
    handle_health_check, handle_health_liveness, handle_identity_get, handle_reservoir_create,
    handle_reservoir_evolve, handle_reservoir_predict,
};
use crate::socket::prepare_socket_path;
use crate::state::SharedState;
use crate::types::{
    ConfigParam, CryptoCommitParams, CryptoRevealParams, CryptoVerifyParams, ReservoirCreateParams,
    ReservoirEvolveParams, ReservoirPredictParams, SeedParam, SubCubeWire,
};

/// tarpc-side error type (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcError {
    /// Error message.
    pub message: String,
}

impl From<IpcError> for TarpcError {
    fn from(value: IpcError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<serde_json::Error> for TarpcError {
    fn from(value: serde_json::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

/// Capability list response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcCapabilityList {
    /// Primal name.
    pub primal: String,
    /// Version string.
    pub version: String,
    /// Method names.
    pub methods: Vec<String>,
}

/// Liveness response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcLiveness {
    /// Status string.
    pub status: String,
}

/// Health check response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcHealthCheck {
    /// Overall status.
    pub status: String,
    /// Component map.
    pub components: TarpcComponents,
}

/// Component health for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcComponents {
    /// Core engine status.
    pub core: String,
    /// Nautilus status.
    pub nautilus: String,
}

/// Identity response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcIdentity {
    /// Primal name.
    pub primal: String,
    /// Instance id.
    pub instance_id: String,
}

/// Commit response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcCommitResponse {
    /// Commitment hash hex.
    pub commitment_hash: String,
    /// Grid size.
    pub grid_size: usize,
}

/// Reveal response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcRevealResponse {
    /// Subcube wire payload.
    pub subcube: SubCubeWire,
}

/// Verify response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcVerifyResponse {
    /// Whether verification succeeded.
    pub valid: bool,
}

/// Reservoir create response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcReservoirCreate {
    /// Status string.
    pub status: String,
    /// Current generation.
    pub generation: usize,
    /// Instance id.
    pub instance_id: String,
}

/// Reservoir evolve response for tarpc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TarpcReservoirEvolve {
    /// Mean squared error after training.
    pub mse: f64,
    /// Generation after evolution.
    pub generation: usize,
}

/// tarpc service — binary counterpart to JSON-RPC methods.
#[tarpc::service]
pub trait BingoCubeTarpc {
    /// Wire Standard L2 capability inventory (`capabilities.list`).
    async fn capability_list() -> TarpcCapabilityList;

    /// Lightweight alive probe (`health.liveness`).
    async fn health_liveness() -> TarpcLiveness;

    /// Full health probe (`health.check`).
    async fn health_check() -> TarpcHealthCheck;

    /// Self-description (`identity.get`).
    async fn identity_get() -> TarpcIdentity;

    /// Generate commitment (`crypto.commit`).
    async fn crypto_commit(
        seed: Vec<u8>,
        config: Option<ConfigParam>,
    ) -> Result<TarpcCommitResponse, TarpcError>;

    /// Progressive reveal (`crypto.reveal`).
    async fn crypto_reveal(
        seed: Vec<u8>,
        x: f64,
        config: Option<ConfigParam>,
    ) -> Result<TarpcRevealResponse, TarpcError>;

    /// Verify subcube (`crypto.verify`).
    async fn crypto_verify(
        seed: Vec<u8>,
        x: f64,
        subcube: SubCubeWire,
        config: Option<ConfigParam>,
    ) -> Result<TarpcVerifyResponse, TarpcError>;

    /// Create nautilus shell (`reservoir.create`).
    async fn reservoir_create(
        params: ReservoirCreateParams,
    ) -> Result<TarpcReservoirCreate, TarpcError>;

    /// Evolve one generation (`reservoir.evolve`).
    async fn reservoir_evolve(
        params: ReservoirEvolveParams,
    ) -> Result<TarpcReservoirEvolve, TarpcError>;

    /// Predict from trained shell (`reservoir.predict`).
    async fn reservoir_predict(input: ReservoirInput) -> Result<Vec<f64>, TarpcError>;
}

/// Handler bridging tarpc calls to shared server state.
#[derive(Clone)]
struct TarpcHandler {
    state: SharedState,
}

impl BingoCubeTarpc for TarpcHandler {
    async fn capability_list(self, _: tarpc::context::Context) -> TarpcCapabilityList {
        let caps = handle_capabilities_list();
        TarpcCapabilityList {
            primal: caps.primal.to_owned(),
            version: caps.version.to_owned(),
            methods: caps.methods.iter().map(|m| (*m).to_owned()).collect(),
        }
    }

    async fn health_liveness(self, _: tarpc::context::Context) -> TarpcLiveness {
        TarpcLiveness {
            status: handle_health_liveness().status.to_owned(),
        }
    }

    async fn health_check(self, _: tarpc::context::Context) -> TarpcHealthCheck {
        let h = handle_health_check();
        TarpcHealthCheck {
            status: h.status.to_owned(),
            components: TarpcComponents {
                core: h.components.core.to_owned(),
                nautilus: h.components.nautilus.to_owned(),
            },
        }
    }

    async fn identity_get(self, _: tarpc::context::Context) -> TarpcIdentity {
        let guard = self.state.read().await;
        let id = handle_identity_get(&guard.instance_id);
        TarpcIdentity {
            primal: id.primal.to_owned(),
            instance_id: id.instance_id,
        }
    }

    async fn crypto_commit(
        self,
        _: tarpc::context::Context,
        seed: Vec<u8>,
        config: Option<ConfigParam>,
    ) -> Result<TarpcCommitResponse, TarpcError> {
        let value = handle_crypto_commit(CryptoCommitParams {
            seed: SeedParam::Bytes(seed),
            config,
        })?;
        Ok(TarpcCommitResponse {
            commitment_hash: value["commitment_hash"]
                .as_str()
                .ok_or_else(|| TarpcError {
                    message: "missing commitment_hash".to_owned(),
                })?
                .to_owned(),
            grid_size: value["grid_size"]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(|| TarpcError {
                    message: "missing grid_size".to_owned(),
                })?,
        })
    }

    async fn crypto_reveal(
        self,
        _: tarpc::context::Context,
        seed: Vec<u8>,
        x: f64,
        config: Option<ConfigParam>,
    ) -> Result<TarpcRevealResponse, TarpcError> {
        let value = handle_crypto_reveal(CryptoRevealParams {
            seed: SeedParam::Bytes(seed),
            x,
            config,
        })?;
        let subcube: SubCubeWire = serde_json::from_value(value)?;
        Ok(TarpcRevealResponse { subcube })
    }

    async fn crypto_verify(
        self,
        _: tarpc::context::Context,
        seed: Vec<u8>,
        x: f64,
        subcube: SubCubeWire,
        config: Option<ConfigParam>,
    ) -> Result<TarpcVerifyResponse, TarpcError> {
        let value = handle_crypto_verify(CryptoVerifyParams {
            seed: SeedParam::Bytes(seed),
            x,
            subcube,
            config,
        })?;
        let valid = value["valid"].as_bool().ok_or_else(|| TarpcError {
            message: "missing valid field".to_owned(),
        })?;
        Ok(TarpcVerifyResponse { valid })
    }

    async fn reservoir_create(
        self,
        _: tarpc::context::Context,
        params: ReservoirCreateParams,
    ) -> Result<TarpcReservoirCreate, TarpcError> {
        let mut guard = self.state.write().await;
        let value = handle_reservoir_create(&mut guard, params)?;
        let status = value["status"]
            .as_str()
            .ok_or_else(|| TarpcError {
                message: "missing status field".to_owned(),
            })?
            .to_owned();
        let generation = value["generation"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| TarpcError {
                message: "missing generation field".to_owned(),
            })?;
        let instance_id = value["instance_id"]
            .as_str()
            .ok_or_else(|| TarpcError {
                message: "missing instance_id field".to_owned(),
            })?
            .to_owned();
        Ok(TarpcReservoirCreate {
            status,
            generation,
            instance_id,
        })
    }

    async fn reservoir_evolve(
        self,
        _: tarpc::context::Context,
        params: ReservoirEvolveParams,
    ) -> Result<TarpcReservoirEvolve, TarpcError> {
        let mut guard = self.state.write().await;
        let value = handle_reservoir_evolve(&mut guard, params)?;
        let mse = value["mse"].as_f64().ok_or_else(|| TarpcError {
            message: "missing mse field".to_owned(),
        })?;
        let generation = value["generation"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| TarpcError {
                message: "missing generation field".to_owned(),
            })?;
        Ok(TarpcReservoirEvolve { mse, generation })
    }

    async fn reservoir_predict(
        self,
        _: tarpc::context::Context,
        input: ReservoirInput,
    ) -> Result<Vec<f64>, TarpcError> {
        let guard = self.state.read().await;
        let value = handle_reservoir_predict(&guard, ReservoirPredictParams { input })?;
        value["prediction"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_f64)
                    .collect::<Vec<f64>>()
            })
            .ok_or_else(|| TarpcError {
                message: "missing prediction array".to_owned(),
            })
    }
}

/// Start the tarpc Unix socket server (non-blocking spawn).
///
/// C2 dual-socket pattern — UDS only. G65 negotiation handles tarpc on
/// any transport via the unified listener.
///
/// # Errors
///
/// Returns an error when the socket cannot be prepared or bound.
#[cfg(unix)]
pub(crate) fn serve_tarpc(
    path: &Path,
    state: SharedState,
    shutdown_rx: watch::Receiver<()>,
) -> Result<(), IpcError> {
    prepare_socket_path(path)?;
    let path = path.to_path_buf();
    let handler = TarpcHandler { state };

    tokio::spawn(async move {
        let listener = match tarpc::serde_transport::unix::listen(&path, Bincode::default).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "tarpc bind failed");
                return;
            }
        };

        tracing::info!(path = %path.display(), "tarpc listening");

        let mut shutdown_rx = shutdown_rx;
        let incoming = listener.filter_map(|result| async { result.ok() });

        tokio::select! {
            () = incoming.for_each(|transport| {
                let handler = handler.clone();
                async move {
                    tokio::spawn(async move {
                        server::BaseChannel::with_defaults(transport)
                            .execute(handler.serve())
                            .for_each(|response| async move {
                                response.await;
                            })
                            .await;
                    });
                }
            }) => {}
            Ok(()) = shutdown_rx.changed() => {
                tracing::info!("tarpc server shutting down");
            }
        }

        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    });

    Ok(())
}

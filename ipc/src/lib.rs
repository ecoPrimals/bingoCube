// SPDX-License-Identifier: AGPL-3.0-or-later
//! # bingocube-ipc
//!
//! JSON-RPC 2.0 and tarpc IPC for the `BingoCube` primal.
//!
//! Supports two modes:
//! - **C2 dual-socket** (default): `.sock` (JSON-RPC) + `.tarpc.sock` (tarpc)
//! - **G65 single-socket** (`--negotiate`): one socket, protocol negotiation at connect time

#![warn(missing_docs)]

mod dispatch;
mod error;
mod negotiation;
mod protocol;
mod server;
mod service;
mod socket;
mod state;
mod types;

#[cfg(feature = "tarpc")]
mod tarpc;

pub use dispatch::dispatch;
pub use error::IpcError;
pub use negotiation::{
    IpcProtocol, NegotiationError, NegotiationRequest, NegotiationResponse,
    ServerNegotiationOutcome, negotiate_client, negotiate_server, negotiate_server_outcome,
    select_protocol,
};
pub use protocol::{JSONRPC_VERSION, JsonRpcRequest, JsonRpcResponse};
pub use server::{BoundEndpoints, ServeConfig, serve, wait_for_shutdown};
pub use service::{METHODS, VERSION};
pub use socket::{
    JSONRPC_SOCKET_NAME, PRIMAL_NAME, TARPC_SOCKET_NAME, default_socket_dir, jsonrpc_socket_path,
    prepare_socket_path, resolve_socket_dir, tarpc_socket_from_jsonrpc, tarpc_socket_path,
};
pub use state::{ServerState, SharedState, new_shared_state};
pub use types::{
    ConfigParam, ConfigPreset, CryptoCommitParams, CryptoRevealParams, CryptoVerifyParams,
    ReservoirCreateParams, ReservoirEvolveParams, ReservoirPredictParams, SeedParam, SubCubeWire,
    commitment_hash, cube_from_params,
};

#[cfg(feature = "tarpc")]
pub use tarpc::BingoCubeTarpc;

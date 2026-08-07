// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPC error types.

use thiserror::Error;

/// Errors surfaced by the `BingoCube` IPC layer.
#[derive(Debug, Error)]
pub enum IpcError {
    /// I/O failure (socket bind, read, write).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Domain handler failure (crypto, reservoir, etc.).
    #[error("handler error: {0}")]
    Handler(String),

    /// Invalid method parameters.
    #[error("invalid params: {0}")]
    InvalidParams(String),

    /// Unknown JSON-RPC method.
    #[error("method not found: {0}")]
    MethodNotFound(String),

    /// Internal server failure.
    #[error("internal error: {0}")]
    Internal(String),

    /// tarpc transport failure (feature-gated).
    #[cfg(feature = "tarpc")]
    #[error("tarpc error: {0}")]
    Tarpc(String),
}

impl IpcError {
    /// Map a handler failure to a JSON-RPC internal error message.
    #[must_use]
    pub fn handler(message: impl Into<String>) -> Self {
        Self::Handler(message.into())
    }

    /// Map invalid parameters.
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams(message.into())
    }
}

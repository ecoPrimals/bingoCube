// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC 2.0 wire types (implemented from scratch with `serde_json`).

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 protocol version.
pub const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC parse error code.
const PARSE_ERROR: i32 = -32_700;
/// Standard JSON-RPC invalid request code.
const INVALID_REQUEST: i32 = -32_600;
/// Standard JSON-RPC method not found code.
const METHOD_NOT_FOUND: i32 = -32_601;
/// Standard JSON-RPC invalid params code.
const INVALID_PARAMS: i32 = -32_602;
/// Standard JSON-RPC internal error code.
const INTERNAL_ERROR: i32 = -32_603;

/// Incoming JSON-RPC 2.0 request.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be `"2.0"`).
    pub jsonrpc: String,
    /// Semantic method name (`domain.operation`).
    pub method: String,
    /// Optional parameters (object or array).
    #[serde(default)]
    pub params: serde_json::Value,
    /// Correlation id (`null` for notifications).
    pub id: serde_json::Value,
}

/// Outgoing JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version.
    pub jsonrpc: &'static str,
    /// Success payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Correlation id.
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Human-readable message.
    pub message: String,
    /// Optional structured detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Build a success response.
    #[must_use]
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Build an error response.
    #[must_use]
    pub fn error(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Parse error (-32700).
    #[must_use]
    pub fn parse_error(id: serde_json::Value, detail: impl Into<String>) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: PARSE_ERROR,
                message: detail.into(),
                data: None,
            },
        )
    }

    /// Invalid request (-32600).
    #[must_use]
    pub fn invalid_request(id: serde_json::Value, detail: impl Into<String>) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: INVALID_REQUEST,
                message: detail.into(),
                data: None,
            },
        )
    }

    /// Method not found (-32601).
    #[must_use]
    pub fn method_not_found(id: serde_json::Value, method: &str) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("method not found: {method}"),
                data: None,
            },
        )
    }

    /// Invalid params (-32602).
    #[must_use]
    pub fn invalid_params(id: serde_json::Value, detail: impl Into<String>) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: INVALID_PARAMS,
                message: detail.into(),
                data: None,
            },
        )
    }

    /// Internal error (-32603).
    #[must_use]
    pub fn internal(id: serde_json::Value, detail: impl Into<String>) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: INTERNAL_ERROR,
                message: detail.into(),
                data: None,
            },
        )
    }
}

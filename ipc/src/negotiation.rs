// SPDX-License-Identifier: AGPL-3.0-or-later
//! G65 protocol negotiation — single-socket tarpc vs JSON-RPC selection.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{info, warn};

/// RPC protocol variants for G65 negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum IpcProtocol {
    /// JSON-RPC 2.0 — default, backward-compatible.
    #[default]
    JsonRpc,
    /// tarpc binary framing (bincode).
    Tarpc,
}

impl fmt::Display for IpcProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire_name())
    }
}

impl IpcProtocol {
    /// Wire name used in `PROTOCOLS:` / `PROTOCOL:` lines.
    #[must_use]
    pub const fn wire_name(&self) -> &'static str {
        match self {
            Self::JsonRpc => "jsonrpc",
            Self::Tarpc => "tarpc",
        }
    }

    /// Parse a protocol from its wire name (case-insensitive).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "jsonrpc" | "json-rpc" | "json_rpc" => Some(Self::JsonRpc),
            "tarpc" | "binary" => Some(Self::Tarpc),
            _ => None,
        }
    }

    /// All protocols supported by this build (tarpc preferred when enabled).
    #[must_use]
    pub fn all_supported() -> Vec<Self> {
        #[cfg(feature = "tarpc")]
        {
            vec![Self::Tarpc, Self::JsonRpc]
        }
        #[cfg(not(feature = "tarpc"))]
        {
            vec![Self::JsonRpc]
        }
    }
}

/// Client `PROTOCOLS:` negotiation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationRequest {
    /// Client-supported protocols in preference order.
    pub supported: Vec<IpcProtocol>,
}

impl NegotiationRequest {
    /// Create a request listing the given protocols.
    #[must_use]
    pub const fn new(supported: Vec<IpcProtocol>) -> Self {
        Self { supported }
    }

    /// Serialize to G65 wire format.
    #[must_use]
    pub fn to_wire(&self) -> String {
        let names: Vec<&str> = self.supported.iter().map(IpcProtocol::wire_name).collect();
        format!("PROTOCOLS: {}\n", names.join(","))
    }

    /// Parse from G65 wire format.
    ///
    /// # Errors
    ///
    /// Returns an error when the line is malformed or lists no recognized protocols.
    pub fn from_wire(line: &str) -> Result<Self, NegotiationError> {
        let body = line
            .trim()
            .strip_prefix("PROTOCOLS: ")
            .ok_or(NegotiationError::InvalidRequest)?;

        let supported: Vec<IpcProtocol> = body
            .split(',')
            .filter_map(|name| IpcProtocol::parse(name.trim()))
            .collect();

        if supported.is_empty() {
            return Err(NegotiationError::NoValidProtocols);
        }

        Ok(Self { supported })
    }
}

/// Server `PROTOCOL:` negotiation response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationResponse {
    /// Protocol selected by the server.
    pub selected: IpcProtocol,
}

impl NegotiationResponse {
    /// Create a response selecting the given protocol.
    #[must_use]
    pub const fn new(selected: IpcProtocol) -> Self {
        Self { selected }
    }

    /// Serialize to G65 wire format.
    #[must_use]
    pub fn to_wire(&self) -> String {
        format!("PROTOCOL: {}\n", self.selected.wire_name())
    }

    /// Parse from G65 wire format.
    ///
    /// # Errors
    ///
    /// Returns an error when the line is malformed or names an unknown protocol.
    pub fn from_wire(line: &str) -> Result<Self, NegotiationError> {
        let name = line
            .trim()
            .strip_prefix("PROTOCOL: ")
            .ok_or(NegotiationError::InvalidResponse)?;

        let selected = IpcProtocol::parse(name).ok_or(NegotiationError::UnknownProtocol)?;

        Ok(Self { selected })
    }
}

/// Errors during G65 protocol negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NegotiationError {
    /// Line does not start with `PROTOCOLS: `.
    #[error("invalid negotiation request (expected PROTOCOLS: ...)")]
    InvalidRequest,
    /// Line does not start with `PROTOCOL: `.
    #[error("invalid negotiation response (expected PROTOCOL: ...)")]
    InvalidResponse,
    /// None of the listed protocols are recognized.
    #[error("no valid protocols in request")]
    NoValidProtocols,
    /// Protocol name not recognized.
    #[error("unknown protocol name")]
    UnknownProtocol,
    /// I/O failure during negotiation.
    #[error("negotiation I/O error: {0}")]
    Io(String),
    /// Timeout waiting for a negotiation line.
    #[error("negotiation timed out")]
    Timeout,
}

/// Outcome of server-side G65 negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerNegotiationOutcome {
    /// Negotiated protocol, or `None` when assuming legacy JSON-RPC.
    pub protocol: Option<IpcProtocol>,
    /// First line consumed when falling back to legacy JSON-RPC.
    pub legacy_first_line: Option<String>,
}

/// Select the first client preference also supported by the server.
///
/// Falls back to [`IpcProtocol::JsonRpc`] when there is no intersection.
#[must_use]
pub fn select_protocol(
    client_prefs: &[IpcProtocol],
    server_supports: &[IpcProtocol],
) -> IpcProtocol {
    for proto in client_prefs {
        if server_supports.contains(proto) {
            return *proto;
        }
    }
    IpcProtocol::JsonRpc
}

/// Client-side negotiation: send `PROTOCOLS`, read `PROTOCOL`.
///
/// # Errors
///
/// Returns [`NegotiationError`] on I/O failure or malformed server response.
pub async fn negotiate_client<T>(
    transport: &mut T,
    supported: &[IpcProtocol],
) -> Result<IpcProtocol, NegotiationError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let request = NegotiationRequest::new(supported.to_vec());
    let wire = request.to_wire();

    transport
        .write_all(wire.as_bytes())
        .await
        .map_err(|e| NegotiationError::Io(e.to_string()))?;
    transport
        .flush()
        .await
        .map_err(|e| NegotiationError::Io(e.to_string()))?;

    let mut reader = BufReader::new(transport);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| NegotiationError::Io(e.to_string()))?;

    let response = NegotiationResponse::from_wire(&line)?;
    info!("G65 client negotiated: {}", response.selected);
    Ok(response.selected)
}

/// Server-side negotiation: read with timeout, respond when `PROTOCOLS` is sent.
///
/// Returns `None` on timeout or when the first line is not a `PROTOCOLS:` request
/// (legacy JSON-RPC clients).
///
/// # Errors
///
/// Returns [`NegotiationError`] on I/O failure or malformed `PROTOCOLS` request.
pub async fn negotiate_server<T>(
    transport: &mut T,
    server_supported: &[IpcProtocol],
    timeout_ms: u64,
) -> Result<Option<IpcProtocol>, NegotiationError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    Ok(
        negotiate_server_outcome(transport, server_supported, timeout_ms)
            .await?
            .protocol,
    )
}

/// Server-side negotiation including any legacy first line consumed from the stream.
///
/// # Errors
///
/// Returns [`NegotiationError`] on I/O failure or malformed `PROTOCOLS` request.
pub async fn negotiate_server_outcome<T>(
    transport: &mut T,
    server_supported: &[IpcProtocol],
    timeout_ms: u64,
) -> Result<ServerNegotiationOutcome, NegotiationError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(transport);
    let mut line = String::new();

    let read_result = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        reader.read_line(&mut line),
    )
    .await;

    match read_result {
        Ok(Ok(n)) if n > 0 => {
            if line.trim().starts_with("PROTOCOLS: ") {
                let request = NegotiationRequest::from_wire(&line)?;
                let selected = select_protocol(&request.supported, server_supported);
                let response = NegotiationResponse::new(selected);

                reader
                    .get_mut()
                    .write_all(response.to_wire().as_bytes())
                    .await
                    .map_err(|e| NegotiationError::Io(e.to_string()))?;
                reader
                    .get_mut()
                    .flush()
                    .await
                    .map_err(|e| NegotiationError::Io(e.to_string()))?;

                info!("G65 server negotiated: {selected}");
                Ok(ServerNegotiationOutcome {
                    protocol: Some(selected),
                    legacy_first_line: None,
                })
            } else {
                warn!("G65: no protocol negotiation, assuming JSON-RPC");
                Ok(legacy_jsonrpc_outcome(Some(line)))
            }
        }
        Ok(Err(e)) => {
            warn!("G65 negotiation read error: {e}");
            Ok(legacy_jsonrpc_outcome(None))
        }
        Ok(Ok(_)) | Err(_) => Ok(legacy_jsonrpc_outcome(None)),
    }
}

const fn legacy_jsonrpc_outcome(line: Option<String>) -> ServerNegotiationOutcome {
    ServerNegotiationOutcome {
        protocol: None,
        legacy_first_line: line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_protocol_display() {
        assert_eq!(IpcProtocol::JsonRpc.to_string(), "jsonrpc");
        assert_eq!(IpcProtocol::Tarpc.to_string(), "tarpc");
    }

    #[test]
    fn ipc_protocol_parse() {
        assert_eq!(IpcProtocol::parse("jsonrpc"), Some(IpcProtocol::JsonRpc));
        assert_eq!(IpcProtocol::parse("json-rpc"), Some(IpcProtocol::JsonRpc));
        assert_eq!(IpcProtocol::parse("tarpc"), Some(IpcProtocol::Tarpc));
        assert_eq!(IpcProtocol::parse("binary"), Some(IpcProtocol::Tarpc));
        assert_eq!(IpcProtocol::parse("unknown"), None);
    }

    #[test]
    fn ipc_protocol_serde_roundtrip() {
        for proto in [IpcProtocol::JsonRpc, IpcProtocol::Tarpc] {
            let json = serde_json::to_string(&proto).expect("serialize");
            let back: IpcProtocol = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(proto, back);
        }
    }

    #[test]
    fn negotiation_request_wire_roundtrip() {
        let req = NegotiationRequest::new(vec![IpcProtocol::Tarpc, IpcProtocol::JsonRpc]);
        let wire = req.to_wire();
        assert_eq!(wire, "PROTOCOLS: tarpc,jsonrpc\n");
        let parsed = NegotiationRequest::from_wire(&wire).expect("parse");
        assert_eq!(req, parsed);
    }

    #[test]
    fn negotiation_request_single_protocol() {
        let req = NegotiationRequest::new(vec![IpcProtocol::JsonRpc]);
        assert_eq!(req.to_wire(), "PROTOCOLS: jsonrpc\n");
    }

    #[test]
    fn negotiation_response_wire_roundtrip() {
        let resp = NegotiationResponse::new(IpcProtocol::Tarpc);
        let wire = resp.to_wire();
        assert_eq!(wire, "PROTOCOL: tarpc\n");
        let parsed = NegotiationResponse::from_wire(&wire).expect("parse");
        assert_eq!(resp, parsed);
    }

    #[test]
    fn select_protocol_prefers_client_order() {
        let client = [IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        let server = [IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::Tarpc);
    }

    #[test]
    fn select_protocol_server_only_jsonrpc() {
        let client = [IpcProtocol::Tarpc, IpcProtocol::JsonRpc];
        let server = [IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::JsonRpc);
    }

    #[test]
    fn select_protocol_no_intersection_falls_back() {
        let client = [IpcProtocol::Tarpc];
        let server = [IpcProtocol::JsonRpc];
        assert_eq!(select_protocol(&client, &server), IpcProtocol::JsonRpc);
    }

    #[tokio::test]
    async fn negotiate_duplex_tarpc_preferred() {
        let (mut client_stream, mut server_stream) = tokio::io::duplex(4096);

        let server_supported = IpcProtocol::all_supported();
        let server_task = tokio::spawn(async move {
            negotiate_server(&mut server_stream, &server_supported, 1000).await
        });

        let client_result = negotiate_client(
            &mut client_stream,
            &[IpcProtocol::Tarpc, IpcProtocol::JsonRpc],
        )
        .await
        .expect("client negotiate");
        assert_eq!(client_result, IpcProtocol::Tarpc);

        let server_result = server_task.await.expect("join").expect("server negotiate");
        assert_eq!(server_result, Some(IpcProtocol::Tarpc));
    }

    #[tokio::test]
    async fn negotiate_duplex_jsonrpc_only() {
        let (mut client_stream, mut server_stream) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            negotiate_server(&mut server_stream, &[IpcProtocol::JsonRpc], 1000).await
        });

        let client_result = negotiate_client(
            &mut client_stream,
            &[IpcProtocol::Tarpc, IpcProtocol::JsonRpc],
        )
        .await
        .expect("client negotiate");
        assert_eq!(client_result, IpcProtocol::JsonRpc);

        let server_result = server_task.await.expect("join").expect("server negotiate");
        assert_eq!(server_result, Some(IpcProtocol::JsonRpc));
    }

    #[tokio::test]
    async fn negotiate_server_non_protocol_line_returns_none() {
        let (mut client_stream, mut server_stream) = tokio::io::duplex(4096);

        tokio::spawn(async move {
            client_stream
                .write_all(b"{\"jsonrpc\":\"2.0\"}\n")
                .await
                .expect("write");
            client_stream.flush().await.expect("flush");
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let outcome =
            negotiate_server_outcome(&mut server_stream, &IpcProtocol::all_supported(), 200)
                .await
                .expect("negotiate");
        assert!(outcome.protocol.is_none());
        assert_eq!(
            outcome.legacy_first_line.as_deref(),
            Some("{\"jsonrpc\":\"2.0\"}\n")
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Async IPC server using G66 transport abstraction.

use std::sync::Arc;

use bingocube_nautilus::InstanceId;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::watch;

use crate::dispatch::dispatch;
use crate::error::IpcError;
use crate::negotiation::{IpcProtocol, negotiate_server_outcome};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::socket::resolve_socket_dir;
use crate::state::{SharedState, new_shared_state};
use crate::transport::{TransportEndpoint, TransportListener, TransportStream};

/// Configuration for the IPC server.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Primary listener endpoint (UDS, TCP, or injected via `TRANSPORT_ENDPOINT`).
    pub endpoint: TransportEndpoint,
    /// Enable the tarpc binary socket (C2 dual-socket pattern, UDS-only).
    pub enable_tarpc: bool,
    /// G65 single-socket protocol negotiation on the primary listener.
    pub negotiate: bool,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            endpoint: TransportEndpoint::platform_default("bingocube", &resolve_socket_dir(None)),
            enable_tarpc: true,
            negotiate: false,
        }
    }
}

/// Resolved bind addresses after server startup.
#[derive(Debug, Clone)]
pub struct BoundEndpoints {
    /// Primary listener endpoint (concrete, with resolved port).
    pub primary: TransportEndpoint,
    /// tarpc C2 endpoint (UDS-only, when enabled).
    pub tarpc: Option<TransportEndpoint>,
}

/// Wait for SIGINT or SIGTERM.
pub async fn wait_for_shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).ok();
        tokio::select! {
            res = ctrl_c => {
                if res.is_err() {
                    tracing::warn!("ctrl-c listener failed");
                }
            }
            () = async {
                if let Some(ref mut term) = sigterm {
                    term.recv().await;
                }
            } => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

/// Start the IPC server on the configured transport until shutdown.
///
/// # Errors
///
/// Returns an error when the transport cannot be bound.
pub async fn serve(config: ServeConfig) -> Result<BoundEndpoints, IpcError> {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "bingocube".to_owned());
    let instance_id = InstanceId::new(&hostname);
    let state = new_shared_state(instance_id);

    let listener = TransportListener::bind(&config.endpoint)?;
    let bound = listener.local_endpoint()?;

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let handle = spawn_listener(
        listener,
        Arc::clone(&state),
        shutdown_rx.clone(),
        config.negotiate,
    );

    let tarpc_endpoint = resolve_tarpc_endpoint(&config, &state, shutdown_rx.clone());

    tracing::info!(endpoint = %bound, "server listening");
    if config.negotiate {
        tracing::info!("G65 single-socket protocol negotiation active");
    }
    if let Some(ref tarpc) = tarpc_endpoint {
        tracing::info!(endpoint = %tarpc, "tarpc C2 server listening");
    }

    wait_for_shutdown().await;
    tracing::info!("shutdown signal received, stopping servers");
    let _ = shutdown_tx.send(());

    let _ = handle.await;

    Ok(BoundEndpoints {
        primary: bound,
        tarpc: tarpc_endpoint,
    })
}

#[allow(unused_variables)]
fn resolve_tarpc_endpoint(
    config: &ServeConfig,
    state: &SharedState,
    shutdown_rx: watch::Receiver<()>,
) -> Option<TransportEndpoint> {
    if config.negotiate || !config.enable_tarpc {
        return None;
    }

    #[cfg(all(unix, feature = "tarpc"))]
    {
        if let TransportEndpoint::Uds { ref path } = config.endpoint {
            let tarpc_path = crate::socket::tarpc_socket_from_jsonrpc(std::path::Path::new(path));
            match crate::tarpc::serve_tarpc(&tarpc_path, Arc::clone(state), shutdown_rx) {
                Ok(()) => {
                    return Some(TransportEndpoint::Uds {
                        path: tarpc_path.to_string_lossy().into_owned(),
                    });
                }
                Err(e) => {
                    tracing::warn!(error = %e, "tarpc C2 startup failed");
                }
            }
        } else {
            tracing::debug!("tarpc C2 dual-socket requires UDS primary endpoint");
        }
    }

    None
}

fn spawn_listener(
    listener: TransportListener,
    state: SharedState,
    mut shutdown_rx: watch::Receiver<()>,
    negotiate: bool,
) -> tokio::task::JoinHandle<()> {
    let server_supported = IpcProtocol::all_supported();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok(stream) => {
                            let state = Arc::clone(&state);
                            let supported = server_supported.clone();
                            tokio::spawn(async move {
                                if negotiate {
                                    handle_negotiated_connection(stream, state, &supported).await;
                                } else if let Err(e) = handle_connection(stream, state, None).await {
                                    tracing::debug!(error = %e, "connection closed with error");
                                }
                            });
                        }
                        Err(e) => tracing::warn!(error = %e, "accept failed"),
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() {
                        break;
                    }
                }
            }
        }
        listener.cleanup();
    })
}

async fn handle_negotiated_connection(
    mut stream: TransportStream,
    state: SharedState,
    server_supported: &[IpcProtocol],
) {
    match negotiate_server_outcome(&mut stream, server_supported, 100).await {
        Ok(outcome) => match outcome.protocol {
            Some(IpcProtocol::Tarpc) => {
                tracing::info!(
                    "G65 negotiated tarpc (stub: full transport wrapping in convergence)"
                );
            }
            Some(IpcProtocol::JsonRpc) | None => {
                if let Err(e) = handle_connection(stream, state, outcome.legacy_first_line).await {
                    tracing::debug!(error = %e, "connection closed with error");
                }
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "G65 negotiation failed, falling back to JSON-RPC");
            if let Err(conn_err) = handle_connection(stream, state, None).await {
                tracing::debug!(error = %conn_err, "connection closed with error");
            }
        }
    }
}

async fn handle_connection(
    stream: TransportStream,
    state: SharedState,
    initial_line: Option<String>,
) -> Result<(), IpcError> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    if let Some(line) = initial_line {
        if !line.trim().is_empty() {
            let response = process_line(&line, &state).await;
            let mut out = serde_json::to_string(&response)?;
            out.push('\n');
            writer.write_all(out.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = process_line(&line, &state).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes()).await?;
        writer.flush().await?;
    }
    Ok(())
}

async fn process_line(line: &str, state: &SharedState) -> JsonRpcResponse {
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return JsonRpcResponse::parse_error(serde_json::Value::Null, e.to_string());
        }
    };

    if request.jsonrpc != crate::protocol::JSONRPC_VERSION {
        return JsonRpcResponse::invalid_request(
            request.id,
            format!("unsupported jsonrpc version: {}", request.jsonrpc),
        );
    }

    let mut guard = state.write().await;
    match dispatch(&request.method, request.params, &mut guard) {
        Ok(result) => JsonRpcResponse::success(request.id, result),
        Err(IpcError::MethodNotFound(method)) => {
            JsonRpcResponse::method_not_found(request.id, &method)
        }
        Err(IpcError::InvalidParams(msg) | IpcError::Handler(msg)) => {
            JsonRpcResponse::invalid_params(request.id, msg)
        }
        Err(e) => JsonRpcResponse::internal(request.id, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ServerState;
    use crate::transport::connect_transport;
    use bingocube_nautilus::InstanceId;
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::RwLock;

    async fn start_test_server(
        endpoint: &TransportEndpoint,
        negotiate: bool,
    ) -> (TransportEndpoint, watch::Sender<()>) {
        let listener = TransportListener::bind(endpoint).expect("bind");
        let bound = listener.local_endpoint().expect("local_endpoint");
        let state = Arc::new(RwLock::new(ServerState::new(InstanceId::new("test"))));
        let (tx, rx) = watch::channel(());
        spawn_listener(listener, state, rx, negotiate);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        (bound, tx)
    }

    #[tokio::test]
    async fn tcp_jsonrpc_health_liveness() {
        let ep = TransportEndpoint::Tcp {
            host: "127.0.0.1".to_owned(),
            port: 0,
        };
        let (bound, _tx) = start_test_server(&ep, false).await;

        let stream = connect_transport(&bound).await.expect("connect");
        let (reader, mut writer) = tokio::io::split(stream);

        let req = json!({
            "jsonrpc": "2.0",
            "method": "health.liveness",
            "params": {},
            "id": 1
        });
        let mut payload = serde_json::to_string(&req).expect("json");
        payload.push('\n');
        writer.write_all(payload.as_bytes()).await.expect("write");
        writer.shutdown().await.expect("shutdown");

        let mut lines = BufReader::new(reader).lines();
        let line = lines.next_line().await.expect("read").expect("line");
        let resp: serde_json::Value = serde_json::from_str(&line).expect("parse");
        assert_eq!(resp["result"]["status"], "alive");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn uds_jsonrpc_health_liveness() {
        let dir = std::env::temp_dir().join(format!("bingocube-g66-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let socket_path = dir.join("test.sock");

        let ep = TransportEndpoint::Uds {
            path: socket_path.to_string_lossy().into_owned(),
        };
        let (bound, _tx) = start_test_server(&ep, false).await;

        let stream = connect_transport(&bound).await.expect("connect");
        let (reader, mut writer) = tokio::io::split(stream);

        let req = json!({
            "jsonrpc": "2.0",
            "method": "health.liveness",
            "params": {},
            "id": 1
        });
        let mut payload = serde_json::to_string(&req).expect("json");
        payload.push('\n');
        writer.write_all(payload.as_bytes()).await.expect("write");
        writer.shutdown().await.expect("shutdown");

        let mut lines = BufReader::new(reader).lines();
        let line = lines.next_line().await.expect("read").expect("line");
        let resp: serde_json::Value = serde_json::from_str(&line).expect("parse");
        assert_eq!(resp["result"]["status"], "alive");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

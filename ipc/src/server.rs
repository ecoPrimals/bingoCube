// SPDX-License-Identifier: AGPL-3.0-or-later
//! Async JSON-RPC server on Unix domain sockets with optional TCP fallback.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bingocube_nautilus::InstanceId;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UnixListener};
use tokio::sync::watch;

use crate::dispatch::dispatch;
use crate::error::IpcError;
use crate::negotiation::{IpcProtocol, negotiate_server_outcome};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::socket::{prepare_socket_path, resolve_socket_dir};
use crate::state::{SharedState, new_shared_state};

/// Configuration for the IPC server.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Directory for Unix socket files.
    pub socket_dir: PathBuf,
    /// Enable the tarpc binary socket (C2 dual-socket pattern).
    pub enable_tarpc: bool,
    /// G65 single-socket protocol negotiation on the JSON-RPC listener.
    pub negotiate: bool,
    /// Optional TCP fallback port (`127.0.0.1`).
    pub tcp_port: Option<u16>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            socket_dir: resolve_socket_dir(None),
            enable_tarpc: true,
            negotiate: false,
            tcp_port: None,
        }
    }
}

/// Resolved bind addresses after server startup.
#[derive(Debug, Clone)]
pub struct BoundEndpoints {
    /// JSON-RPC Unix socket path.
    pub jsonrpc_socket: PathBuf,
    /// tarpc Unix socket path (when enabled).
    pub tarpc_socket: Option<PathBuf>,
    /// TCP fallback address (when enabled).
    pub tcp_addr: Option<std::net::SocketAddr>,
}

/// Wait for SIGINT or SIGTERM (Unix).
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

/// Start JSON-RPC (and optionally tarpc) servers until shutdown.
///
/// # Errors
///
/// Returns an error when socket binding or tarpc startup fails.
pub async fn serve(config: ServeConfig) -> Result<BoundEndpoints, IpcError> {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "bingocube".to_owned());
    let instance_id = InstanceId::new(&hostname);
    let state = new_shared_state(instance_id);

    let jsonrpc_path = crate::socket::jsonrpc_socket_path(&config.socket_dir);
    prepare_socket_path(&jsonrpc_path)?;

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let jsonrpc_handle = spawn_jsonrpc_unix(
        &jsonrpc_path,
        Arc::clone(&state),
        shutdown_rx.clone(),
        config.negotiate,
    );

    let tarpc_socket = if config.negotiate {
        None
    } else if config.enable_tarpc {
        #[cfg(feature = "tarpc")]
        {
            let tarpc_path = crate::socket::tarpc_socket_path(&config.socket_dir);
            crate::tarpc::serve_tarpc(&tarpc_path, Arc::clone(&state), shutdown_rx.clone())?;
            Some(tarpc_path)
        }
        #[cfg(not(feature = "tarpc"))]
        {
            None
        }
    } else {
        None
    };

    let tcp_addr = if let Some(port) = config.tcp_port {
        Some(spawn_jsonrpc_tcp(
            port,
            Arc::clone(&state),
            shutdown_rx.clone(),
        )?)
    } else {
        None
    };

    tracing::info!(path = %jsonrpc_path.display(), "JSON-RPC server listening");
    if config.negotiate {
        tracing::info!("G65 single-socket protocol negotiation active");
    }
    if let Some(ref tarpc) = tarpc_socket {
        tracing::info!(path = %tarpc.display(), "tarpc server listening");
    }
    if let Some(addr) = tcp_addr {
        tracing::info!(%addr, "JSON-RPC TCP fallback listening");
    }

    wait_for_shutdown().await;
    tracing::info!("shutdown signal received, stopping servers");
    let _ = shutdown_tx.send(());

    let _ = jsonrpc_handle.await;
    cleanup_socket(&jsonrpc_path);
    if let Some(ref tarpc) = tarpc_socket {
        cleanup_socket(tarpc);
    }

    Ok(BoundEndpoints {
        jsonrpc_socket: jsonrpc_path,
        tarpc_socket,
        tcp_addr,
    })
}

fn cleanup_socket(path: &Path) {
    if path.exists()
        && let Err(e) = std::fs::remove_file(path)
    {
        tracing::warn!(path = %path.display(), error = %e, "failed to remove socket file");
    }
}

fn spawn_jsonrpc_unix(
    path: &Path,
    state: SharedState,
    mut shutdown_rx: watch::Receiver<()>,
    negotiate: bool,
) -> tokio::task::JoinHandle<()> {
    let path = path.to_path_buf();
    tokio::spawn(async move {
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "failed to bind unix socket");
                return;
            }
        };

        let server_supported = IpcProtocol::all_supported();

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _)) => {
                            let state = Arc::clone(&state);
                            let server_supported = server_supported.clone();
                            tokio::spawn(async move {
                                if negotiate {
                                    handle_negotiated_connection(stream, state, &server_supported).await;
                                } else if let Err(e) = handle_connection(stream, state, None).await {
                                    tracing::debug!(error = %e, "connection closed with error");
                                }
                            });
                        }
                        Err(e) => tracing::warn!(error = %e, "unix accept failed"),
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() {
                        break;
                    }
                }
            }
        }
        cleanup_socket(&path);
    })
}

async fn handle_negotiated_connection(
    mut stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    state: SharedState,
    server_supported: &[IpcProtocol],
) {
    match negotiate_server_outcome(&mut stream, server_supported, 100).await {
        Ok(outcome) => match outcome.protocol {
            Some(IpcProtocol::Tarpc) => {
                tracing::info!(
                    "G65 negotiated tarpc (stub: closing connection until transport wrapping lands)"
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

fn spawn_jsonrpc_tcp(
    port: u16,
    state: SharedState,
    shutdown_rx: watch::Receiver<()>,
) -> Result<std::net::SocketAddr, IpcError> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let local_addr = listener.local_addr()?;
    let listener = TcpListener::from_std(listener)?;

    tokio::spawn(async move {
        let mut shutdown_rx = shutdown_rx;
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _)) => {
                            let state = Arc::clone(&state);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, state, None).await {
                                    tracing::debug!(error = %e, "tcp connection error");
                                }
                            });
                        }
                        Err(e) => tracing::warn!(error = %e, "tcp accept failed"),
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() {
                        break;
                    }
                }
            }
        }
    });

    Ok(local_addr)
}

async fn handle_connection(
    stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
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
    use bingocube_nautilus::InstanceId;
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn unix_jsonrpc_health_liveness() {
        let dir = std::env::temp_dir().join(format!("bingocube-ipc-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let socket_path = dir.join("test.sock");
        prepare_socket_path(&socket_path).expect("prepare");

        let state = Arc::new(RwLock::new(ServerState::new(InstanceId::new("test"))));
        let (_tx, shutdown_rx) = watch::channel(());
        spawn_jsonrpc_unix(&socket_path, state, shutdown_rx, false);

        // Give the listener time to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&socket_path).await.expect("connect");
        let (reader, mut writer) = stream.into_split();
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

        let _ = std::fs::remove_file(&socket_path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

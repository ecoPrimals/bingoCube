// SPDX-License-Identifier: AGPL-3.0-or-later
//! G66 transport abstraction — silicon-agnostic IPC.
//!
//! All platform-specific code (`#[cfg(unix)]`) is confined to this module.
//! Business logic operates on [`TransportStream`] without knowing whether
//! bytes travel over Unix domain sockets, TCP, or future transports.

use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Platform-neutral endpoint descriptor.
///
/// Describes *where* to connect without prescribing *how* bytes move.
/// The launcher, biomeOS, or songBird decides the transport and injects
/// it via the `TRANSPORT_ENDPOINT` environment variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport")]
pub enum TransportEndpoint {
    /// Unix domain socket (Linux, macOS).
    #[serde(rename = "uds")]
    Uds {
        /// Filesystem path to the socket.
        path: String,
    },
    /// TCP connection (all platforms).
    #[serde(rename = "tcp")]
    Tcp {
        /// Hostname or IP address.
        host: String,
        /// Port number.
        port: u16,
    },
    /// Mesh relay via songBird routing (future).
    #[serde(rename = "mesh_relay")]
    MeshRelay {
        /// Peer identifier.
        peer_id: String,
        /// Capability name.
        capability: String,
    },
}

impl fmt::Display for TransportEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uds { path } => write!(f, "uds:{path}"),
            Self::Tcp { host, port } => write!(f, "tcp:{host}:{port}"),
            Self::MeshRelay {
                peer_id,
                capability,
            } => {
                write!(f, "mesh:{peer_id}/{capability}")
            }
        }
    }
}

impl TransportEndpoint {
    /// Platform-appropriate default endpoint.
    ///
    /// On Unix: UDS in the given socket directory.
    /// On non-Unix: TCP localhost with a conventional port.
    #[must_use]
    pub fn platform_default(primal: &str, socket_dir: &std::path::Path) -> Self {
        #[cfg(unix)]
        {
            Self::Uds {
                path: socket_dir
                    .join(format!("{primal}.sock"))
                    .to_string_lossy()
                    .into_owned(),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (primal, socket_dir);
            Self::Tcp {
                host: "127.0.0.1".to_owned(),
                port: 7700,
            }
        }
    }

    /// Read endpoint from `TRANSPORT_ENDPOINT` env var, falling back to
    /// [`platform_default`](Self::platform_default).
    ///
    /// # Errors
    ///
    /// Returns a JSON parse error if the env var is set but malformed.
    pub fn from_env_or_default(
        primal: &str,
        socket_dir: &std::path::Path,
    ) -> Result<Self, serde_json::Error> {
        match std::env::var("TRANSPORT_ENDPOINT") {
            Ok(val) => serde_json::from_str(&val),
            Err(_) => Ok(Self::platform_default(primal, socket_dir)),
        }
    }

    /// Whether this endpoint represents a local (same-host) connection.
    ///
    /// UDS is always local. TCP is local when bound to loopback.
    /// Mesh relay is never local.
    #[must_use]
    pub fn is_local(&self) -> bool {
        match self {
            Self::Uds { .. } => true,
            Self::Tcp { host, .. } => host == "127.0.0.1" || host == "localhost" || host == "::1",
            Self::MeshRelay { .. } => false,
        }
    }
}

/// Connected byte pipe — platform details confined here.
///
/// Implements [`AsyncRead`] and [`AsyncWrite`]. All protocol logic
/// (G65 negotiation, JSON-RPC, tarpc framing) operates on this type
/// without knowing the underlying transport.
pub enum TransportStream {
    /// Unix domain socket stream.
    #[cfg(unix)]
    Unix(tokio::net::UnixStream),
    /// TCP stream.
    Tcp(tokio::net::TcpStream),
}

impl fmt::Debug for TransportStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(unix)]
            Self::Unix(_) => f.write_str("TransportStream::Unix"),
            Self::Tcp(_) => f.write_str("TransportStream::Tcp"),
        }
    }
}

impl AsyncRead for TransportStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(s) => Pin::new(s).poll_read(cx, buf),
            Self::Tcp(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TransportStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(s) => Pin::new(s).poll_write(cx, buf),
            Self::Tcp(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(s) => Pin::new(s).poll_flush(cx),
            Self::Tcp(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            Self::Unix(s) => Pin::new(s).poll_shutdown(cx),
            Self::Tcp(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// Server-side listener — accepts connections as [`TransportStream`].
///
/// All `#[cfg(unix)]` conditionals for listening live here, not in
/// server logic.
pub enum TransportListener {
    /// Unix domain socket listener.
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
    /// TCP listener.
    Tcp(tokio::net::TcpListener),
}

impl fmt::Debug for TransportListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(unix)]
            Self::Unix(_) => f.write_str("TransportListener::Unix"),
            Self::Tcp(_) => f.write_str("TransportListener::Tcp"),
        }
    }
}

impl TransportListener {
    /// Bind a listener to the given endpoint.
    ///
    /// For UDS: creates parent directory and removes stale socket files.
    /// For TCP: binds synchronously and sets non-blocking mode.
    ///
    /// # Errors
    ///
    /// Returns an I/O error on bind failure or unsupported endpoint type.
    pub fn bind(endpoint: &TransportEndpoint) -> io::Result<Self> {
        match endpoint {
            #[cfg(unix)]
            TransportEndpoint::Uds { path } => {
                let p = std::path::Path::new(path);
                crate::socket::prepare_socket_path(p)?;
                let listener = tokio::net::UnixListener::bind(p)?;
                Ok(Self::Unix(listener))
            }
            #[cfg(not(unix))]
            TransportEndpoint::Uds { path } => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("UDS not available on this platform for {path}"),
            )),
            TransportEndpoint::Tcp { host, port } => {
                let addr = format!("{host}:{port}");
                let std_listener = std::net::TcpListener::bind(addr)?;
                std_listener.set_nonblocking(true)?;
                let listener = tokio::net::TcpListener::from_std(std_listener)?;
                Ok(Self::Tcp(listener))
            }
            TransportEndpoint::MeshRelay { .. } => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "mesh relay listeners require songBird routing",
            )),
        }
    }

    /// Accept a new connection, returning a [`TransportStream`].
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the accept syscall fails.
    pub async fn accept(&self) -> io::Result<TransportStream> {
        match self {
            #[cfg(unix)]
            Self::Unix(l) => {
                let (stream, _) = l.accept().await?;
                Ok(TransportStream::Unix(stream))
            }
            Self::Tcp(l) => {
                let (stream, _) = l.accept().await?;
                Ok(TransportStream::Tcp(stream))
            }
        }
    }

    /// Report the concrete bound endpoint (resolves ephemeral ports).
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the local address cannot be determined.
    pub fn local_endpoint(&self) -> io::Result<TransportEndpoint> {
        match self {
            #[cfg(unix)]
            Self::Unix(l) => {
                let addr = l.local_addr()?;
                let path = addr
                    .as_pathname()
                    .ok_or_else(|| io::Error::other("unnamed unix socket"))?
                    .to_string_lossy()
                    .into_owned();
                Ok(TransportEndpoint::Uds { path })
            }
            Self::Tcp(l) => {
                let addr = l.local_addr()?;
                Ok(TransportEndpoint::Tcp {
                    host: addr.ip().to_string(),
                    port: addr.port(),
                })
            }
        }
    }

    /// Clean up resources (removes UDS socket file, no-op for TCP).
    pub fn cleanup(&self) {
        #[cfg(unix)]
        if let Self::Unix(l) = self {
            if let Ok(addr) = l.local_addr() {
                if let Some(path) = addr.as_pathname() {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
}

/// Connect to a remote endpoint.
///
/// # Errors
///
/// Returns an I/O error on connection failure or unsupported endpoint type.
pub async fn connect_transport(endpoint: &TransportEndpoint) -> io::Result<TransportStream> {
    match endpoint {
        #[cfg(unix)]
        TransportEndpoint::Uds { path } => {
            let stream = tokio::net::UnixStream::connect(path).await?;
            Ok(TransportStream::Unix(stream))
        }
        #[cfg(not(unix))]
        TransportEndpoint::Uds { path } => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("UDS not available on this platform for {path}"),
        )),
        TransportEndpoint::Tcp { host, port } => {
            let stream = tokio::net::TcpStream::connect(format!("{host}:{port}")).await?;
            Ok(TransportStream::Tcp(stream))
        }
        TransportEndpoint::MeshRelay { .. } => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "mesh relay requires songBird routing",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn endpoint_serde_roundtrip_uds() {
        let ep = TransportEndpoint::Uds {
            path: "/run/bingocube/bingocube.sock".to_owned(),
        };
        let json = serde_json::to_string(&ep).expect("serialize");
        let back: TransportEndpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ep, back);
        assert!(json.contains(r#""transport":"uds""#));
    }

    #[test]
    fn endpoint_serde_roundtrip_tcp() {
        let ep = TransportEndpoint::Tcp {
            host: "127.0.0.1".to_owned(),
            port: 7700,
        };
        let json = serde_json::to_string(&ep).expect("serialize");
        let back: TransportEndpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ep, back);
        assert!(json.contains(r#""transport":"tcp""#));
    }

    #[test]
    fn endpoint_serde_roundtrip_mesh() {
        let ep = TransportEndpoint::MeshRelay {
            peer_id: "beardog-001".to_owned(),
            capability: "crypto.commit".to_owned(),
        };
        let json = serde_json::to_string(&ep).expect("serialize");
        let back: TransportEndpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ep, back);
    }

    #[test]
    fn endpoint_display() {
        assert_eq!(
            TransportEndpoint::Uds {
                path: "/run/foo.sock".to_owned()
            }
            .to_string(),
            "uds:/run/foo.sock"
        );
        assert_eq!(
            TransportEndpoint::Tcp {
                host: "127.0.0.1".to_owned(),
                port: 7700,
            }
            .to_string(),
            "tcp:127.0.0.1:7700"
        );
        assert_eq!(
            TransportEndpoint::MeshRelay {
                peer_id: "peer".to_owned(),
                capability: "cap".to_owned(),
            }
            .to_string(),
            "mesh:peer/cap"
        );
    }

    #[test]
    fn is_local_checks() {
        assert!(
            TransportEndpoint::Uds {
                path: "/any".to_owned()
            }
            .is_local()
        );
        assert!(
            TransportEndpoint::Tcp {
                host: "127.0.0.1".to_owned(),
                port: 1,
            }
            .is_local()
        );
        assert!(
            TransportEndpoint::Tcp {
                host: "localhost".to_owned(),
                port: 1,
            }
            .is_local()
        );
        assert!(
            TransportEndpoint::Tcp {
                host: "::1".to_owned(),
                port: 1,
            }
            .is_local()
        );
        assert!(
            !TransportEndpoint::Tcp {
                host: "10.0.0.1".to_owned(),
                port: 1,
            }
            .is_local()
        );
        assert!(
            !TransportEndpoint::MeshRelay {
                peer_id: "x".to_owned(),
                capability: "y".to_owned(),
            }
            .is_local()
        );
    }

    #[test]
    fn platform_default_returns_endpoint() {
        let ep = TransportEndpoint::platform_default(
            "bingocube",
            std::path::Path::new("/run/bingocube"),
        );
        #[cfg(unix)]
        assert!(matches!(ep, TransportEndpoint::Uds { .. }));
        #[cfg(not(unix))]
        assert!(matches!(ep, TransportEndpoint::Tcp { .. }));
    }

    #[tokio::test]
    async fn tcp_bind_accept_connect_roundtrip() {
        let ep = TransportEndpoint::Tcp {
            host: "127.0.0.1".to_owned(),
            port: 0,
        };
        let listener = TransportListener::bind(&ep).expect("bind");
        let bound = listener.local_endpoint().expect("local_endpoint");

        let port = match &bound {
            TransportEndpoint::Tcp { port, .. } => *port,
            other => panic!("expected tcp, got {other}"),
        };
        assert_ne!(port, 0);

        let connect_ep = TransportEndpoint::Tcp {
            host: "127.0.0.1".to_owned(),
            port,
        };

        let accept_task = tokio::spawn(async move { listener.accept().await });

        let mut client = connect_transport(&connect_ep).await.expect("connect");
        client.write_all(b"hello").await.expect("write");
        client.flush().await.expect("flush");

        let mut server = accept_task.await.expect("join").expect("accept");
        let mut buf = [0u8; 5];
        server.read_exact(&mut buf).await.expect("read");
        assert_eq!(&buf, b"hello");
    }

    #[test]
    fn mesh_relay_bind_unsupported() {
        let ep = TransportEndpoint::MeshRelay {
            peer_id: "x".to_owned(),
            capability: "y".to_owned(),
        };
        let err = TransportListener::bind(&ep).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[tokio::test]
    async fn mesh_relay_connect_unsupported() {
        let ep = TransportEndpoint::MeshRelay {
            peer_id: "x".to_owned(),
            capability: "y".to_owned(),
        };
        let err = connect_transport(&ep).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }
}

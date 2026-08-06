// SPDX-License-Identifier: AGPL-3.0-or-later
//! Socket path resolution for JSON-RPC and tarpc endpoints.

use std::path::{Path, PathBuf};

/// Primal identifier used in socket filenames and IPC responses.
pub const PRIMAL_NAME: &str = "bingoCube";

/// JSON-RPC socket filename.
pub const JSONRPC_SOCKET_NAME: &str = "bingocube.sock";

/// tarpc socket filename (C2 dual-socket convention).
pub const TARPC_SOCKET_NAME: &str = "bingocube.tarpc.sock";

/// Resolve the default socket directory.
///
/// Priority: explicit override via `socket_dir`, then `$XDG_RUNTIME_DIR/bingocube`,
/// then `/run/bingocube`, then `$TMPDIR/bingocube`.
#[must_use]
pub fn default_socket_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BINGOCUBE_SOCKET_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return Path::new(&runtime).join("bingocube");
    }
    PathBuf::from("/run/bingocube")
}

/// Resolve socket directory, falling back to temp when `/run` is unavailable.
#[must_use]
pub fn resolve_socket_dir(preferred: Option<PathBuf>) -> PathBuf {
    preferred.unwrap_or_else(default_socket_dir)
}

/// Full path to the JSON-RPC Unix socket.
#[must_use]
pub fn jsonrpc_socket_path(socket_dir: &Path) -> PathBuf {
    socket_dir.join(JSONRPC_SOCKET_NAME)
}

/// Full path to the tarpc Unix socket.
#[must_use]
pub fn tarpc_socket_path(socket_dir: &Path) -> PathBuf {
    socket_dir.join(TARPC_SOCKET_NAME)
}

/// Derive tarpc socket path from a JSON-RPC socket path (C2 convention).
#[must_use]
pub fn tarpc_socket_from_jsonrpc(jsonrpc_path: &Path) -> PathBuf {
    let stem = jsonrpc_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("bingocube");
    let filename = format!("{stem}.tarpc.sock");
    match jsonrpc_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(filename),
        _ => PathBuf::from(filename),
    }
}

/// Prepare a socket path for binding (create parent dir, remove stale file).
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the stale socket
/// cannot be removed.
pub fn prepare_socket_path(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarpc_path_from_jsonrpc_follows_c2_convention() {
        let jsonrpc = PathBuf::from("/run/bingocube/bingocube.sock");
        let tarpc = tarpc_socket_from_jsonrpc(&jsonrpc);
        assert_eq!(tarpc, PathBuf::from("/run/bingocube/bingocube.tarpc.sock"));
    }
}

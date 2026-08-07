// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared server state for reservoir operations.

use std::sync::Arc;

use bingocube_nautilus::{InstanceId, NautilusShell};
use tokio::sync::RwLock;

use crate::error::IpcError;

/// Mutable state held by the IPC server across requests.
#[derive(Debug)]
pub struct ServerState {
    /// Unique instance identifier for this server process.
    pub instance_id: InstanceId,
    /// Active nautilus shell (created via `reservoir.create`).
    pub reservoir: Option<NautilusShell>,
}

impl ServerState {
    /// Create fresh server state with a generated instance id.
    #[must_use]
    pub fn new(instance_id: InstanceId) -> Self {
        Self {
            instance_id,
            reservoir: None,
        }
    }

    /// Replace the active reservoir shell.
    pub fn set_reservoir(&mut self, shell: NautilusShell) {
        self.reservoir = Some(shell);
    }

    /// Borrow the active reservoir shell.
    ///
    /// # Errors
    ///
    /// Returns an error when no shell has been created yet.
    pub fn reservoir(&self) -> Result<&NautilusShell, IpcError> {
        self.reservoir
            .as_ref()
            .ok_or_else(|| IpcError::handler("no reservoir shell — call reservoir.create first"))
    }

    /// Mutably borrow the active reservoir shell.
    ///
    /// # Errors
    ///
    /// Returns an error when no shell has been created yet.
    pub fn reservoir_mut(&mut self) -> Result<&mut NautilusShell, IpcError> {
        self.reservoir
            .as_mut()
            .ok_or_else(|| IpcError::handler("no reservoir shell — call reservoir.create first"))
    }
}

/// Thread-safe handle to [`ServerState`].
pub type SharedState = Arc<RwLock<ServerState>>;

/// Build a new shared state handle.
#[must_use]
pub fn new_shared_state(instance_id: InstanceId) -> SharedState {
    Arc::new(RwLock::new(ServerState::new(instance_id)))
}

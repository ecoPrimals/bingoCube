//! # bingocube-nautilus
//!
//! Evolutionary reservoir computing built on BingoCube boards.
//!
//! The **nautilus shell** is a layered evolutionary history of board populations.
//! Each generation wraps the previous — preserving heritage while adding adaptation.
//!
//! ## Architecture
//!
//! ```text
//! Traditional ESN (requires recurrence):
//!   input(t) → reservoir(t) = f(W_in·x(t) + W_res·state(t-1))
//!                                                 ↑ feedback loop
//!
//! BingoCube Reservoir (pure feed-forward):
//!   input → Board₁ response → ┐
//!   input → Board₂ response → ├→ FC readout → output
//!   input → Board₃ response → ┘
//!           ↑ no feedback, N boards run in parallel
//! ```
//!
//! ## Within an Instance
//!
//! A single [`NautilusShell`] evolves its [`Population`] of boards through
//! [`Generation`]s. At each generation:
//!
//! 1. Input data streams through all boards (each board is a random projection)
//! 2. The [`LinearReadout`] extracts predictions from the ensemble response
//! 3. Boards are evaluated by fitness (prediction correlation)
//! 4. Top performers inform the next generation via [`Evolution`]
//!
//! ## Between Instances
//!
//! A trained shell serializes to JSON (or binary). Another machine loads it
//! and either:
//! - Continues evolving from the inherited generation
//! - Uses the shell as "informed randomness" for a different environment
//! - Merges shells from multiple sources
//!
//! The shell is the portable unit of learned structure.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod response;
mod population;
mod evolution;
mod readout;
mod shell;
mod constraints;
mod brain;

pub use response::{BoardResponse, ReservoirInput, ResponseVector};
pub use population::{Population, FitnessRecord};
pub use evolution::{Evolution, EvolutionConfig, SelectionMethod};
pub use readout::LinearReadout;
pub use shell::{NautilusShell, ShellConfig, GenerationRecord, InstanceId, Akd1000Export};
pub use constraints::{ConstraintLevel, DriftMonitor, DriftAction, EdgeSeeder, board_satisfies};
pub use brain::{NautilusBrain, NautilusBrainConfig, BetaObservation};

/// Re-export core types used across the API.
pub use bingocube_core::{Board, BingoCube, Config, BingoCubeError};

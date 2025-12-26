//! # bingocube-adapters
//!
//! Visualization adapters for BingoCube. These are OPTIONAL helpers that allow
//! visualization systems (like petalTongue) to render BingoCube data.
//!
//! ## Design Philosophy
//!
//! BingoCube core is a pure tool - it provides cryptographic primitives.
//! These adapters help systems that WANT to visualize BingoCubes, but are
//! not required for using BingoCube as a tool.
//!
//! ## Features
//!
//! - `visual`: Enable visual rendering (requires egui)
//! - `audio`: Enable audio sonification (requires audio libraries)
//! - `animation`: Enable animation control (requires animation engine)
//!
//! ## Example
//!
//! ```rust,ignore
//! use bingocube_core::{BingoCube, Config};
//! use bingocube_adapters::visual::BingoCubeVisualAdapter;
//!
//! let cube = BingoCube::from_seed(b"alice", Config::default())?;
//!
//! // In your egui UI:
//! BingoCubeVisualAdapter::render(&cube, 0.5, ui);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

#[cfg(feature = "visual")]
pub mod visual;

#[cfg(feature = "audio")]
pub mod audio;

// #[cfg(feature = "animation")]
// pub mod animation;

// Re-export core for convenience
pub use bingocube_core;

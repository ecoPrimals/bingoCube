// SPDX-License-Identifier: AGPL-3.0-or-later
//! # bingocube-core
//!
//! Core implementation of `BingoCube`: a multi-dimensional visual verification system
//! that combines structured combinatorics with cryptographic hashing.
//!
//! ## Overview
//!
//! `BingoCube` creates verifiable, multi-resolution visual artifacts by:
//! 1. Generating two "bingo-style" boards (A and B) with column range constraints
//! 2. Cross-binding them via cryptographic hashing
//! 3. Producing a color grid that commits to both boards
//! 4. Supporting progressive reveal via continuous parameter x ∈ (0,1]
//!
//! ## Example
//!
//! ```rust
//! use bingocube_core::{BingoCube, Config};
//!
//! // Generate BingoCube from seed
//! let cube = BingoCube::from_seed(b"alice_identity", Config::default())
//!     .expect("failed to generate cube");
//!
//! // Get full color grid
//! let grid = cube.color_grid();
//!
//! // Get partial reveal (50%)
//! let subcube = cube.subcube(0.5).expect("failed to generate subcube");
//!
//! // Verify subcube
//! assert!(cube.verify_subcube(&subcube, 0.5));
//! ```

#![warn(missing_docs)]

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during `BingoCube` operations
#[derive(Debug, Error)]
pub enum BingoCubeError {
    /// Invalid grid size (must be > 0)
    #[error("Invalid grid size: {0} (must be > 0)")]
    InvalidGridSize(usize),

    /// Invalid universe size (must be divisible by grid size)
    #[error("Invalid universe size: {0} (must be divisible by grid size {1})")]
    InvalidUniverseSize(usize, usize),

    /// Invalid palette size (must be > 0)
    #[error("Invalid palette size: {0} (must be > 0)")]
    InvalidPaletteSize(usize),

    /// Invalid reveal parameter (must be in (0,1])
    #[error("Invalid reveal parameter: {0} (must be in (0,1])")]
    InvalidRevealParameter(f64),
}

/// Configuration for `BingoCube` generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Grid dimension (L×L)
    pub grid_size: usize,

    /// Universe size (0..universe_size-1)
    pub universe_size: usize,

    /// Color palette size
    pub palette_size: usize,

    /// Optional: Position of free cell (row, col)
    pub free_cell: Option<(usize, usize)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grid_size: 5,            // 5×5 classic bingo
            universe_size: 100,      // 0-99
            palette_size: 16,        // 16 colors
            free_cell: Some((2, 2)), // Center cell
        }
    }
}

impl Config {
    /// Create a small configuration (classic bingo)
    #[must_use]
    pub fn small() -> Self {
        Self::default()
    }

    /// Create a medium configuration
    #[must_use]
    pub const fn medium() -> Self {
        Self {
            grid_size: 8,
            universe_size: 512,
            palette_size: 64,
            free_cell: Some((3, 3)),
        }
    }

    /// Create a large configuration
    #[must_use]
    pub const fn large() -> Self {
        Self {
            grid_size: 12,
            universe_size: 1000,
            palette_size: 256,
            free_cell: Some((5, 5)),
        }
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid
    pub const fn validate(&self) -> Result<(), BingoCubeError> {
        if self.grid_size == 0 {
            return Err(BingoCubeError::InvalidGridSize(self.grid_size));
        }

        if !self.universe_size.is_multiple_of(self.grid_size) {
            return Err(BingoCubeError::InvalidUniverseSize(
                self.universe_size,
                self.grid_size,
            ));
        }

        if self.palette_size == 0 {
            return Err(BingoCubeError::InvalidPaletteSize(self.palette_size));
        }

        Ok(())
    }

    /// Get per-column range size
    #[must_use]
    pub const fn range_size(&self) -> usize {
        self.universe_size / self.grid_size
    }
}

/// A single bingo board with column range constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    /// Grid values (L×L), None for free cells
    pub grid: Vec<Vec<Option<u32>>>,

    /// Grid dimension
    pub size: usize,

    /// Column permutation (symbol order)
    pub permutation: Vec<usize>,
}

impl Board {
    /// Generate a random board from RNG
    ///
    /// # Errors
    ///
    /// Returns error if config is invalid
    pub fn generate<R: Rng>(config: &Config, rng: &mut R) -> Result<Self, BingoCubeError> {
        config.validate()?;

        let size = config.grid_size;
        let range_size = config.range_size();

        // Generate column permutation
        let mut permutation: Vec<usize> = (0..size).collect();
        permutation.shuffle(rng);

        // Generate grid
        let mut grid = vec![vec![None; size]; size];

        for col in 0..size {
            // Determine range for this column
            let range_start = col * range_size;
            let range_end = range_start + range_size;

            // Generate distinct values for this column
            #[expect(
                clippy::cast_possible_truncation,
                reason = "column ranges are bounded by validated universe and grid size"
            )]
            let mut values: Vec<u32> = (range_start..range_end).map(|v| v as u32).collect();
            values.shuffle(rng);

            // Fill column (skip free cell if present)
            let mut value_idx = 0;
            for (row, row_cells) in grid.iter_mut().enumerate().take(size) {
                if let Some((free_row, free_col)) = config.free_cell
                    && row == free_row && col == free_col
                {
                    row_cells[col] = None; // Free cell
                    continue;
                }

                if value_idx < values.len() {
                    row_cells[col] = Some(values[value_idx]);
                    value_idx += 1;
                }
            }
        }

        Ok(Self {
            grid,
            size,
            permutation,
        })
    }

    /// Get value at position (row, col)
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> Option<u32> {
        self.grid
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .flatten()
    }
}

/// A color (index into palette)
pub type Color = u8;

/// A cell position
pub type Position = (usize, usize);

/// A scalar field value (hash output)
type Scalar = u64;

/// The main `BingoCube` structure
#[derive(Debug, Clone)]
pub struct BingoCube {
    /// Board A (depth layer 0)
    pub board_a: Board,

    /// Board B (depth layer 1)
    pub board_b: Board,

    /// Configuration
    pub config: Config,

    /// Scalar field (d[i,j] values)
    scalar_field: Vec<Vec<Scalar>>,

    /// Color grid (c[i,j] values)
    color_grid: Vec<Vec<Color>>,
}

impl BingoCube {
    /// Generate a `BingoCube` from a seed
    ///
    /// # Errors
    ///
    /// Returns error if config is invalid
    pub fn from_seed(seed: &[u8], config: Config) -> Result<Self, BingoCubeError> {
        config.validate()?;

        // Derive two seeds from input seed
        let seed_a = blake3::hash(&[seed, b"_BOARD_A"].concat());
        let seed_b = blake3::hash(&[seed, b"_BOARD_B"].concat());

        let mut rng_a = ChaCha20Rng::from_seed(*seed_a.as_bytes());
        let mut rng_b = ChaCha20Rng::from_seed(*seed_b.as_bytes());

        let board_a = Board::generate(&config, &mut rng_a)?;
        let board_b = Board::generate(&config, &mut rng_b)?;

        Self::from_boards(board_a, board_b, config)
    }

    /// Create a `BingoCube` from explicit boards
    ///
    /// # Errors
    ///
    /// Returns error if config is invalid or boards have wrong size
    pub fn from_boards(
        board_a: Board,
        board_b: Board,
        config: Config,
    ) -> Result<Self, BingoCubeError> {
        config.validate()?;

        let size = config.grid_size;

        // Compute scalar field
        let mut scalar_field = vec![vec![0u64; size]; size];
        for (i, row) in scalar_field.iter_mut().enumerate().take(size) {
            for (j, cell) in row.iter_mut().enumerate().take(size) {
                *cell = Self::compute_scalar(i, j, &board_a, &board_b);
            }
        }

        // Compute color grid
        let mut color_grid = vec![vec![0u8; size]; size];
        for (i, row) in color_grid.iter_mut().enumerate().take(size) {
            for (j, cell) in row.iter_mut().enumerate().take(size) {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "palette index is reduced modulo palette_size"
                )]
                let c = (scalar_field[i][j] % config.palette_size as u64) as u8;
                *cell = c;
            }
        }

        Ok(Self {
            board_a,
            board_b,
            config,
            scalar_field,
            color_grid,
        })
    }

    /// Compute scalar value for cell (i, j)
    fn compute_scalar(i: usize, j: usize, board_a: &Board, board_b: &Board) -> Scalar {
        // Build input: "BINGOCUBE_V1" || i || j || A[i,j] || B[i,j]
        let mut input = Vec::new();
        input.extend_from_slice(b"BINGOCUBE_V1");
        input.extend_from_slice(&i.to_le_bytes());
        input.extend_from_slice(&j.to_le_bytes());

        // Add A[i,j] value (or marker for None)
        match board_a.get(i, j) {
            Some(val) => input.extend_from_slice(&val.to_le_bytes()),
            None => input.extend_from_slice(b"FREE_CELL_A"),
        }

        // Add B[i,j] value (or marker for None)
        match board_b.get(i, j) {
            Some(val) => input.extend_from_slice(&val.to_le_bytes()),
            None => input.extend_from_slice(b"FREE_CELL_B"),
        }

        // Hash and interpret as u64
        let hash = blake3::hash(&input);
        let bytes = hash.as_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    /// Get the full color grid
    #[must_use]
    pub const fn color_grid(&self) -> &Vec<Vec<Color>> {
        &self.color_grid
    }

    /// Get color at position
    #[must_use]
    pub fn get_color(&self, row: usize, col: usize) -> Option<Color> {
        self.color_grid.get(row).and_then(|r| r.get(col)).copied()
    }

    /// Get scalar value at position
    #[must_use]
    pub fn get_scalar(&self, row: usize, col: usize) -> Option<Scalar> {
        self.scalar_field.get(row).and_then(|r| r.get(col)).copied()
    }

    /// Get a subcube at reveal level x
    ///
    /// # Errors
    ///
    /// Returns error if x is not in (0,1]
    pub fn subcube(&self, x: f64) -> Result<SubCube, BingoCubeError> {
        if x <= 0.0 || x > 1.0 {
            return Err(BingoCubeError::InvalidRevealParameter(x));
        }

        let size = self.config.grid_size;
        let total_cells = size * size;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "grid dimensions are small, x is in (0,1], product is always non-negative and fits in usize"
        )]
        let reveal_count = (x * total_cells as f64).ceil() as usize;

        // Collect all cells with their scalar values
        let mut cells: Vec<(Position, Scalar, Color)> = Vec::new();
        for (i, (sf_row, cg_row)) in self
            .scalar_field
            .iter()
            .zip(&self.color_grid)
            .enumerate()
            .take(size)
        {
            for (j, (&s, &c)) in sf_row.iter().zip(cg_row).enumerate().take(size) {
                cells.push(((i, j), s, c));
            }
        }

        // Sort by scalar value (descending)
        cells.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top reveal_count cells
        let revealed: HashMap<Position, Color> = cells
            .into_iter()
            .take(reveal_count)
            .map(|((i, j), _, color)| ((i, j), color))
            .collect();

        Ok(SubCube { size, revealed, x })
    }

    /// Verify a subcube
    #[must_use]
    pub fn verify_subcube(&self, subcube: &SubCube, x: f64) -> bool {
        self.subcube(x).is_ok_and(|expected| expected == *subcube)
    }
}

/// A partial reveal of a [`BingoCube`] at level x
#[derive(Debug, Clone, PartialEq)]
pub struct SubCube {
    /// Grid size
    pub size: usize,

    /// Revealed cells: (position) -> color
    pub revealed: HashMap<Position, Color>,

    /// Reveal parameter
    pub x: f64,
}

impl SubCube {
    /// Check if a cell is revealed
    #[must_use]
    pub fn is_revealed(&self, row: usize, col: usize) -> bool {
        self.revealed.contains_key(&(row, col))
    }

    /// Get color at position (if revealed)
    #[must_use]
    pub fn get_color(&self, row: usize, col: usize) -> Option<Color> {
        self.revealed.get(&(row, col)).copied()
    }

    /// Get number of revealed cells
    #[must_use]
    pub fn revealed_count(&self) -> usize {
        self.revealed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid = Config::default();
        assert!(valid.validate().is_ok());

        let invalid_size = Config {
            grid_size: 0,
            ..Default::default()
        };
        assert!(invalid_size.validate().is_err());

        let invalid_universe = Config {
            universe_size: 99, // Not divisible by 5
            ..Default::default()
        };
        assert!(invalid_universe.validate().is_err());
    }

    #[test]
    fn test_board_generation() {
        let config = Config::default();
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let board = Board::generate(&config, &mut rng).expect("board generation failed");

        assert_eq!(board.size, 5);
        assert_eq!(board.grid.len(), 5);
        assert_eq!(board.permutation.len(), 5);

        // Check free cell
        assert_eq!(board.get(2, 2), None);

        // Check column range constraints
        for col in 0..5 {
            let range_start = col * 20;
            let range_end = range_start + 20;

            for row in 0..5 {
                if row == 2 && col == 2 {
                    continue; // Skip free cell
                }

                if let Some(val) = board.get(row, col) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "test grid column ranges fit in u32"
                    )]
                    let rs = range_start as u32;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "test grid column ranges fit in u32"
                    )]
                    let re = range_end as u32;
                    assert!(val >= rs);
                    assert!(val < re);
                }
            }
        }
    }

    #[test]
    fn test_bingocube_from_seed() {
        let config = Config::default();
        let cube = BingoCube::from_seed(b"test_seed", config).expect("cube generation failed");

        assert_eq!(cube.config.grid_size, 5);
        assert_eq!(cube.color_grid.len(), 5);
        assert_eq!(cube.scalar_field.len(), 5);
    }

    #[test]
    fn test_subcube_generation() {
        let config = Config::default();
        let cube = BingoCube::from_seed(b"test_seed", config).expect("cube generation failed");

        // Test various x values
        let sub_20 = cube.subcube(0.2).expect("subcube generation failed");
        let sub_50 = cube.subcube(0.5).expect("subcube generation failed");
        let sub_100 = cube.subcube(1.0).expect("subcube generation failed");

        assert_eq!(sub_20.revealed_count(), 5); // 20% of 25
        assert_eq!(sub_50.revealed_count(), 13); // 50% of 25
        assert_eq!(sub_100.revealed_count(), 25); // 100% of 25

        // Test nesting property: sub_20 ⊂ sub_50 ⊂ sub_100
        for pos in sub_20.revealed.keys() {
            assert!(sub_50.revealed.contains_key(pos));
            assert!(sub_100.revealed.contains_key(pos));
        }

        for pos in sub_50.revealed.keys() {
            assert!(sub_100.revealed.contains_key(pos));
        }
    }

    #[test]
    fn test_subcube_verification() {
        let config = Config::default();
        let cube = BingoCube::from_seed(b"test_seed", config).expect("cube generation failed");

        let subcube = cube.subcube(0.5).expect("subcube generation failed");
        assert!(cube.verify_subcube(&subcube, 0.5));

        // Wrong x should fail
        assert!(!cube.verify_subcube(&subcube, 0.3));
    }

    #[test]
    fn test_deterministic_generation() {
        let config = Config::default();

        let cube1 = BingoCube::from_seed(b"same_seed", config.clone()).expect("generation failed");
        let cube2 = BingoCube::from_seed(b"same_seed", config).expect("generation failed");

        // Same seed should produce identical cubes
        for i in 0..5 {
            for j in 0..5 {
                assert_eq!(cube1.get_color(i, j), cube2.get_color(i, j));
                assert_eq!(cube1.get_scalar(i, j), cube2.get_scalar(i, j));
            }
        }
    }

    #[test]
    fn test_different_seeds_produce_different_cubes() {
        let config = Config::default();

        let cube1 = BingoCube::from_seed(b"seed1", config.clone()).expect("generation failed");
        let cube2 = BingoCube::from_seed(b"seed2", config).expect("generation failed");

        // Different seeds should produce different cubes
        let mut scalar_differences = 0;
        for i in 0..5 {
            for j in 0..5 {
                // Skip free cell (both cubes have same free cell hash)
                if i == 2 && j == 2 {
                    continue;
                }
                if cube1.get_scalar(i, j) != cube2.get_scalar(i, j) {
                    scalar_differences += 1;
                }
            }
        }

        // Expect all non-free cells to be different (24 cells)
        assert_eq!(scalar_differences, 24);
    }
}

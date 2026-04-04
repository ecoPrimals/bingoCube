# bingoCube — Context

Human-verifiable cryptographic commitment system. Generates multi-dimensional visual
artifacts by cross-binding two bingo boards via BLAKE3 hashing, producing a color grid
with progressive reveal.

## Workspace Structure

| Crate | Role | Type |
|-------|------|------|
| `bingocube-core` | Two-board cross-binding, scalar field, color grid, subcube reveal | library |
| `bingocube-adapters` | Visual (egui), audio, animation adapters | library (feature-gated) |
| `bingocube-demos` | Interactive egui demo binary | binary |
| `bingocube-nautilus` | Evolutionary reservoir computing via board populations | library |

## Key Concepts

- **Board**: L×L grid with column-range constraints, generated from ChaCha20 RNG
- **Scalar field**: BLAKE3 hash of board cell pairs → u64
- **Color grid**: Scalar field mod palette_size → u8 color indices
- **SubCube**: Progressive reveal at level x ∈ (0,1] — top-x% cells by scalar value
- **Nautilus Shell**: Population of boards evolved via selection/crossover/mutation

## Tests

54 tests (15 core, 7 adapters, 31 nautilus, 1 doctest), 0 failures.

## Status

v0.1.1 — Edition 2024, clippy pedantic+nursery clean, `forbid(unsafe_code)` workspace-wide.
Ecosystem tool (not an IPC daemon). Consumed by primals and springs as a Rust crate dependency.

## Dependencies

Pure Rust. No C dependencies. Key deps: blake3, rand/rand_chacha, serde, thiserror, egui (optional).

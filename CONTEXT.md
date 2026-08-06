# bingoCube — Context

Human-verifiable cryptographic commitment system with IPC server. Generates
multi-dimensional visual artifacts by cross-binding two bingo boards via BLAKE3
hashing, producing a color grid with progressive reveal. Evolutionary reservoir
computing via nautilus shell populations.

## Workspace Structure

| Crate | Role | Type |
|-------|------|------|
| `bingocube-core` | Two-board cross-binding, scalar field, color grid, subcube reveal | library |
| `bingocube-adapters` | Visual (egui), audio, animation adapters | library (feature-gated) |
| `bingocube-demos` | Interactive egui demo binary | binary + library |
| `bingocube-nautilus` | Evolutionary reservoir computing via board populations | library |
| `bingocube-ipc` | JSON-RPC 2.0 + tarpc 0.37 IPC server (C2 dual-socket) | library |
| `bingocube-cli` | UniBin binary: serve, demo, generate, verify | binary |

## IPC Methods (10)

capabilities.list, health.liveness, health.check, identity.get,
crypto.commit, crypto.reveal, crypto.verify,
reservoir.create, reservoir.evolve, reservoir.predict

## Key Concepts

- **Board**: L×L grid with column-range constraints, generated from ChaCha20 RNG
- **Scalar field**: BLAKE3 hash of board cell pairs → u64
- **Color grid**: Scalar field mod palette_size → u8 color indices
- **SubCube**: Progressive reveal at level x ∈ (0,1] — top-x% cells by scalar value
- **Nautilus Shell**: Population of boards evolved via selection/crossover/mutation

## Tests

82 tests (7 core, 21 adapters, 47 nautilus, 6 IPC, 1 doctest), 0 failures.

## Status

v0.2.0 — C2 dual-socket shipped. Edition 2024, clippy pedantic+nursery clean,
`forbid(unsafe_code)` workspace-wide. Zero `.expect()` in library code.
scyBorg triple license (AGPL + ORC + CC-BY-SA).

## Dependencies

Pure Rust. No C dependencies. Key deps: blake3, rand/rand_chacha, serde,
thiserror, tokio, clap, tarpc (feature-gated), egui (feature-gated).

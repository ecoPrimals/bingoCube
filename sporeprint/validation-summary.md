+++
title = "bingoCube Validation Summary"
description = "Human-verifiable cryptographic commitment system — cross-bound bingo boards, progressive reveal, evolutionary reservoir computing. G65 protocol negotiation + C2 dual-socket IPC. 94 tests, pure Rust."
date = 2026-08-06

[taxonomies]
primals = ["bingocube"]
springs = []
+++

## Status

- **Gate**: CLEAR — G65 protocol negotiation shipped (C2 dual-socket as fallback)
- **Phase**: G65 (single-socket protocol negotiation: tarpc vs JSON-RPC)
- **Edition**: 2024
- **Tests**: 94 passing (7 core, 21 adapters, 47 nautilus, 18 IPC, 1 doctest)
- **Coverage**: 84% line (llvm-cov), fail-under: 80%
- **Clippy**: 0 warnings (`pedantic` + `nursery`, `-D warnings`)
- **Unsafe**: zero (`forbid(unsafe_code)` workspace-wide)
- **Pure Rust**: No C dependencies (blake3, rand/rand_chacha, serde, tokio, tarpc, egui optional)
- **License**: scyBorg triple (AGPL-3.0-or-later + ORC + CC-BY-SA 4.0)
- **cargo deny**: advisories ok, bans ok, licenses ok, sources ok

## IPC Methods (10)

| Domain | Methods |
|--------|---------|
| **capabilities** | capabilities.list |
| **health** | health.liveness, health.check |
| **identity** | identity.get |
| **crypto** | crypto.commit, crypto.reveal, crypto.verify |
| **reservoir** | reservoir.create, reservoir.evolve, reservoir.predict |

## Key Concepts

| Concept | Description |
|---------|-------------|
| **Board** | L x L grid with column-range constraints, ChaCha20 RNG |
| **Scalar field** | BLAKE3 hash of board cell pairs mapped to u64 |
| **Color grid** | Scalar field mod palette_size for visual rendering |
| **SubCube** | Progressive reveal at level x in (0,1] — top-x% cells by scalar value |
| **Nautilus Shell** | Population of boards evolved via selection/crossover/mutation |

## Crates (6)

| Crate | Role | Type |
|-------|------|------|
| `bingocube-core` | Two-board cross-binding, scalar field, color grid, subcube reveal | library |
| `bingocube-adapters` | Visual (egui), audio, animation adapters | library (feature-gated) |
| `bingocube-demos` | Interactive egui demo + library target | binary + library |
| `bingocube-nautilus` | Evolutionary reservoir computing via board populations | library |
| `bingocube-ipc` | JSON-RPC 2.0 + tarpc 0.37 IPC server (C2 + G65 negotiation) | library |
| `bingocube-cli` | UniBin binary (serve, demo, generate, verify) | binary |

## Composition Role

bingoCube operates as both an **ecosystem library** and an **IPC service**.
Library consumers link at compile time; the UniBin binary (`bingocube serve`)
exposes crypto and reservoir operations via JSON-RPC and tarpc sockets.

## Downstream Consumers

- bearDog (commitment artifact generation)
- lithoSpore (visual provenance verification)
- primalSpring (deployment commitment proof)

## Degradation

If the IPC server is unavailable, consumers fall back to direct library linkage.
Core cryptographic operations have no external dependencies.

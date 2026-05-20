+++
title = "bingoCube Validation Summary"
description = "Human-verifiable cryptographic commitment system — cross-bound bingo boards, progressive reveal, evolutionary reservoir computing. 73 tests, pure Rust."
date = 2026-05-20

[taxonomies]
primals = ["bingocube"]
springs = []
+++

## Status

- **Gate**: CLEAR (ecosystem library/tool — no IPC, MethodGate N/A)
- **Phase**: N/A (library crates, no runtime server)
- **Edition**: 2024
- **Tests**: 73 passing (15 core, 7 adapters, 31 nautilus, 1 doctest, 19 integration)
- **Coverage**: 83.4% line (tarpaulin, fail-under: 60%)
- **Clippy**: 0 warnings (`pedantic` + `nursery`, `-D warnings`)
- **Unsafe**: zero (`forbid(unsafe_code)` workspace-wide)
- **Pure Rust**: No C dependencies (blake3, rand/rand_chacha, serde, egui optional)

## Key Concepts

| Concept | Description |
|---------|-------------|
| **Board** | L x L grid with column-range constraints, ChaCha20 RNG |
| **Scalar field** | BLAKE3 hash of board cell pairs mapped to u64 |
| **Color grid** | Scalar field mod palette_size for visual rendering |
| **SubCube** | Progressive reveal at level x in (0,1] — top-x% cells by scalar value |
| **Nautilus Shell** | Population of boards evolved via selection/crossover/mutation |

## Crates (4)

| Crate | Role | Type |
|-------|------|------|
| `bingocube-core` | Two-board cross-binding, scalar field, color grid, subcube reveal | library |
| `bingocube-adapters` | Visual (egui), audio, animation adapters | library (feature-gated) |
| `bingocube-demos` | Interactive egui demo binary | binary |
| `bingocube-nautilus` | Evolutionary reservoir computing via board populations | library |

## Composition Role

bingoCube is an **ecosystem library** — not part of runtime compositions.
It provides human-verifiable commitment artifacts that primals and springs
consume as a Rust crate dependency. The visual commitment system enables
cryptographic proofs that humans can visually verify without tooling.

## Downstream Consumers

- bearDog (commitment artifact generation)
- lithoSpore (visual provenance verification)
- primalSpring (deployment commitment proof)

## Degradation

bingoCube is a library — no runtime degradation mode. Consumers link
against it at compile time; no IPC dependency.

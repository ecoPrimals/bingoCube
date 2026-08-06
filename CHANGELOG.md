# Changelog

All notable changes to bingoCube are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] — 2026-08-06

### Added — G65 Protocol Negotiation
- `negotiation` module in bingocube-ipc: single-socket protocol selection (tarpc vs JSON-RPC)
- G65 wire protocol: `PROTOCOLS: tarpc,jsonrpc\n` → `PROTOCOL: tarpc\n`
- `IpcProtocol`, `NegotiationRequest`, `NegotiationResponse`, `NegotiationError` types
- `negotiate_client()`, `negotiate_server()`, `negotiate_server_outcome()` async functions
- `ServerNegotiationOutcome` preserves legacy first line for backward-compatible JSON-RPC fallback
- `--negotiate` CLI flag (env: `BINGOCUBE_NEGOTIATE`) for G65 single-socket mode
- 100ms timeout for backward compatibility with Phase 1/2 clients
- 12 new tests (wire roundtrip, duplex negotiation, legacy fallback, selection algorithm)

### Changed
- `ServeConfig` gains `negotiate: bool` — when true, skips separate `.tarpc.sock` listener
- Server accept loop routes tarpc-negotiated connections to stub handler (full transport wrapping in convergence)

### Metrics
- 6 crates, 94 tests (was 82), ~9,700 lines Rust (was 9,253)
- G65 protocol negotiation: SHIPPED
- All prior C2 dual-socket and zero-copy optimizations intact

## [0.2.0] — 2026-08-06

### Added — C2 Dual-Socket Cephalization
- `bingocube-ipc` crate: JSON-RPC 2.0 server on Unix socket + tarpc 0.37 C2 dual-socket (feature-gated)
- `bingocube-cli` crate: UniBin binary with clap subcommands (serve, demo, generate, verify)
- 10 semantic IPC methods: capabilities.list, health.liveness, health.check, identity.get, crypto.commit, crypto.reveal, crypto.verify, reservoir.create, reservoir.evolve, reservoir.predict
- `.github/workflows/ci.yml` — fmt, clippy, test, doc, deny CI pipeline
- `VisualConfig`, `AudioConfig`, `AnimationConfig` — hardcoded constants evolved to configuration
- scyBorg triple license model (AGPL + ORC + CC-BY-SA 4.0)

### Changed
- **API evolution**: `NautilusShell::new()`, `from_seed()`, `evolve_generation()`, `merge_shell()` now return `Result` — zero `.expect()` in library code
- **Zero-copy hashing**: `compute_scalar()` uses incremental `blake3::Hasher` (was `Vec::new()` per cell)
- `color_grid()` returns `&[Vec<Color>]` (was `&Vec<Vec<Color>>`)
- Seed derivation uses streaming hash (was `.concat()` allocation)
- Tarpaulin fail-under raised from 60% to 80%
- WhitePaper Biometric Identity split into 3 focused docs (was 2,270 lines)
- All adapter configs now builder-pattern with defaults preserving prior behavior

### Fixed
- 7 broken rustdoc intra-doc links in nautilus
- cargo-deny advisory + license failures (transitive egui deps)
- Demo graceful error handling (was `.expect()` on startup)

### Metrics
- 6 crates (was 4), 82 tests (was 73), 9,253 lines Rust (was 7,024)
- clippy pedantic+nursery: 0 warnings
- cargo deny: advisories ok, bans ok, licenses ok, sources ok
- cargo doc: 0 warnings
- All files under 1,000 lines

## [0.1.1] — 2026-04-04

### Changed
- Edition 2024 (was 2021)
- License `AGPL-3.0-or-later` (was bare `AGPL-3.0`)
- Workspace lints: `forbid(unsafe_code)`, `warn(missing_docs)`, clippy pedantic + nursery
- All 4 member crates inherit workspace lints
- SPDX headers on all source files
- `gen` variable renamed to `gen_idx` / `generation` (reserved keyword in 2024 edition)
- `rng.gen()` → `rng.r#gen()` for rand 0.8 compat under 2024 edition

### Added
- `CHANGELOG.md` (this file)
- `CONTEXT.md`
- `deny.toml`
- `nautilus/` documented in README project structure

### Fixed
- 22 clippy errors (cast safety, doc_markdown, const fn, option_if_let_else, iterator patterns)
- Restored `animation` module behind `animation` feature gate in adapters

### Changed (Deep Debt Sprint — 2026-04-05)
- Public-readiness scrub: internal docs removed, home paths scrubbed, path dep made optional
- Coverage: 62.6% → 83.4% (73 tests, tarpaulin.toml with fail-under=60.0)
- Refactored shell.rs into shell.rs + snapshot.rs + evolve.rs
- README AGPL wording fixed, whitePaper license aligned

## [0.1.0] — 2025-12-26

### Added
- Initial release: `bingocube-core`, `bingocube-adapters`, `bingocube-demos`
- Two-board cross-binding with BLAKE3 + ChaCha20
- Progressive reveal via continuous parameter x ∈ (0,1]
- Visual, audio, and animation adapters (feature-gated)
- `bingocube-nautilus`: evolutionary reservoir computing (shell, population, evolution, constraints, brain, response, readout)
- 5 nautilus examples (shell_lifecycle, live_qcd_prediction, quenched_to_dynamical, predict_live_exp029, full_brain_rehearsal)

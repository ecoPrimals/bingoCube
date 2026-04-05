# Changelog

All notable changes to bingoCube are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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

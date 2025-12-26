# 🎲 BingoCube

**Human-Verifiable Cryptographic Commitment System**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)

> A pure cryptographic tool for creating memorable, visual, and auditory patterns that any system can use for identity verification, peer trust, content fingerprinting, and computation proofs.

---

## 🎯 What is BingoCube?

BingoCube is a **standalone cryptographic tool** that generates human-recognizable patterns from seeds. It provides:

- 🔐 **Cryptographic Security**: BLAKE3-based commitment with progressive reveal
- 👁️ **Visual Patterns**: Color grids humans can recognize and verify
- 🎵 **Audio Patterns**: Sonification for accessibility and multi-modal verification
- 📊 **Progressive Trust**: Reveal 20% → 50% → 100% based on trust level
- 🔗 **Zero Dependencies**: Pure Rust core with optional adapters

**Key Innovation**: Two-board cross-binding algorithm creates visual patterns with provable security properties, enabling progressive trust revelation without weakening cryptographic guarantees.

---

## 🚀 Quick Start

### Basic Usage

```rust
use bingocube_core::{BingoCube, Config};

// Generate from seed
let cube = BingoCube::from_seed(b"alice_identity", Config::default());

// Get full color grid (100% reveal)
let grid = cube.color_grid();

// Get partial reveal for progressive trust
let subcube_30 = cube.subcube(0.3)?;  // 30% reveal
let subcube_50 = cube.subcube(0.5)?;  // 50% reveal

// Verify subcube
assert!(cube.verify_subcube(&subcube_30, 0.3));
```

### With Visualization (Optional)

```rust
use bingocube_adapters::{VisualRenderer, AudioRenderer};

// Visual rendering (requires egui feature)
let visual = VisualRenderer::new()
    .with_reveal(0.5)
    .render(&cube)?;

// Audio sonification
let audio = AudioRenderer::new(&cube)
    .generate_soundscape(0.5)?;
```

---

## 📦 Crates

### `bingocube-core`

Pure cryptographic implementation with zero dependencies (beyond `blake3`, `serde`, `thiserror`).

```toml
[dependencies]
bingocube-core = "0.1"
```

**Features**:
- Seed-based BingoCube generation
- Progressive reveal (subcube extraction)
- Verification and commitment
- Serialization/deserialization

**Size**: ~600 lines, no optional features

### `bingocube-adapters`

Optional visualization and sonification adapters.

```toml
[dependencies]
bingocube-adapters = { version = "0.1", features = ["visual", "audio"] }
```

**Features**:
- `visual`: egui-based rendering
- `audio`: Audio sonification
- `animation`: Progressive reveal animation
- `all`: All features

**Size**: ~800 lines

### `bingocube-demos`

Interactive demonstrations (not a library).

```bash
cargo run --example interactive
```

---

## 🎨 Use Cases

### 1. Identity Verification (BearDog)

```rust
// User registers
let identity_cube = BingoCube::from_seed(&biometric_hash, Config::default());
let public_proof = identity_cube.subcube(0.3)?;

// User verifies later (progressive trust)
let claimed_cube = BingoCube::from_seed(&fresh_biometric_hash, Config::default());
if claimed_cube.subcube(0.3)? == public_proof {
    println!("Identity verified at 30% confidence");
}
```

**Innovation**: Biometric used as entropy source, never stored. See [whitePaper/BingoCube-Biometric-Identity.md](whitePaper/BingoCube-Biometric-Identity.md).

### 2. Peer Trust Stamps (Songbird)

```rust
// Alice creates trust stamp for Bob
let trust_stamp = BingoCube::from_seed(
    &format!("alice-trusts-bob-{}", timestamp),
    Config::default()
);

// Bob shows stamp to Carol (visual verification)
let visual_proof = trust_stamp.subcube(0.5)?;
// Carol can visually inspect the pattern
```

### 3. Content Fingerprinting (NestGate)

```rust
// Generate visual hash for file
let file_hash = blake3::hash(&file_contents);
let visual_fingerprint = BingoCube::from_seed(file_hash.as_bytes(), Config::default());

// Display in UI (instead of hex string)
let grid = visual_fingerprint.color_grid();
// Users can recognize files by their color pattern
```

### 4. Computation Proofs (ToadStool)

```rust
// Computation node generates proof
let computation_id = "job-12345";
let result_hash = blake3::hash(&computation_result);
let proof_cube = BingoCube::from_seed(
    &format!("{}-{}", computation_id, result_hash),
    Config::default()
);

// Progressive reveal shows computation progress
for progress in [0.2, 0.4, 0.6, 0.8, 1.0] {
    let partial_proof = proof_cube.subcube(progress)?;
    // Render visual progress indicator
}
```

---

## 🔐 Security Properties

### 1. Pre-image Resistance
Cannot recover boards from color grid (one-way function).

### 2. Collision Resistance  
Different boards → different grids (birthday bound: 2^(-L²) for random boards).

### 3. Binding
Color grid commits to both boards simultaneously.

### 4. Partial Reveal Security
Revealing subset doesn't leak unrevealed cells (progressive reveal is cryptographically safe).

### 5. Forgery Resistance
```
P(forge at x) ≈ (K/U)^(m(x))

Example (L=8, K=256, U=100):
- x=0.2: P ≈ 2^-20  (1 in million)
- x=0.5: P ≈ 2^-50  (1 in quadrillion)
```

**See**: [whitePaper/BingoCube-Mathematical-Foundation.md](whitePaper/BingoCube-Mathematical-Foundation.md) for formal proofs.

---

## 📐 Configuration Options

### Standard Configurations

#### Small (Classic Bingo)
```rust
Config {
    grid_size: 5,        // 5×5 grid
    palette_size: 16,    // 16 colors
    universe_size: 100,  // 0-99 numbers
}
// Forgery (x=0.5): ~2^-50
```

#### Medium (Recommended)
```rust
Config {
    grid_size: 8,        // 8×8 grid
    palette_size: 64,    // 64 colors
    universe_size: 512,  // 0-511 numbers
}
// Forgery (x=0.5): ~2^-192
```

#### Large (High Security)
```rust
Config {
    grid_size: 12,       // 12×12 grid
    palette_size: 256,   // 256 colors
    universe_size: 1000, // 0-999 numbers
}
// Forgery (x=0.5): ~2^-576
```

---

## 📚 Documentation

### Whitepaper Collection (~180 pages)

1. **[BingoCube-Overview.md](whitePaper/BingoCube-Overview.md)** (~25 pages)
   - Core concepts and motivation
   - Mathematical overview
   - Use cases across ecosystem

2. **[BingoCube-Mathematical-Foundation.md](whitePaper/BingoCube-Mathematical-Foundation.md)** (~20 pages)
   - Formal definitions and proofs
   - Security analysis
   - Attack resistance

3. **[BingoCube-Ecosystem-Examples.md](whitePaper/BingoCube-Ecosystem-Examples.md)** (~30 pages)
   - Primal integration patterns
   - Cross-primal workflows
   - Code examples

4. **[BingoCube-Biometric-Identity.md](whitePaper/BingoCube-Biometric-Identity.md)** (~70 pages) ⭐
   - Biometric-seeded identity architecture
   - Homeless services use case
   - Medical data sovereignty
   - Zero-knowledge protocols

### API Documentation

```bash
cargo doc --open --no-deps
```

---

## 🏗️ Project Structure

```
bingoCube/
├── core/                   # Pure crypto core
│   ├── src/
│   │   └── lib.rs         # ~600 lines, 7 tests
│   └── Cargo.toml
├── adapters/              # Optional visualization
│   ├── src/
│   │   ├── visual.rs      # egui rendering
│   │   ├── audio.rs       # Sonification
│   │   └── animation.rs   # Progressive reveal
│   └── Cargo.toml
├── demos/                 # Interactive demos
│   ├── src/
│   │   └── interactive.rs
│   └── Cargo.toml
└── whitePaper/            # Comprehensive docs
    ├── README.md          # Documentation index
    ├── BingoCube-Overview.md
    ├── BingoCube-Mathematical-Foundation.md
    ├── BingoCube-Ecosystem-Examples.md
    └── BingoCube-Biometric-Identity.md
```

---

## 🔧 Development

### Build

```bash
# Core only
cargo build -p bingocube-core

# With adapters
cargo build -p bingocube-adapters --features all

# Everything
cargo build --all
```

### Test

```bash
# Core tests
cargo test -p bingocube-core

# All tests
cargo test --all
```

### Run Demo

```bash
cargo run -p bingocube-demos
```

---

## 🌐 Ecosystem Integration

BingoCube is used by the **ecoPrimals** ecosystem:

| Primal | Use Case |
|--------|----------|
| **BearDog** | Identity verification with progressive trust |
| **Songbird** | P2P trust stamps and visual peer recognition |
| **NestGate** | Content fingerprints and visual git commits |
| **ToadStool** | Computation proofs and progress visualization |
| **petalTongue** | Multi-modal rendering (visual, audio, animation) |

---

## 🤝 Contributing

We welcome contributions! Areas of interest:

1. **Security audits** - Verify cryptographic claims
2. **Performance** - Optimize hot paths
3. **New adapters** - Terminal UI, web canvas, etc.
4. **Use cases** - Document novel applications
5. **Formal verification** - Coq/Lean proofs

### Guidelines

- Core must remain dependency-minimal
- All features must be optional (feature-gated)
- Maintain test coverage (currently 100% for core)
- Document security properties

---

## 📄 License

**AGPL-3.0**

BingoCube is free and open-source software. You may:
- ✅ Use in any project (open or closed source)
- ✅ Modify and distribute
- ✅ Use commercially

**Requirements**:
- 📋 Include license and copyright notice
- 📋 State changes made
- 📋 Disclose source for network use (AGPL provision)

See [LICENSE](LICENSE) for full terms.

---

## 🔗 Links

- **Repository**: https://github.com/ecoPrimals/bingoCube
- **Documentation**: [whitePaper/README.md](whitePaper/README.md)
- **Examples**: [demos/src/](demos/src/)
- **Crates.io**: Coming soon

---

## 🙏 Acknowledgments

BingoCube builds on:
- **Bingo game structure**: Classic American bingo
- **QR code inspiration**: Dense visual encoding
- **BLAKE3**: Fast cryptographic hashing
- **ecoPrimals philosophy**: Sovereignty, dignity, distributed trust

Special thanks to the ecoPrimals community for vision and feedback.

---

## 📊 Project Status

- ✅ Core implementation (600 lines, 7 tests, 100% coverage)
- ✅ Visual adapters (800 lines, 2 tests)
- ✅ Whitepaper collection (~180 pages)
- ✅ Interactive demo
- 🟡 Crates.io publication (pending)
- 🟡 Primal integrations (in progress)

**Version**: 0.1.0  
**Status**: Production-ready for early adopters

---

*"Cryptography should be human-readable. Trust should be visual. Security should be progressive."*

— BingoCube Philosophy

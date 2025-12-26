# BingoCube: Multi-Dimensional Visual Verification System

**Version**: 1.0  
**Date**: December 25, 2025  
**Authors**: ecoPrimals Team

---

## Abstract

BingoCube is a novel visual encoding system that combines structured combinatorics with cryptographic hashing to create verifiable, multi-resolution visual artifacts. By cross-binding two "bingo-style" constraint boards through cryptographic hashing, BingoCube generates QR-like color grids with progressive reveal capabilities controlled by a continuous parameter x ∈ (0,1].

This system bridges human-verifiable visual patterns with cryptographic commitments, enabling applications in identity verification, P2P trust attestation, content addressing, and computation proofs across distributed systems.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Core Concept](#2-core-concept)
3. [Mathematical Foundation](#3-mathematical-foundation)
4. [Use Cases](#4-use-cases)
5. [Visual Examples](#5-visual-examples)
6. [Security Properties](#6-security-properties)
7. [Implementation Considerations](#7-implementation-considerations)
8. [Ecosystem Integration](#8-ecosystem-integration)
9. [Future Directions](#9-future-directions)

---

## 1. Introduction

### 1.1 Motivation

Modern distributed systems require human-verifiable artifacts that are:
- **Structured**: Recognizable patterns for human cognition
- **Secure**: Cryptographically bound to underlying data
- **Progressive**: Gradual reveal for trust building
- **Multi-modal**: Representable through visual, audio, and haptic channels

Traditional approaches (QR codes, checksums, visual hashes) optimize for machine readability at the expense of human comprehension. BingoCube inverts this: it starts with human-recognizable structure (bingo grids) and adds cryptographic binding.

### 1.2 Key Innovation

The core innovation is **two-board cross-binding**:

```
Board A (5×5)          Board B (5×5)          Color Grid (5×5)
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│ 7  23 42 61 84│       │ 3  18 38 57 79│       │ █ █ █ █ █ │
│ 2  29 44 68 75│       │ 9  27 33 63 88│       │ █ █ █ █ █ │
│ 11 16 ✱  52 90│   ×   │ 1  21 ✱  59 72│   →   │ █ █ █ █ █ │
│ 14 25 31 67 81│       │ 12 19 47 54 86│       │ █ █ █ █ █ │
│ 5  30 49 60 77│       │ 8  22 41 66 73│       │ █ █ █ █ █ │
└─────────────┘       └─────────────┘       └─────────────┘

        Hash(i, j, A[i,j], B[i,j]) → Color[i,j]
```

Each cell's color depends on **both** boards, creating a cryptographic commitment to the entire structure.

### 1.3 Progressive Reveal

The parameter x ∈ (0,1] selects nested subsets of cells:

```
x = 0.2 (20% reveal)   x = 0.5 (50% reveal)   x = 1.0 (full reveal)
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│ · · · · ·   │       │ █ · █ · █   │       │ █ █ █ █ █   │
│ · █ · · ·   │       │ █ █ █ █ ·   │       │ █ █ █ █ █   │
│ · · · █ ·   │       │ · █ █ █ █   │       │ █ █ █ █ █   │
│ █ · · · ·   │       │ █ █ · █ ·   │       │ █ █ █ █ █   │
│ · · · · █   │       │ · · █ · █   │       │ █ █ █ █ █   │
└─────────────┘       └─────────────┘       └─────────────┘
```

This creates a **family of nested masks** 𝓜ₓ where x₁ < x₂ ⟹ 𝓜ₓ₁ ⊆ 𝓜ₓ₂.

---

## 2. Core Concept

### 2.1 The "Bingo Constraint"

Traditional bingo enforces structure through **column range locking**:

| Column | Symbol | Range | Example Values |
|--------|--------|-------|----------------|
| 0 | B | 0-19 | {7, 2, 11, 14, 5} |
| 1 | I | 20-39 | {23, 29, 16, 25, 30} |
| 2 | N | 40-59 | {42, 44, ✱, 31, 49} |
| 3 | G | 60-79 | {61, 68, 52, 67, 60} |
| 4 | O | 80-99 | {84, 75, 90, 81, 77} |

**Properties**:
- Each column has distinct values from its range
- Order within column matters (row index is significant)
- Optional free space (center cell ✱)
- Column symbols can be permuted

### 2.2 Two-Board Structure

BingoCube uses **two independent boards** (A and B), each following bingo constraints:

```
Board A: Generated with range [0, U-1], split into L columns
Board B: Generated with range [0, U-1], split into L columns
```

**Key properties**:
- Same universe size U
- Same grid dimensions L×L
- Independent random generation
- Both follow column range locking
- Can have different column permutations

### 2.3 Cryptographic Cross-Binding

For each cell (i,j), compute a **binding scalar**:

```
d[i,j] = Hash("BINGOCUBE_V1" || i || j || A[i,j] || B[i,j])
```

This hash:
- **Binds both boards**: changing either A or B changes d[i,j]
- **Position-aware**: different positions have different hashes
- **Deterministic**: same inputs → same output
- **Uniformly distributed**: no structure leakage from bingo constraints

### 2.4 Color Mapping

Convert hash to visual color:

```
c[i,j] = d[i,j] mod K    (where K = palette size)
```

Result: A single L×L color grid that represents both boards.

---

## 3. Mathematical Foundation

### 3.1 Parameter Space

| Parameter | Symbol | Type | Description |
|-----------|--------|------|-------------|
| Grid size | L | ℕ⁺ | Width/height of square grid |
| Depth | D | ℕ⁺ | Number of layers (here D=2) |
| Universe | U | ℕ⁺ | Number space size |
| Range size | R | ℕ⁺ | R = U/L (per-column range) |
| Palette | K | ℕ⁺ | Number of colors |
| Reveal param | x | (0,1] | Continuous reveal parameter |

**Standard configurations**:
- Small: L=5, U=100, K=16 (classic bingo)
- Medium: L=8, U=512, K=64
- Large: L=12, U=1000, K=256

### 3.2 Board Generation

For each board (A or B):

**Input**: L, U, optional column permutation π

**Output**: L×L grid with bingo constraints

**Algorithm**:
```
1. Compute R = U/L
2. For each column j ∈ {0..L-1}:
   a. Define range: [j·R, (j+1)·R - 1]
   b. Select L distinct values from range (or L-1 if free cell)
   c. Assign to rows in random order
3. Apply column permutation π (shuffle column order)
```

**Constraint satisfaction**:
- ∀j: values in column j are distinct
- ∀j: values in column j ∈ [j·R, (j+1)·R - 1]
- Optional: one free cell per board (traditional center, or any position)

### 3.3 Hash-Based Scalar Field

**Definition**: Scalar field d: [0,L-1]² → ℤ₊

```
d[i,j] = H("BINGOCUBE_V1" || i || j || A[i,j] || B[i,j])
```

Where H is a cryptographic hash (BLAKE3, SHA-256, etc.) interpreted as unsigned integer.

**Properties**:
1. **Deterministic**: d[i,j] is reproducible from (A, B, i, j)
2. **Uniform**: d[i,j] values are uniformly distributed
3. **Collision-resistant**: Different (A,B) pairs → different d field (w.h.p.)
4. **Binding**: Cannot change A or B without changing d
5. **Position-dependent**: Moving values changes d[i,j]

### 3.4 Progressive Reveal Mathematics

**Mask function**: 𝓜: (0,1] → 𝒫([0,L-1]²)

```
m(x) = ⌈x · L²⌉              (number of cells to reveal)

𝓜ₓ = top-m(x) cells by d[i,j] value
```

**Key properties**:

1. **Nested**: x₁ < x₂ ⟹ 𝓜ₓ₁ ⊆ 𝓜ₓ₂
2. **Deterministic**: Same (A,B) → same 𝓜ₓ for all x
3. **Smooth**: Small Δx → small change in revealed cells
4. **Complete**: 𝓜₁ = all cells (full grid)

**Subcube at level x**:
```
Subcube(x) = {(i, j, c[i,j]) : (i,j) ∈ 𝓜ₓ}
```

### 3.5 Information-Theoretic Properties

**Board entropy**:
```
H(Board) ≈ log₂(∏ⱼ (R choose L)) ≈ L² log₂(R)
```

For L=5, R=20: H ≈ 25 × 4.32 ≈ 108 bits per board

**Cross-binding entropy**:
```
H(d-field) ≈ L² × hash_output_bits
```

For 256-bit hash: H ≈ 6400 bits total

**Progressive reveal information**:
```
I(x) = x · H(full_grid)    (information revealed at parameter x)
```

---

## 4. Use Cases

### 4.1 Identity Verification (BearDog)

**Scenario**: Visual identity proof without revealing full credentials

**Implementation**:
- Board A: Derived from identity seed
- Board B: Derived from timestamp + challenge
- x parameter: Trust level (0.2 for initial, 1.0 for full verification)

**Flow**:
```
1. Alice generates BingoCube from her identity
2. Alice shows x=0.2 reveal (20% of cells)
3. Bob challenges with different x values
4. Alice reveals requested subcubes
5. Bob verifies consistency across x levels
```

**Security**: Cannot forge subcubes without knowing both boards

**Example** (L=5, 25 cells total):
- x=0.2 (5 cells): "I'm probably Alice"
- x=0.5 (13 cells): "I'm definitely Alice"
- x=1.0 (25 cells): "Full identity commitment"

### 4.2 P2P Trust Stamps (Songbird)

**Scenario**: Visual representation of P2P connection trust

**Implementation**:
- Board A: Peer's identity hash
- Board B: Connection history hash (uptime, bandwidth, reliability)
- x parameter: Trust score (computed from metrics)

**Flow**:
```
1. Peer connection established
2. Generate BingoCube (A=peer_id, B=history)
3. Display with x = trust_score
4. As trust grows, x increases → more cells reveal
5. Visual pattern becomes recognizable
```

**Visual trust levels**:
```
x=0.1 (new peer)       x=0.5 (established)    x=0.9 (highly trusted)
█ · · · ·              █ █ · █ █              █ █ █ █ █
· · · · ·              █ · █ · █              █ █ █ █ █
· · · · ·              · █ █ █ ·              █ █ █ █ █
· · · · ·              █ · █ · █              █ █ █ █ █
· · · · ·              █ █ · █ █              █ █ █ █ █
```

**Benefit**: Trust is **visually recognizable** and **progressively revealed**

### 4.3 Content Addressing (NestGate)

**Scenario**: Visual hash for content verification

**Implementation**:
- Board A: Content hash (first L² values)
- Board B: Metadata hash (size, timestamp, permissions)
- x parameter: Redundancy level

**Flow**:
```
1. Store content in NestGate
2. Generate BingoCube from content + metadata
3. Display at x=1.0 as full commitment
4. For quick verification, show x=0.3 (30% of cells)
5. Verify partial reveals match stored artifact
```

**Use case: Git-like commits**:
```
Commit 1a2b3c: █████    (x=0.2 reveal)
               ·····
               ██·██
               ·····
               ·····
```

Users can **visually recognize** commits by their partial BingoCube patterns.

### 4.4 Computation Proof (ToadStool)

**Scenario**: Visual proof of computation completion

**Implementation**:
- Board A: Input data hash
- Board B: Output data hash
- x parameter: Computation progress (0→1 as computation proceeds)

**Flow**:
```
1. Submit computation to ToadStool
2. Generate initial BingoCube (B=null → random, x=0)
3. As computation progresses: x increases
4. On completion: B=output_hash, x=1.0
5. Progressive reveal shows computation advancement
```

**Visual progress**:
```
t=0% (starting)    t=50% (halfway)    t=100% (complete)
· · · · ·          █ · █ · ·          █ █ █ █ █
· · · · ·          · █ · █ ·          █ █ █ █ █
· · · · ·          █ · █ · █          █ █ █ █ █
· · · · ·          · █ · █ ·          █ █ █ █ █
· · · · ·          · · █ · █          █ █ █ █ █
```

**Benefit**: Users can **see computation progressing** in real-time

---

## 5. Visual Examples

### 5.1 Simple Example (L=3, U=9, K=4)

**Board A**:
```
Column ranges: [0-2], [3-5], [6-8]
┌─────────┐
│ 0  4  7 │
│ 2  3  8 │
│ 1  5  6 │
└─────────┘
```

**Board B**:
```
Column ranges: [0-2], [3-5], [6-8]
┌─────────┐
│ 1  5  8 │
│ 0  4  6 │
│ 2  3  7 │
└─────────┘
```

**Cross-hash scalar field** (showing d[i,j] mod 100):
```
┌─────────┐
│ 47 89 23│
│ 15 62 91│
│ 38 74 56│
└─────────┘
```

**Color grid** (K=4 colors: 🟦🟩🟨🟥):
```
┌─────────┐
│ 🟥 🟩 🟥│  (47%4=3, 89%4=1, 23%4=3)
│ 🟥 🟦 🟥│  (15%4=3, 62%4=2, 91%4=3)
│ 🟦 🟦 🟦│  (38%4=2, 74%4=2, 56%4=0)
└─────────┘
```

**Progressive reveals**:

x=0.33 (top 3 cells by d[i,j]):
```
┌─────────┐
│ ·  🟩 · │  (89, 91, 74 are top 3)
│ ·  ·  🟥│
│ ·  🟦 · │
└─────────┘
```

x=0.67 (top 6 cells):
```
┌─────────┐
│ 🟥 🟩 · │
│ ·  🟦 🟥│
│ ·  🟦 🟦│
└─────────┘
```

x=1.0 (all 9 cells):
```
┌─────────┐
│ 🟥 🟩 🟥│
│ 🟥 🟦 🟥│
│ 🟦 🟦 🟦│
└─────────┘
```

### 5.2 Identity Example (L=5, classic bingo)

**Alice's identity BingoCube**:

Board A: Derived from `ALICE_SEED_2024`
Board B: Derived from `TIMESTAMP_20241225`

```
Full reveal (x=1.0):
┌───────────────────┐
│ 🟦 🟨 🟥 🟩 🟦 🟨│
│ 🟩 🟥 🟦 🟨 🟩 🟥│
│ 🟨 🟦 ✱  🟥 🟨 🟦│
│ 🟥 🟩 🟨 🟦 🟥 🟩│
│ 🟦 🟨 🟥 🟩 🟦 🟨│
└───────────────────┘
```

**Challenge-response at x=0.4 (10 cells)**:
```
┌───────────────────┐
│ 🟦 ·  🟥 ·  🟦 · │
│ ·  🟥 ·  ·  🟩 · │
│ 🟨 ·  ✱  🟥 ·  · │
│ ·  🟩 ·  ·  🟥 · │
│ 🟦 ·  🟥 ·  ·  · │
└───────────────────┘
```

**Verification**: Alice must reveal **exactly these 10 cells** with these colors. Cannot be forged without knowing both boards.

---

## 6. Security Properties

### 6.1 Commitment Binding

**Property**: The color grid is a cryptographic commitment to both boards.

**Proof sketch**:
- Changing any value in A or B changes d[i,j] for that cell
- d[i,j] is cryptographic hash → uniformly distributed
- c[i,j] = d[i,j] mod K → changes unpredictably
- Finding collision requires breaking hash function

**Implication**: Cannot forge a valid BingoCube without knowing A and B.

### 6.2 Progressive Reveal Security

**Property**: Revealing 𝓜ₓ doesn't compromise 𝓜ₓ′ for x′ > x.

**Reason**:
- Each cell's d[i,j] depends on full hash output
- Revealing subset doesn't leak information about unrevealed cells
- Hash pre-image resistance prevents working backwards

**Implication**: Can safely reveal low-x subcubes without compromising high-x security.

### 6.3 Structure Hiding

**Property**: Bingo structure (column ranges) is hidden by hashing.

**Analysis**:
- Raw boards have detectable structure (column ranges)
- Hash mixing destroys this structure
- d[i,j] values appear uniformly random
- Color distribution appears uniform (if K divides hash space evenly)

**Implication**: Adversary cannot extract board values from color grid alone.

### 6.4 Verification Without Reveal

**Property**: Can verify BingoCube validity without seeing boards.

**Protocol**:
```
1. Prover generates (A, B) and publishes color grid
2. Verifier challenges with random x
3. Prover reveals 𝓜ₓ subset
4. Verifier recomputes d[i,j] for revealed cells
5. Verifier checks colors match
```

**Security**: Prover cannot cheat unless they know both boards that generate the grid.

---

## 7. Implementation Considerations

### 7.1 Hash Function Choice

**Requirements**:
- Cryptographic security (collision resistance, pre-image resistance)
- Fast computation
- Uniform output distribution

**Recommendations**:
- **BLAKE3**: Fastest, parallelizable, modern
- **SHA-256**: Widely available, battle-tested
- **SHA-3**: NIST standard, alternative to SHA-2

**Output encoding**:
```rust
let hash_bytes = blake3::hash(input);
let d_ij = u64::from_le_bytes(hash_bytes[0..8]);
```

### 7.2 Color Palette Design

**Considerations**:
- **Accessibility**: Color-blind friendly palettes
- **Print**: Works in grayscale/low-fi printing
- **Cultural**: Avoid colors with specific cultural meanings

**Suggested palettes**:

K=4 (minimal):
```
🟦 Blue   (Primary, trust)
🟩 Green  (Secondary, success)
🟨 Yellow (Tertiary, caution)
🟥 Red    (Accent, alert)
```

K=16 (balanced):
- Use perceptually uniform color space (LAB, HSLuv)
- Ensure minimum contrast ratios
- Test with color-blindness simulators

K=256 (full):
- Use grayscale or heatmap gradient
- Better for machine processing than human viewing

### 7.3 Board Generation Randomness

**Critical**: Board generation must use cryptographically secure randomness.

**Good**:
```rust
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

let mut rng = OsRng;
column_values.shuffle(&mut rng);
```

**Bad**:
```rust
// DON'T USE: Predictable seed
let mut rng = StdRng::seed_from_u64(timestamp);
```

**For deterministic generation** (e.g., from seed):
```rust
use blake3;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

let seed = blake3::hash(b"USER_SEED_DATA");
let mut rng = ChaCha20Rng::from_seed(seed.into());
```

### 7.4 x Parameter Quantization

For UI/API purposes, quantize x to reasonable steps:

**Coarse** (10 levels):
```
x ∈ {0.1, 0.2, 0.3, ..., 1.0}
```

**Fine** (100 levels):
```
x ∈ {0.01, 0.02, 0.03, ..., 1.0}
```

**Adaptive** (based on grid size):
```
step = 1/L²  (one cell at a time)
```

---

## 8. Ecosystem Integration

### 8.1 petalTongue Visualization

BingoCube is **ideal** for petalTongue's multi-modal capabilities:

**Visual Modality**:
- Display L×L color grid
- Animate x parameter (0→1 progressive reveal)
- Show nested masks 𝓜ₓ
- Toggle between Board A, Board B, cross-matrix views

**Audio Modality**:
- Map colors to tones/instruments
- Sonify reveal progression (ascending tones as x increases)
- Audible "ping" when high-value d[i,j] cells appear
- Different soundscapes for different boards

**Animation Modality**:
- Smooth x sweep animation
- Flow particles for progressive reveal
- Pulse effects on newly revealed cells
- Layer transition animations

### 8.2 BearDog Integration

**Identity Cards**:
```rust
struct IdentityCard {
    board_a: Board,  // From identity seed
    board_b: Board,  // From credential data
    trust_level: f64, // Maps to x parameter
}
```

**Visual ID**:
- QR-like but human-recognizable
- Progressive trust reveal (start at x=0.2, grow to x=1.0)
- Challenge-response for verification

### 8.3 Songbird Integration

**P2P Trust Stamps**:
```rust
struct PeerStamp {
    peer_id_board: Board,
    history_board: Board,
    trust_score: f64,  // Computed from metrics
}
```

**Federation Trust**:
- Each federation tower has a BingoCube
- x parameter = federation health/trust
- Visual recognition of trusted peers

### 8.4 NestGate Integration

**Content Fingerprints**:
```rust
struct ContentFingerprint {
    content_hash_board: Board,
    metadata_board: Board,
    redundancy_level: f64,
}
```

**Visual Git Commits**:
- Each commit has recognizable BingoCube pattern
- x=0.3 reveals enough to identify commit
- Full x=1.0 for complete verification

### 8.5 ToadStool Integration

**Computation Proofs**:
```rust
struct ComputationProof {
    input_board: Board,
    output_board: Board,
    progress: f64,  // 0.0 (start) → 1.0 (complete)
}
```

**Progress Visualization**:
- x increases as computation proceeds
- Visual feedback for long-running tasks
- Final BingoCube as computation receipt

---

## 9. Future Directions

### 9.1 Error Correction

Add redundancy for noisy channels:
- Reserve cells for check values
- Use Reed-Solomon or LDPC codes
- Enable partial recovery from damaged grids

### 9.2 Hierarchical BingoCubes

Multi-scale structure:
- L=3 "macro" grid
- Each macro cell contains L=3 "micro" grid
- Progressive zoom: x controls both levels

### 9.3 Animated BingoCubes

Time-varying grids:
- Board B changes over time
- Creates "living" visual pattern
- One-time-password style (new grid every 30s)

### 9.4 3D BingoCubes

Extend to depth D>2:
- Multiple layers (A, B, C, ...)
- Volumetric visualization
- VR/AR applications

### 9.5 Quantum-Resistant Variants

Prepare for post-quantum:
- Use quantum-resistant hash functions
- Lattice-based constructions
- Maintain visual properties

---

## 10. Conclusion

BingoCube bridges human-recognizable patterns with cryptographic security. By combining:

1. **Structured generation** (bingo constraints)
2. **Cryptographic binding** (cross-board hashing)
3. **Progressive reveal** (continuous x parameter)
4. **Multi-modal representation** (visual, audio, haptic)

...it enables a new class of human-verifiable, machine-checkable artifacts for distributed systems.

The system is particularly well-suited to the ecoPrimals ecosystem, where primal sovereignty, human dignity, and universal access are core values. BingoCube makes cryptographic commitments **visible, recognizable, and progressive**—transforming abstract hashes into tangible, trustable patterns.

---

## Appendix A: Complete Small Example

**Parameters**: L=3, U=9, K=4, D=2

**Board A** (column ranges [0-2], [3-5], [6-8]):
```
┌─────────┐
│ 1  3  7 │
│ 0  5  6 │
│ 2  4  8 │
└─────────┘
```

**Board B**:
```
┌─────────┐
│ 2  4  8 │
│ 1  3  6 │
│ 0  5  7 │
└─────────┘
```

**Hash computation** (using BLAKE3, showing first 2 bytes):

| Cell | Input | Hash (hex) | d mod 100 |
|------|-------|------------|-----------|
| (0,0) | "V1\|0\|0\|1\|2" | 3FA2... | 62 |
| (0,1) | "V1\|0\|1\|3\|4" | 8B45... | 69 |
| (0,2) | "V1\|0\|2\|7\|8" | D312... | 18 |
| (1,0) | "V1\|1\|0\|0\|1" | 4C89... | 73 |
| ... | ... | ... | ... |

**Color grid** (K=4: 🟦=0, 🟩=1, 🟨=2, 🟥=3):
```
┌─────────┐
│ 🟦 🟩 🟦│
│ 🟩 🟥 🟨│
│ 🟨 🟦 🟥│
└─────────┘
```

**Progressive reveals**:
- x=0.22 (2 cells): Show cells with d[i,j] in top 2
- x=0.56 (5 cells): Show cells with d[i,j] in top 5  
- x=1.00 (9 cells): Show all cells

---

## Appendix B: API Surface

Proposed API for BingoCube library:

```rust
pub struct BingoCube {
    board_a: Board,
    board_b: Board,
    scalar_field: Vec<Vec<u64>>,
    config: Config,
}

impl BingoCube {
    // Generate from seed
    pub fn from_seed(seed: &[u8], config: Config) -> Self;
    
    // Generate from explicit boards
    pub fn from_boards(board_a: Board, board_b: Board) -> Self;
    
    // Get color grid
    pub fn color_grid(&self) -> ColorGrid;
    
    // Get subcube at reveal level x
    pub fn subcube(&self, x: f64) -> SubCube;
    
    // Verify subcube
    pub fn verify_subcube(&self, subcube: &SubCube, x: f64) -> bool;
    
    // Export for verification
    pub fn to_commitment(&self) -> Commitment;
}
```

---

**End of Overview Document**

*For implementation details, see: `BingoCube-Implementation.md`*  
*For mathematical proofs, see: `BingoCube-Security-Analysis.md`*  
*For use case patterns, see: `BingoCube-Ecosystem-Patterns.md`*


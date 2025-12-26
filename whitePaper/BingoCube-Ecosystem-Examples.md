# BingoCube: Ecosystem Integration Examples

**Version**: 1.0  
**Date**: December 25, 2025  
**Authors**: ecoPrimals Team

---

## Table of Contents

1. [BearDog: Identity & Security](#1-beardog-identity--security)
2. [Songbird: P2P Trust & Discovery](#2-songbird-p2p-trust--discovery)
3. [NestGate: Content & Storage](#3-nestgate-content--storage)
4. [ToadStool: Compute & Verification](#4-toadstool-compute--verification)
5. [petalTongue: Multi-Modal Visualization](#5-petaltongue-multi-modal-visualization)
6. [Cross-Primal Workflows](#6-cross-primal-workflows)

---

## 1. BearDog: Identity & Security

### 1.1 Visual Identity Cards

**Use Case**: Human-verifiable digital identity without centralized authority

**Structure**:
```rust
struct IdentityBingoCube {
    board_a: Board,  // From identity seed (permanent)
    board_b: Board,  // From context (timestamp, challenge, nonce)
    trust_level: f64, // Maps to x reveal parameter
    metadata: IdentityMetadata,
}

struct IdentityMetadata {
    pub_key_hash: [u8; 32],
    created_at: Timestamp,
    domain: String,  // "ecoPrimals", "BearDog", etc.
}
```

**Board Generation**:
```
Board A (Identity):
  seed = BLAKE3(user_seed || "IDENTITY" || pub_key)
  RNG = ChaCha20Rng::from_seed(seed)
  board_a = generate_board(L=5, U=100, RNG)

Board B (Context):
  seed = BLAKE3(timestamp || challenge || nonce)
  RNG = ChaCha20Rng::from_seed(seed)
  board_b = generate_board(L=5, U=100, RNG)
```

### 1.2 Progressive Trust Protocol

**Scenario**: Alice proves her identity to Bob with progressive reveal

**Protocol**:
```
1. Initial Contact (x=0.2):
   Alice: "I am alice@ecoprimals.bio"
   Alice generates: BingoCube(A=identity, B=timestamp)
   Alice shows: 20% reveal (5 cells)
   
   Visual:
   ┌───────────┐
   │ · · 🟥 · ·│
   │ · 🟦 · · ·│
   │ · · ✱ 🟨 ·│
   │ 🟩 · · · ·│
   │ · · · · ·│
   └───────────┘

2. Challenge (x=0.5):
   Bob: "Prove it - show x=0.5"
   Alice reveals: 13 cells
   
   Visual:
   ┌───────────┐
   │ 🟦 · 🟥 🟨 ·│
   │ · 🟦 🟩 · 🟥│
   │ 🟨 · ✱ 🟨 🟦│
   │ 🟩 🟥 · 🟦 ·│
   │ · 🟨 🟩 · ·│
   └───────────┘
   
   Bob verifies: Cells from x=0.2 are still present and correct

3. Full Verification (x=1.0):
   Bob: "Show full identity"
   Alice reveals: All 25 cells
   Bob verifies: Complete consistency
```

**Security**:
- Can't forge partial reveal without knowing Board A (identity seed)
- Progressive reveal builds trust incrementally
- Each challenge uses different Board B (timestamp) → fresh challenge
- Forgery probability: ~2^(-32) at x=0.3, ~2^(-50) at x=0.5

### 1.3 Multi-Factor Identity

**Enhanced Protocol**: Combine multiple BingoCubes

```rust
struct MultiFactorIdentity {
    who: BingoCube,     // Board A = identity seed
    what: BingoCube,    // Board A = password hash
    where: BingoCube,   // Board A = device fingerprint
    when: BingoCube,    // Board A = time-based token
}
```

**Visual Representation**: 2×2 grid of BingoCubes

```
WHO (identity)      WHAT (password)
┌───────────┐      ┌───────────┐
│ 🟦 🟥 🟨 🟩 🟦│      │ 🟥 🟨 🟦 🟩 🟥│
│ 🟩 🟦 🟥 🟨 🟩│      │ 🟦 🟥 🟩 🟨 🟦│
│ 🟨 🟥 ✱ 🟦 🟨│      │ 🟩 🟦 ✱ 🟥 🟩│
│ 🟦 🟩 🟨 🟥 🟦│      │ 🟨 🟩 🟦 🟥 🟨│
│ 🟥 🟨 🟦 🟩 🟥│      │ 🟥 🟨 🟩 🟦 🟥│
└───────────┘      └───────────┘

WHERE (device)      WHEN (time token)
┌───────────┐      ┌───────────┐
│ 🟩 🟦 🟥 🟨 🟩│      │ 🟨 🟩 🟦 🟥 🟨│
│ 🟨 🟥 🟦 🟩 🟨│      │ 🟥 🟦 🟩 🟨 🟥│
│ 🟦 🟩 ✱ 🟥 🟦│      │ 🟦 🟨 ✱ 🟩 🟦│
│ 🟥 🟨 🟩 🟦 🟥│      │ 🟩 🟥 🟨 🟦 🟩│
│ 🟦 🟩 🟨 🟥 🟦│      │ 🟨 🟦 🟥 🟩 🟨│
└───────────┘      └───────────┘
```

**Verification**: All 4 cubes must match at requested x level

---

## 2. Songbird: P2P Trust & Discovery

### 2.1 Peer Trust Stamps

**Use Case**: Visual representation of P2P connection quality

**Structure**:
```rust
struct PeerStamp {
    board_a: Board,          // From peer_id
    board_b: Board,          // From connection_history
    trust_score: f64,        // Computed from metrics
    metrics: PeerMetrics,
}

struct PeerMetrics {
    uptime: Duration,
    bandwidth_shared: u64,
    reliability: f64,        // 0.0-1.0
    failed_requests: u32,
    successful_requests: u32,
}
```

**Trust Score Calculation**:
```rust
fn compute_trust_score(metrics: &PeerMetrics) -> f64 {
    let uptime_factor = (metrics.uptime.as_secs() as f64 / 86400.0).min(1.0);
    let reliability_factor = metrics.reliability;
    let success_rate = metrics.successful_requests as f64 
                     / (metrics.successful_requests + metrics.failed_requests).max(1) as f64;
    
    let trust = (uptime_factor * 0.3 + reliability_factor * 0.4 + success_rate * 0.3);
    trust.clamp(0.0, 1.0)
}
```

### 2.2 Progressive Trust Visualization

**Scenario**: Watching peer trust grow over time

**Timeline**:
```
t=0 (new peer, trust=0.1):
┌───────────┐
│ · · · · ·│
│ · 🟦 · · ·│
│ · · ✱ · ·│
│ · · · · ·│
│ · · · · ·│
└───────────┘

t=1 hour (connecting, trust=0.3):
┌───────────┐
│ · · 🟥 · ·│
│ · 🟦 · 🟨 ·│
│ 🟩 · ✱ · ·│
│ · · · 🟦 ·│
│ · · · · ·│
└───────────┘

t=1 day (reliable, trust=0.7):
┌───────────┐
│ 🟦 🟩 🟥 🟨 ·│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 · 🟦│
└───────────┘

t=1 week (highly trusted, trust=0.95):
┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 🟥 🟦│
└───────────┘
```

**User Experience**: Users can **visually recognize** trusted peers by their BingoCube pattern.

### 2.3 Federation Tower Trust

**Use Case**: Multi-tower federation with trust levels

**Structure**: Each tower has a BingoCube

```
Tower Alpha (x=0.9, highly trusted):
┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 · 🟦│  (23/25 cells)
└───────────┘

Tower Beta (x=0.6, moderate trust):
┌───────────┐
│ 🟥 🟨 · 🟩 🟦│
│ · 🟦 🟥 · 🟨│
│ 🟩 · ✱ 🟦 ·│
│ 🟦 🟨 · · 🟥│
│ · 🟩 🟦 🟨 ·│  (15/25 cells)
└───────────┘

Tower Gamma (x=0.2, new/untrusted):
┌───────────┐
│ · · · 🟩 ·│
│ · · · · ·│
│ · · ✱ 🟦 ·│
│ · 🟨 · · ·│
│ · · 🟦 · ·│  (5/25 cells)
└───────────┘
```

**Federation Decision**: Route through towers with x > 0.7

---

## 3. NestGate: Content & Storage

### 3.1 Content Fingerprints

**Use Case**: Visual hash for stored content

**Structure**:
```rust
struct ContentFingerprint {
    board_a: Board,           // From content hash
    board_b: Board,           // From metadata hash
    redundancy: f64,          // 0.0-1.0, maps to x
    metadata: ContentMetadata,
}

struct ContentMetadata {
    content_hash: [u8; 32],
    size: u64,
    mime_type: String,
    timestamp: Timestamp,
    permissions: Permissions,
}
```

**Board Generation**:
```
Board A (Content):
  hash = BLAKE3(file_contents)
  seed = hash[0..32]
  board_a = generate_board(seed, L=5, U=100)

Board B (Metadata):
  meta_bytes = serialize(metadata)
  hash = BLAKE3(meta_bytes)
  seed = hash[0..32]
  board_b = generate_board(seed, L=5, U=100)
```

### 3.2 Visual Git Commits

**Scenario**: Recognize commits by their BingoCube pattern

**Commit History**:
```
Commit 7f3a2b1 "Add BingoCube support":
┌───────────┐
│ 🟦 🟥 🟨 🟩 🟦│
│ 🟩 🟦 🟥 🟨 🟩│
│ 🟨 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟥 🟦│
│ 🟥 🟨 🟦 🟩 🟥│
└───────────┘

Commit 9e8c4d2 "Fix layout bug":
┌───────────┐
│ 🟥 🟨 🟦 🟩 🟥│
│ 🟦 🟩 🟨 🟦 🟨│
│ 🟩 🟦 ✱ 🟥 🟩│
│ 🟨 🟥 🟩 🟨 🟦│
│ 🟦 🟨 🟥 🟩 🟥│
└───────────┘

Commit a1b4c8f "Update docs":
┌───────────┐
│ 🟩 🟦 🟥 🟨 🟩│
│ 🟨 🟥 🟩 🟦 🟨│
│ 🟦 🟩 ✱ 🟨 🟦│
│ 🟥 🟨 🟦 🟩 🟥│
│ 🟨 🟦 🟩 🟥 🟨│
└───────────┘
```

**Benefit**: Developers can **visually recognize** commits without memorizing hex hashes.

### 3.3 Redundancy Visualization

**Use Case**: Show data redundancy/availability via x parameter

```
x=0.3 (30% redundancy - risky):
┌───────────┐
│ · · 🟥 · ·│
│ · 🟦 · 🟨 ·│
│ 🟩 · ✱ · ·│
│ · · · 🟦 ·│
│ · 🟨 🟩 · ·│
└───────────┘
⚠️ Warning: Low redundancy

x=0.7 (70% redundancy - safe):
┌───────────┐
│ 🟦 🟩 🟥 🟨 ·│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 · 🟦│
└───────────┘
✅ Good: High redundancy

x=1.0 (100% redundancy - maximum):
┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 🟥 🟦│
└───────────┘
💎 Perfect: Full redundancy
```

---

## 4. ToadStool: Compute & Verification

### 4.1 Computation Proofs

**Use Case**: Visual proof of computation completion

**Structure**:
```rust
struct ComputationProof {
    board_a: Board,     // From input hash
    board_b: Board,     // From output hash
    progress: f64,      // 0.0-1.0
    metadata: ComputeMetadata,
}

struct ComputeMetadata {
    input_hash: [u8; 32],
    output_hash: [u8; 32],
    started_at: Timestamp,
    completed_at: Option<Timestamp>,
    compute_units: u64,
}
```

### 4.2 Real-Time Progress Visualization

**Scenario**: Long-running computation (e.g., video encoding)

**Timeline**:
```
t=0% (starting):
Board A: Fixed (from input)
Board B: Null → Random
x: 0.0

┌───────────┐
│ · · · · ·│
│ · · · · ·│
│ · · ✱ · ·│
│ · · · · ·│
│ · · · · ·│
└───────────┘
Status: Starting...

t=25% (early progress):
Board A: Fixed
Board B: Partial output
x: 0.25

┌───────────┐
│ · · 🟥 · ·│
│ · 🟦 · · ·│
│ 🟨 · ✱ 🟩 ·│
│ · · · 🟦 ·│
│ · · · · ·│
└───────────┘
Status: Processing... (6/25)

t=50% (halfway):
Board A: Fixed
Board B: More complete
x: 0.50

┌───────────┐
│ 🟦 · 🟥 🟨 ·│
│ · 🟦 🟩 · 🟥│
│ 🟨 · ✱ 🟩 🟦│
│ 🟦 🟩 · 🟦 ·│
│ · 🟨 🟩 · ·│
└───────────┘
Status: Processing... (13/25)

t=100% (complete):
Board A: Fixed (input)
Board B: Fixed (output)
x: 1.0

┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 🟥 🟦│
└───────────┘
Status: ✅ Complete (25/25)
```

**User Experience**: Watch computation "grow" from empty grid to complete pattern.

### 4.3 Result Verification

**Protocol**: Verify computation result without re-running

```rust
struct VerificationChallenge {
    expected_cube: BingoCube,
    challenge_x: f64,  // Random x value
}

fn verify_computation(
    input: &[u8],
    output: &[u8],
    proof: &ComputationProof
) -> Result<bool> {
    // Regenerate expected BingoCube
    let board_a = generate_board_from_hash(input);
    let board_b = generate_board_from_hash(output);
    let expected = BingoCube::from_boards(board_a, board_b);
    
    // Challenge with random x
    let challenge_x = random_f64();
    let expected_subcube = expected.subcube(challenge_x);
    let proof_subcube = proof.cube.subcube(challenge_x);
    
    // Verify match
    Ok(expected_subcube == proof_subcube)
}
```

### 4.4 Distributed Computation

**Use Case**: Multiple workers, combined proof

```
Worker 1 (chunk 0-33%):        Worker 2 (chunk 33-66%):      Worker 3 (chunk 66-100%):
Progress: 100%                 Progress: 100%                Progress: 100%
┌───────────┐                 ┌───────────┐                 ┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│                 │ 🟥 🟨 🟦 🟩 🟥│                 │ 🟩 🟦 🟥 🟨 🟩│
│ 🟨 🟦 🟩 🟨 🟥│                 │ 🟦 🟩 🟨 🟦 🟨│                 │ 🟨 🟥 🟩 🟦 🟨│
│ 🟩 🟥 ✱ 🟦 🟨│                 │ 🟩 🟦 ✱ 🟥 🟩│                 │ 🟦 🟩 ✱ 🟨 🟦│
│ 🟦 🟩 🟨 🟦 🟩│                 │ 🟨 🟥 🟩 🟨 🟦│                 │ 🟥 🟨 🟦 🟩 🟥│
│ 🟥 🟨 🟩 🟥 🟦│                 │ 🟦 🟨 🟥 🟩 🟥│                 │ 🟨 🟦 🟩 🟥 🟨│
└───────────┘                 └───────────┘                 └───────────┘

Combined Proof (all chunks):
┌───────────┐
│ 🟦 🟩 🟥 🟨 🟦│
│ 🟨 🟦 🟩 🟨 🟥│
│ 🟩 🟥 ✱ 🟦 🟨│  ← Generated from HASH(worker1_output || worker2_output || worker3_output)
│ 🟦 🟩 🟨 🟦 🟩│
│ 🟥 🟨 🟩 🟥 🟦│
└───────────┘
```

---

## 5. petalTongue: Multi-Modal Visualization

### 5.1 Visual Modality

**Display Options**:

1. **Single Grid View**: Show color grid at specified x
2. **Dual Board View**: Show Board A and Board B side-by-side
3. **Progressive Animation**: Animate x from 0→1
4. **Interactive Reveal**: User controls x with slider

**Example UI**:
```
┌──────────────────────────────────────────────────┐
│ BingoCube Viewer                           [×]   │
├──────────────────────────────────────────────────┤
│                                                  │
│  Reveal: ████████░░ 80%                         │
│                                                  │
│  ┌───────────────────┐                          │
│  │ 🟦 🟩 🟥 🟨 🟦 🟨  │                          │
│  │ 🟨 🟦 🟩 🟨 🟥 🟦  │                          │
│  │ 🟩 🟥 ✱ 🟦 🟨 🟩  │  20/25 cells visible     │
│  │ 🟦 🟩 🟨 🟦 🟩 🟥  │                          │
│  │ · 🟨 🟩 · 🟦 ·   │                          │
│  └───────────────────┘                          │
│                                                  │
│  [Show Board A] [Show Board B] [Animate]        │
│                                                  │
└──────────────────────────────────────────────────┘
```

### 5.2 Audio Modality

**Sonification Strategy**:

```rust
struct BingoCubeAudioRenderer {
    // Map colors to instruments/notes
    color_to_instrument: HashMap<Color, Instrument>,
    color_to_pitch: HashMap<Color, f32>,
}

impl BingoCubeAudioRenderer {
    fn sonify_cell(&self, cell: (usize, usize, Color)) {
        let (i, j, color) = cell;
        
        // Position → panning
        let pan = (j as f32 / L as f32) * 2.0 - 1.0;  // -1.0 (left) to 1.0 (right)
        
        // Row → pitch offset
        let pitch_offset = (i as f32 / L as f32) * 12.0;  // One octave
        
        // Color → instrument + base pitch
        let instrument = self.color_to_instrument[&color];
        let base_pitch = self.color_to_pitch[&color];
        
        // Play note
        play_note(instrument, base_pitch + pitch_offset, pan);
    }
    
    fn sonify_reveal_animation(&self, cube: &BingoCube, duration: Duration) {
        let steps = 100;
        let step_duration = duration / steps;
        
        for step in 0..=steps {
            let x = (step as f64) / (steps as f64);
            let subcube = cube.subcube(x);
            
            // Play newly revealed cells
            for cell in subcube.newly_revealed_since(prev_x) {
                self.sonify_cell(cell);
                sleep(step_duration);
            }
        }
    }
}
```

**Audio Mapping Example**:
```
🟦 Blue   → Piano, C4 (261.63 Hz)
🟩 Green  → Strings, E4 (329.63 Hz)
🟨 Yellow → Synth, G4 (392.00 Hz)
🟥 Red    → Brass, C5 (523.25 Hz)

Progressive reveal creates ascending melody as x increases
```

### 5.3 Animation Modality

**Animation Strategies**:

1. **Progressive Reveal**: Cells fade in as x increases
2. **Flow Animation**: Particles flow from low-d to high-d cells
3. **Pulse Animation**: Newly revealed cells pulse
4. **Ripple Effect**: Reveal propagates from center outward

**Example Code**:
```rust
struct BingoCubeAnimator {
    animation_state: AnimationState,
    particle_engine: ParticleEngine,
}

impl BingoCubeAnimator {
    fn animate_progressive_reveal(
        &mut self,
        cube: &BingoCube,
        from_x: f64,
        to_x: f64,
        duration: Duration,
    ) {
        let start_time = Instant::now();
        
        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= duration { break; }
            
            // Interpolate x
            let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
            let current_x = from_x + (to_x - from_x) * progress;
            
            // Get subcube at current x
            let subcube = cube.subcube(current_x);
            
            // Render with fade-in for newly revealed cells
            for cell in subcube.cells() {
                let alpha = self.compute_cell_alpha(cell, current_x);
                self.render_cell(cell, alpha);
            }
            
            // Update particle effects
            self.particle_engine.update(elapsed);
            
            sleep(Duration::from_millis(16));  // 60 FPS
        }
    }
}
```

---

## 6. Cross-Primal Workflows

### 6.1 Identity-Verified Content Storage

**Workflow**: BearDog identity → NestGate storage

```
1. Alice creates content
2. Alice's BearDog identity generates BingoCube_identity
3. Content is hashed → BingoCube_content
4. Combined proof: (BingoCube_identity, BingoCube_content)
5. Store in NestGate with combined visual signature

Visual representation:
┌─────────────────┬─────────────────┐
│ Identity        │ Content         │
│ (Alice)         │ (document.pdf)  │
│                 │                 │
│ 🟦 🟩 🟥 🟨 🟦   │ 🟥 🟨 🟦 🟩 🟥   │
│ 🟨 🟦 🟩 🟨 🟥   │ 🟦 🟩 🟨 🟦 🟨   │
│ 🟩 🟥 ✱ 🟦 🟨   │ 🟩 🟦 ✱ 🟥 🟩   │
│ 🟦 🟩 🟨 🟦 🟩   │ 🟨 🟥 🟩 🟨 🟦   │
│ 🟥 🟨 🟩 🟥 🟦   │ 🟦 🟨 🟥 🟩 🟥   │
└─────────────────┴─────────────────┘
         WHO              WHAT
```

### 6.2 Trusted Computation

**Workflow**: Songbird peer discovery → ToadStool compute

```
1. Discover compute peers via Songbird
2. Each peer has trust stamp (BingoCube with x=trust_score)
3. Select peers with x > 0.7 for computation
4. Submit computation to selected peers
5. Each peer returns computation proof (BingoCube)
6. Verify proofs match expected output

Peer Selection UI:
┌────────────────────────────────────────┐
│ Peer: peer1.songbird.local             │
│ Trust: ████████░░ 80%                  │
│ ┌─────────────┐                        │
│ │ 🟦 🟩 🟥 🟨 🟦 │ (20/25 cells)        │
│ │ 🟨 🟦 🟩 🟨 🟥 │                        │
│ │ 🟩 🟥 ✱ 🟦 🟨 │ ✅ Selected            │
│ │ 🟦 🟩 🟨 🟦 🟩 │                        │
│ │ 🟥 🟨 🟩 · 🟦 │                        │
│ └─────────────┘                        │
├────────────────────────────────────────┤
│ Peer: peer2.songbird.local             │
│ Trust: ███░░░░░░░ 30%                  │
│ ┌─────────────┐                        │
│ │ · · 🟥 · ·  │ (8/25 cells)           │
│ │ · 🟦 · 🟨 ·  │                        │
│ │ 🟩 · ✱ · ·  │ ❌ Not selected        │
│ │ · · · 🟦 ·  │                        │
│ │ · 🟨 🟩 · ·  │                        │
│ └─────────────┘                        │
└────────────────────────────────────────┘
```

### 6.3 Federated Identity

**Workflow**: Multi-tower identity verification

```
Alice's identity recognized across 3 Songbird towers:

Tower Alpha:        Tower Beta:         Tower Gamma:
(x=0.9, verified)   (x=0.7, verified)   (x=0.3, pending)

🟦 🟩 🟥 🟨 🟦      🟦 🟩 🟥 🟨 ·      · · 🟥 · ·
🟨 🟦 🟩 🟨 🟥      🟨 🟦 🟩 · 🟥      · 🟦 · 🟨 ·
🟩 🟥 ✱ 🟦 🟨      🟩 🟥 ✱ 🟦 🟨      🟩 · ✱ · ·
🟦 🟩 🟨 🟦 🟩      🟦 🟩 · 🟦 ·      · · · 🟦 ·
🟥 🟨 🟩 · 🟦      🟥 🟨 🟩 · ·      · 🟨 🟩 · ·

✅ Trusted         ✅ Trusted         ⏳ Building trust

Federation decision: Alice is trusted (2/3 towers verify)
```

### 6.4 Content Pipeline with Provenance

**Workflow**: Full lifecycle tracking

```
1. CREATE (BearDog identity):
   Alice creates content
   BingoCube_creator = (A=alice_identity, B=timestamp)

2. STORE (NestGate):
   Content stored with fingerprint
   BingoCube_storage = (A=content_hash, B=metadata_hash)

3. DISTRIBUTE (Songbird):
   Content shared via P2P
   BingoCube_peer = (A=peer_id, B=connection_history)
   
4. PROCESS (ToadStool):
   Content processed (e.g., transcoded)
   BingoCube_compute = (A=input_hash, B=output_hash)

5. VERIFY (petalTongue):
   Visualize entire pipeline
   Show all 4 BingoCubes as provenance chain

Visual Provenance Chain:
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│ Creator  │ → │ Storage  │ → │ P2P Dist │ → │ Compute  │
│          │   │          │   │          │   │          │
│ 🟦 🟩 🟥 🟨│   │ 🟥 🟨 🟦 🟩│   │ 🟩 🟦 🟥 🟨│   │ 🟨 🟩 🟦 🟥│
│ 🟨 🟦 🟩 🟨│   │ 🟦 🟩 🟨 🟦│   │ 🟨 🟥 🟩 🟦│   │ 🟦 🟨 🟥 🟩│
│ 🟩 🟥 ✱ 🟦│   │ 🟩 🟦 ✱ 🟥│   │ 🟦 🟩 ✱ 🟨│   │ 🟥 🟦 ✱ 🟨│
│ 🟦 🟩 🟨 🟦│   │ 🟨 🟥 🟩 🟨│   │ 🟥 🟨 🟦 🟩│   │ 🟩 🟥 🟨 🟦│
│ 🟥 🟨 🟩 🟥│   │ 🟦 🟨 🟥 🟩│   │ 🟨 🟦 🟩 🟥│   │ 🟨 🟩 🟦 🟥│
└──────────┘   └──────────┘   └──────────┘   └──────────┘
   Alice        NestGate       Songbird       ToadStool
```

Each step has a **recognizable visual signature**, creating a **visual audit trail**.

---

## 7. Comparison Table

| Primal | Use Case | Board A | Board B | x Meaning | Visual Benefit |
|--------|----------|---------|---------|-----------|----------------|
| **BearDog** | Identity | Identity seed | Challenge/timestamp | Trust level | Progressive reveal for verification |
| **Songbird** | P2P Trust | Peer ID | Connection history | Trust score | Visual recognition of trusted peers |
| **NestGate** | Content | Content hash | Metadata hash | Redundancy | Visual commit/file fingerprints |
| **ToadStool** | Compute | Input hash | Output hash | Progress | Real-time computation visualization |
| **petalTongue** | Visualization | Any Board A | Any Board B | Custom reveal | Multi-modal representation |

---

## 8. Summary

BingoCube provides a **universal visual language** for the ecoPrimals ecosystem:

1. **Human-Verifiable**: People can recognize patterns (unlike hex hashes)
2. **Progressive**: Trust/information builds gradually via x parameter
3. **Cross-Primal**: Same visual system works across all primals
4. **Multi-Modal**: petalTongue renders as visual, audio, or haptic
5. **Secure**: Cryptographically bound to underlying data

**The result**: A distributed system where trust, identity, content, and computation are all **visible, recognizable, and verifiable** by humans, not just machines.

---

**Next**: See implementation guide for building BingoCube into your primal.


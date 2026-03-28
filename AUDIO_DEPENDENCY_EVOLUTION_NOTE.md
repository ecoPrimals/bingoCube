# 🎵 Audio Architecture Evolution Note

**Date**: January 3, 2026  
**From**: petalTongue Team  
**To**: BingoCube Team

---

## 🎯 Audio Dependency Evolution

### Current Situation

BingoCube's audio adapter (`bingocube-adapters` with `audio` feature) currently depends on `rodio` → `cpal` → `alsa-sys`, which requires:
- `libasound2-dev` on Linux
- Platform-specific audio libraries
- Creates build dependency for consumers

### Impact on Primal Integration

When primals (like petalTongue) integrate BingoCube:
```toml
bingocube-adapters = { ..., features = ["visual", "audio"] }
```

They inherit ALL audio dependencies, even if they have their own audio system.

### Recommended Evolution

**Phase 1: Feature Separation** ✅ (Current)
```toml
# Primals can choose what they need
bingocube-adapters = { ..., features = ["visual"] }  # No audio deps
```

**Phase 2: Audio Interface (Recommended)**
```rust
// Define audio trait, not implementation
pub trait BingoCubeAudio {
    fn describe_soundscape(&self, reveal: f64) -> String;
    fn get_audio_data(&self) -> Vec<f32>;  // Raw samples
}

// Let consumer provide playback
impl BingoCubeAudio for MyAudioSystem {
    fn get_audio_data(&self) -> Vec<f32> {
        // Generate samples, consumer plays them
    }
}
```

**Phase 3: Full Separation (Ideal)**
```
BingoCube:     Generates audio DATA (samples, frequencies)
Consumer:      Handles PLAYBACK (rodio, toadstool, pure Rust, etc.)
```

### Benefits of Evolution

1. **Zero Build Dependencies**
   - BingoCube compiles everywhere
   - No platform-specific libraries required
   - Faster CI/CD

2. **Flexibility for Consumers**
   - petalTongue: Use pure Rust tones
   - Another primal: Use toadstool
   - Desktop app: Use rodio
   - Web app: Use Web Audio API

3. **Separation of Concerns**
   - BingoCube: Data generation & visualization
   - Audio system: Playback & synthesis
   - Each does what it's best at

4. **Cross-Platform by Default**
   - Core logic works everywhere
   - Audio is optional enhancement
   - Graceful degradation

### Example: How petalTongue Does It

```rust
// petalTongue generates its own audio (pure Rust)
pub fn generate_bingocube_sound(cube: &BingoCube, reveal: f64) -> Vec<f32> {
    // Use BingoCube data to create tones
    // Play with petalTongue's audio system
}

// OR delegate to toadstool
pub async fn request_audio_from_toadstool(cube_data: &CubeData) -> Result<Audio> {
    // toadstool handles advanced synthesis
}
```

### Implementation Path

**Option A: Make audio feature truly optional**
```toml
[features]
default = []
visual = []
audio = ["rodio", "cpal"]  # Only if explicitly enabled
```

**Option B: Provide audio data interface**
```rust
pub struct BingoCubeAudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub description: String,
}

impl BingoCube {
    pub fn generate_audio_data(&self, reveal: f64) -> BingoCubeAudioData {
        // Generate samples without playing them
    }
}
```

**Option C: Document external audio integration**
```markdown
# BingoCube Audio Integration

BingoCube generates audio data. To play it:

1. Pure Rust: Use mathematical waveforms
2. Toadstool: Advanced synthesis
3. rodio: Cross-platform playback
4. Platform-specific: Use native APIs
```

### Current Status (petalTongue)

- **Removed**: `features = ["visual", "audio"]`
- **Using**: `features = ["visual"]` only
- **Audio**: Handled by petalTongue's own pure Rust system
- **Result**: No ALSA dependencies, builds everywhere

### Questions for BingoCube Team

1. Is audio generation core to BingoCube, or an adapter?
2. Would separating audio data from playback work?
3. Can we make audio feature truly optional (no transitive deps)?

### Resources

- petalTongue's pure Rust audio: `petalTongue/crates/petal-tongue-ui/src/audio_pure_rust.rs`
- Audio provider system: `petalTongue/crates/petal-tongue-ui/src/audio_providers.rs`
- Zero-dependency approach: Generate WAV bytes, consumer plays

---

## 🤝 Primal Principle

**"Primals have self-knowledge and discover others at runtime"**

Applied to audio:
- BingoCube knows: "I can generate audio DATA"
- Consumer discovers: "I have audio PLAYBACK capability"
- Integration: Data flows to playback, no hard dependencies

---

**Status**: Informational  
**Priority**: Medium (improves integration experience)  
**Impact**: Removes build barriers for consumers  

🎵 **Separation of concerns = Better primal ecosystem** 🎵


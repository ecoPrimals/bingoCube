# BingoCube Repository Extraction Plan

**Date**: December 26, 2025  
**Target Repo**: `git@github.com:ecoPrimals/bingoCube.git`  
**Current Location**: `/home/eastgate/Development/ecoPrimals/phase2/petalTongue/bingoCube/`  
**Future Location**: `/home/eastgate/Development/ecoPrimals/phase2/bingoCube/` (parallel to petalTongue)

---

## 🎯 Objective

Extract BingoCube from its nested location within petalTongue to become an independent, standalone repository at the same level as other primal repositories.

**Why**: BingoCube is a **tool**, not part of petalTongue. Any primal can use it. It should be:
- ✅ Independent repository
- ✅ Versioned separately
- ✅ Distributable via crates.io or git dependency
- ✅ Clear licensing and ownership
- ✅ Easy for other primals to adopt

---

## 📦 What to Extract

### Current Structure (Nested)
```
petalTongue/
├── bingoCube/                 ← Extract this entire directory
│   ├── core/                  ← Pure crypto tool
│   ├── adapters/              ← Visual/audio adapters
│   ├── demos/                 ← Interactive demonstrations
│   └── whitePaper/            ← Mathematical foundations (4 docs, 156K)
├── crates/
│   └── petal-tongue-ui/
│       └── src/
│           └── bingocube_integration.rs  ← Keep in petalTongue (adapter pattern)
```

### Future Structure (Parallel)
```
ecoPrimals/phase2/
├── petalTongue/               ← Visualization primal
│   └── crates/
│       └── petal-tongue-ui/
│           └── src/
│               └── bingocube_integration.rs  ← Uses bingoCube as dependency
│
├── bingoCube/                 ← Standalone tool (NEW REPO)
│   ├── core/
│   ├── adapters/
│   ├── demos/
│   ├── whitePaper/
│   ├── README.md
│   ├── LICENSE
│   └── Cargo.toml
```

---

## 🚀 Extraction Steps

### Phase 1: Prepare Standalone Repository

```bash
# 1. Navigate to parent directory
cd /home/eastgate/Development/ecoPrimals/phase2/

# 2. Copy BingoCube out of petalTongue
cp -r petalTongue/bingoCube/ ./bingoCube/

# 3. Initialize git repository
cd bingoCube/
git init
git remote add origin git@github.com:ecoPrimals/bingoCube.git

# 4. Create root-level files
touch README.md LICENSE .gitignore

# 5. Verify structure
tree -L 2
```

### Phase 2: Update Cargo Workspace

Create root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "core",
    "adapters",
    "demos",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0"
authors = ["ecoPrimals Team"]
repository = "https://github.com/ecoPrimals/bingoCube"

[workspace.dependencies]
bingocube-core = { path = "core", version = "0.1.0" }
bingocube-adapters = { path = "adapters", version = "0.1.0" }
blake3 = "1.5"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

### Phase 3: Update Individual Cargo.toml Files

#### `core/Cargo.toml`:
```toml
[package]
name = "bingocube-core"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "Pure cryptographic tool for human-verifiable commitments"

[dependencies]
blake3.workspace = true
serde.workspace = true
thiserror.workspace = true
rand = "0.8"

[dev-dependencies]
criterion = "0.5"
```

#### `adapters/Cargo.toml`:
```toml
[package]
name = "bingocube-adapters"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "Optional visualization adapters for BingoCube"

[dependencies]
bingocube-core.workspace = true
serde.workspace = true

[dependencies.egui]
version = "0.28"
optional = true

[features]
default = []
visual = ["egui"]
audio = []
animation = []
all = ["visual", "audio", "animation"]
```

### Phase 4: Create Comprehensive README

```markdown
# 🎲 BingoCube

**Human-Verifiable Cryptographic Commitment System**

[![Crates.io](https://img.shields.io/crates/v/bingocube-core.svg)](https://crates.io/crates/bingocube-core)
[![Documentation](https://docs.rs/bingocube-core/badge.svg)](https://docs.rs/bingocube-core)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)

> A pure tool for creating memorable, visual, and auditory cryptographic patterns.

## 🎯 What is BingoCube?

BingoCube is a **standalone cryptographic tool** that any system can use for:

- 🔐 **Identity verification** (BearDog)
- 🤝 **Peer trust stamps** (Songbird)
- 📄 **Content fingerprints** (NestGate)
- ⚙️ **Computation proofs** (ToadStool)
- 🎨 **Visual representation** (petalTongue)

[... rest of README ...]
```

### Phase 5: Update petalTongue Dependencies

In `petalTongue/Cargo.toml`:

```toml
# Change from path dependency
# OLD:
bingocube-core = { path = "../bingoCube/core" }
bingocube-adapters = { path = "../bingoCube/adapters", features = ["visual"] }

# NEW (git dependency):
bingocube-core = { git = "https://github.com/ecoPrimals/bingoCube", rev = "main" }
bingocube-adapters = { git = "https://github.com/ecoPrimals/bingoCube", rev = "main", features = ["visual"] }

# OR (once published to crates.io):
bingocube-core = "0.1"
bingocube-adapters = { version = "0.1", features = ["visual"] }
```

### Phase 6: Git Operations

```bash
# In bingoCube/ (new standalone repo)
git add .
git commit -m "Initial commit: Extract BingoCube as standalone tool

- Pure cryptographic core (600 lines, 7 tests)
- Optional visualization adapters (800 lines, 2 tests)
- Interactive demos (300 lines)
- Comprehensive whitepaper (4 documents, ~110 pages)
- Ready for ecosystem-wide adoption"

git branch -M main
git push -u origin main

# Tag initial release
git tag v0.1.0
git push origin v0.1.0
```

### Phase 7: Update petalTongue

```bash
# In petalTongue/ (update to use new repo)
cd /home/eastgate/Development/ecoPrimals/phase2/petalTongue/

# Remove nested bingoCube directory
rm -rf bingoCube/

# Update Cargo.toml (as shown in Phase 5)
# Update any import paths if needed

# Test that everything still works
cargo build --all
cargo test --all

# Commit the change
git add .
git commit -m "Refactor: Use BingoCube as external dependency

BingoCube is now a standalone repository:
https://github.com/ecoPrimals/bingoCube

This change:
- Removes nested bingoCube/ directory
- Updates to git dependency
- Maintains all functionality
- Follows tool independence pattern"

git push
```

---

## 📋 Checklist

### Pre-Extraction
- [x] BingoCube whitepaper complete (4 documents)
- [x] BingoCube code production-ready
- [x] Tests passing (9 tests, 100% rate)
- [x] Documentation comprehensive
- [x] GitHub repo created: `git@github.com:ecoPrimals/bingoCube.git`

### During Extraction
- [ ] Copy bingoCube/ to parallel directory
- [ ] Create root Cargo.toml (workspace)
- [ ] Update individual Cargo.toml files
- [ ] Create comprehensive README.md
- [ ] Create LICENSE file (AGPL-3.0)
- [ ] Create .gitignore
- [ ] Initialize git repository
- [ ] Add remote origin
- [ ] Initial commit and push

### Post-Extraction
- [ ] Remove bingoCube/ from petalTongue
- [ ] Update petalTongue Cargo.toml (git dependency)
- [ ] Verify petalTongue builds
- [ ] Verify petalTongue tests pass
- [ ] Update petalTongue documentation
- [ ] Tag v0.1.0 release

### Documentation Updates
- [ ] Update petalTongue README (link to BingoCube repo)
- [ ] Update BINGOCUBE_TOOL_USE_PATTERNS.md (new repo location)
- [ ] Update ROOT_DOCS_INDEX.md
- [ ] Create migration note for other developers

---

## 🎯 Benefits of Extraction

### For BingoCube
1. **Independent Versioning**: Can release updates without coordinating with petalTongue
2. **Broader Adoption**: Easy for any primal to add as dependency
3. **Clear Ownership**: Standalone project with its own identity
4. **Better Documentation**: Focused docs for tool users
5. **Easier Testing**: Isolated test suite
6. **Publish to crates.io**: Official Rust package distribution

### For petalTongue
1. **Cleaner Codebase**: No embedded sub-projects
2. **Lighter Repository**: ~1,700 lines moved out
3. **Clear Dependencies**: Explicit external tool usage
4. **Easier Maintenance**: Update BingoCube via version bump
5. **Example Pattern**: Shows how to use external tools

### For Ecosystem
1. **Reusable Tool**: BearDog, NestGate, others can use immediately
2. **Clear Pattern**: How to create ecosystem tools
3. **Discoverability**: Separate repo = easier to find
4. **Community**: Can accept contributions independently

---

## 📚 Documentation That Needs Updating

### In BingoCube (New Repo)
- `README.md` - Comprehensive tool overview
- `LICENSE` - AGPL-3.0
- `CONTRIBUTING.md` - How to contribute
- `CHANGELOG.md` - Version history
- `.gitignore` - Rust + cargo ignores
- `whitePaper/README.md` - Already good, minor path updates

### In petalTongue (Post-Extraction)
- `README.md` - Update BingoCube reference (external tool)
- `BINGOCUBE_TOOL_USE_PATTERNS.md` - Update paths and repo references
- `ROOT_DOCS_INDEX.md` - Update BingoCube section
- `Cargo.toml` - Git dependency instead of path

---

## 🔗 Dependency Graph (After Extraction)

```
┌─────────────────────────────────────────────────────┐
│ GitHub: ecoPrimals/bingoCube (Standalone)           │
│ ├── bingocube-core                                  │
│ └── bingocube-adapters                              │
└─────────────────────────────────────────────────────┘
         ↑
         │ (git dependency or crates.io)
         │
┌────────────────────────────────────────────────────┐
│ GitHub: ecoPrimals/petalTongue                     │
│ └── Uses BingoCube for visualization demo         │
└────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────┐
│ Future: BearDog (Identity Primal)                  │
│ └── Uses bingocube-core for identity verification │
└────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────┐
│ Future: NestGate (Content Primal)                  │
│ └── Uses bingocube-core for content fingerprints  │
└────────────────────────────────────────────────────┘
```

---

## 🚨 Potential Issues and Solutions

### Issue 1: petalTongue Build Breaks

**Symptom**: `cargo build` fails with "cannot find crate `bingocube_core`"

**Solution**:
```bash
# Make sure git repo is accessible
git ls-remote git@github.com:ecoPrimals/bingoCube.git

# Force cargo to fetch new dependency
cargo clean
cargo update
cargo build
```

### Issue 2: Integration Tests Fail

**Symptom**: UI tests that use BingoCube fail

**Solution**:
- Verify `bingocube_integration.rs` still compiles
- Check import paths (should be unchanged)
- Ensure adapters feature is enabled in Cargo.toml

### Issue 3: Git Clone Requires SSH Key

**Symptom**: Others can't clone because they don't have SSH access

**Solution Option 1** - Make repo public:
```bash
# In GitHub settings, make repo public
```

**Solution Option 2** - Use HTTPS:
```toml
# In Cargo.toml
bingocube-core = { git = "https://github.com/ecoPrimals/bingoCube" }
```

---

## 📊 Success Criteria

After extraction, verify:

- ✅ BingoCube builds standalone: `cd bingoCube && cargo build --all`
- ✅ BingoCube tests pass: `cargo test --all`
- ✅ petalTongue builds: `cd petalTongue && cargo build --all`
- ✅ petalTongue tests pass: `cargo test --all`
- ✅ BingoCube UI demo works in petalTongue
- ✅ Git repo is accessible and clonable
- ✅ Documentation is complete and accurate
- ✅ No broken links in documentation
- ✅ Other developers can use as dependency

---

## 🎉 Post-Extraction Announcements

### To ecoPrimals Team
```
🎲 BingoCube is now an independent tool!

Repository: https://github.com/ecoPrimals/bingoCube
Whitepaper: 4 documents, ~110 pages
Status: Production ready

Any primal can now use BingoCube:

```toml
[dependencies]
bingocube-core = { git = "https://github.com/ecoPrimals/bingoCube" }
```

Use cases:
- Identity verification (BearDog)
- Content fingerprinting (NestGate)
- P2P trust (Songbird)
- Computation proofs (ToadStool)
- Visualization (petalTongue)

See whitepaper for integration patterns!
```

### To Other Primal Developers
```
🎲 New Tool Available: BingoCube

What: Human-verifiable cryptographic commitments
Why: Visual patterns humans can recognize + progressive reveal
How: Add as dependency, use in your primal

Documentation: whitePaper/ directory
Examples: demos/ directory
Integration: See BingoCube-Ecosystem-Examples.md

Questions? See README or open an issue!
```

---

## 📅 Timeline

| Phase | Estimated Time | Responsible |
|-------|----------------|-------------|
| Pre-Extraction Prep | ✅ Complete | Done |
| Extract & Setup Repo | 30 minutes | Now |
| Update Documentation | 30 minutes | Now |
| Test & Verify | 15 minutes | Now |
| Push to GitHub | 5 minutes | Now |
| Update petalTongue | 20 minutes | Now |
| Announce to Team | 5 minutes | After push |
| **Total** | **~2 hours** | **Today** |

---

## 🔗 Related Documents

- [BingoCube README](bingoCube/README.md)
- [BINGOCUBE_TOOL_USE_PATTERNS.md](../BINGOCUBE_TOOL_USE_PATTERNS.md)
- [BingoCube Whitepaper Overview](bingoCube/whitePaper/BingoCube-Overview.md)
- [BingoCube Biometric Identity](bingoCube/whitePaper/BingoCube-Biometric-Identity.md) ⭐ NEW!
- [Ecosystem Examples](bingoCube/whitePaper/BingoCube-Ecosystem-Examples.md)

---

**Status**: Ready to execute  
**Next Action**: Run Phase 1 extraction steps  
**Expected Completion**: Today (December 26, 2025)

---

*"Tools should be independent, reusable, and discoverable. BingoCube embodies this principle."*


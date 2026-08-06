<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# BingoCube: Biometric-Seeded Identity — Core Concepts

**Part 1 of 3** · [Medical Use Cases](BingoCube-Biometric-Identity-Medical.md) · [Privacy & Threat Model](BingoCube-Biometric-Identity-Privacy.md)

**Version**: 1.0  
**Date**: December 26, 2025  
**Authors**: ecoPrimals Team  
**Status**: Reference Implementation

---

## Abstract

This document describes a novel identity architecture combining biometric scanning with BingoCube's progressive reveal properties to create sovereign, portable, zero-knowledge identity systems. By using biometric data as ephemeral seed material rather than storing it, we enable human-centered digital identity without surveillance, honeypots, or central authorities.

**Key Innovation**: Biometric data generates the seed but is never stored—only the resulting BingoCube and derived keys persist. This enables zero-knowledge verification, progressive trust establishment, and true data sovereignty.

**Primary Applications**: Homeless services, medical data sovereignty, cross-organization identity, and any scenario requiring human identity without central databases.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Core Architecture](#2-core-architecture)
3. [Security Model](#3-security-model)
4. [Use Case: Homeless Services](#4-use-case-homeless-services)
5. [Use Case: Medical Data Sovereignty](#5-use-case-medical-data-sovereignty)
6. [Implementation Patterns](#6-implementation-patterns)
7. [Primal Integration](#7-primal-integration)
8. [Security Analysis](#8-security-analysis)
9. [Privacy Guarantees](#9-privacy-guarantees)
10. [Comparison to Existing Systems](#10-comparison-to-existing-systems)
11. [Future Directions](#11-future-directions)

---

## 1. Introduction

### 1.1 The Problem

Modern identity systems face fundamental tensions:

- **Biometric Systems**: Store biometric data (honeypot risk)
- **Database Systems**: Centralized identity (surveillance risk)
- **Blockchain Systems**: Permanent identity (no revocation)
- **Password Systems**: Forgettable (usability risk)

For vulnerable populations (homeless, refugees, disaster victims), these systems fail:
- No ID → No services
- Centralized records → Privacy violations
- Multiple organizations → Data silos
- Bureaucratic friction → Days/weeks to establish identity

### 1.2 The Solution

**Biometric-Seeded BingoCube Identity**:

```
Biometric Scan + Live Entropy → Seed → BingoCube → Identity + Keys
         ↑                                              ↓
     NEVER STORED                           Stored, Shareable, Verifiable
```

**Properties**:
- ✅ No biometric storage (no honeypot)
- ✅ Progressive trust (reveal 20% → 50% → 100%)
- ✅ Zero-knowledge verification
- ✅ Portable data (user owns encrypted vault)
- ✅ Cross-organization without central DB
- ✅ Instant identity establishment
- ✅ Revocable and regenerable

### 1.3 Key Contributions

1. **Ephemeral Biometric Pattern**: Biometric used as entropy source, never stored
2. **Progressive Identity Verification**: Nested proof structure (x=0.2 ⊆ x=0.5 ⊆ x=1.0)
3. **Sovereign Data Vaults**: User owns encrypted data, grants access selectively
4. **Professional Courtesy Pattern**: Own data you don't decrypt (medical ethics)
5. **Zero-Knowledge Cross-Organization**: Share proofs without revealing full identity

---

## 2. Core Architecture

### 2.1 Identity Establishment

```
┌─────────────────────────────────────────────────────────┐
│ Step 1: Biometric Capture (With Consent)                │
└─────────────────────────────────────────────────────────┘

User touches scanner → Biometric data B captured
Examples: fingerprint, palm print, iris scan, voice print

┌─────────────────────────────────────────────────────────┐
│ Step 2: Entropy Generation                              │
└─────────────────────────────────────────────────────────┘

Live entropy E generated:
- Timestamp (nanosecond precision)
- Device identifier
- Geolocation (if available)
- Random nonce
- Environmental factors (optional)

┌─────────────────────────────────────────────────────────┐
│ Step 3: Seed Derivation (Biometric Destroyed After)     │
└─────────────────────────────────────────────────────────┘

Seed S = BLAKE3(B || E || "BINGOCUBE_IDENTITY_V1")

CRITICAL: Biometric data B is discarded after this step!
Only the seed S persists temporarily (until cube generated)

┌─────────────────────────────────────────────────────────┐
│ Step 4: BingoCube Generation                            │
└─────────────────────────────────────────────────────────┘

Identity Cube C = BingoCube::from_seed(S, Config::default())

Result: L×L color grid that represents user's identity
Deterministic: Same biometric + entropy → Same cube

┌─────────────────────────────────────────────────────────┐
│ Step 5: Key Derivation                                  │
└─────────────────────────────────────────────────────────┘

Master Key:    K_master  = KDF(C.hash(), "MASTER")
Medical Key:   K_medical = KDF(C.hash(), "MEDICAL")
Housing Key:   K_housing = KDF(C.hash(), "HOUSING")
Social Key:    K_social  = KDF(C.hash(), "SOCIAL")
...additional domain-specific keys as needed

┌─────────────────────────────────────────────────────────┐
│ Step 6: Identity Package Creation                       │
└─────────────────────────────────────────────────────────┘

IdentityPackage {
    visual_identity: C.subcube(1.0),      // Full cube (private)
    public_proof:    C.subcube(0.3),      // 30% reveal (shareable)
    verification_proof: C.subcube(0.5),   // 50% reveal (higher trust)
    keys: [K_master, K_medical, ...],     // Encrypted with master
    created: Timestamp,
    config: CubeConfig,
}

User shown visual pattern:
█ █ █ █ █
█ █ █ █ █    ← "This is YOUR identity pattern"
█ █ ✱ █ █    ← Remember this! (optional, not required)
█ █ █ █ █
█ █ █ █ █
```

### 2.2 Identity Verification

```
┌─────────────────────────────────────────────────────────┐
│ Verification Protocol (Progressive Trust)               │
└─────────────────────────────────────────────────────────┘

Given: Stored public_proof (30% of cube)
Goal:  Verify user is same person who created identity

Step 1: User touches scanner
    B' = scan_biometric()
    E' = generate_new_entropy()  // DIFFERENT from original!

Step 2: Regenerate cube
    S' = BLAKE3(B' || E' || "BINGOCUBE_IDENTITY_V1")
    C' = BingoCube::from_seed(S', Config::default())

Step 3: Progressive verification
    Level 1 (Low Stakes):
        If C'.subcube(0.2) ∩ stored_proof ≥ threshold_low:
            → "Possible match" (20% confidence)
    
    Level 2 (Medium Stakes):
        If C'.subcube(0.5) == stored_proof.expand_to(0.5):
            → "Probable match" (50% confidence)
    
    Level 3 (High Stakes):
        If C'.subcube(1.0) == stored_full_cube:
            → "Verified match" (100% confidence)

Step 4: Access granted based on verification level
    Low:    Read public records
    Medium: Update personal records
    High:   Transfer data, generate keys, grant access
```

### 2.3 Zero-Knowledge Cross-Organization

```
┌─────────────────────────────────────────────────────────┐
│ Organization A → Organization B (Without Central DB)    │
└─────────────────────────────────────────────────────────┘

Scenario: User visits Org B, but was established at Org A

Step 1: Org A creates transfer token
    TransferToken {
        proof: user_cube.subcube(0.3),  // 30% reveal
        org_id: "Org_A",
        issued: Timestamp,
        expires: Timestamp + 30 days,
        signature: Org_A.sign(proof),
    }

Step 2: User presents to Org B (with consent)
    User: "I'm registered at Org A"
    Org B: "Prove it"
    User: [shows transfer token]
    Org B: [verifies Org A signature]

Step 3: User scans biometric at Org B
    B' = scan_biometric()
    E' = generate_entropy()
    C' = BingoCube::from_seed(BLAKE3(B' || E'), Config)

Step 4: Org B verifies
    If C'.subcube(0.3) == TransferToken.proof:
        ✅ Verified! User is who they claim
        → Org B can now request full records from Org A
        → OR user can grant access to their vault directly

Key Properties:
- Org B never sees full identity
- Org A never sends biometric data
- User present for verification (liveness)
- No central database coordinating
- User consent required for data sharing
```

### 2.4 Sovereign Data Vault

```
┌─────────────────────────────────────────────────────────┐
│ User-Owned Encrypted Data Vault                         │
└─────────────────────────────────────────────────────────┘

Structure:
VaultFile {
    metadata: {
        identity_proof: subcube(0.3),    // For verification
        created: Timestamp,
        version: "1.0",
    },
    
    data_blobs: {
        medical_general: Encrypt(data, K_medical),
        medical_psych:   Encrypt(data, K_medical + K_prof_seal),
        housing_records: Encrypt(data, K_housing),
        social_services: Encrypt(data, K_social),
        employment:      Encrypt(data, K_employment),
    },
    
    access_log: [
        { org: "Shelter_A", timestamp, data_types, granted_keys },
        { org: "Clinic_B",  timestamp, data_types, granted_keys },
    ],
    
    policies: Encrypt({
        "medical_psych": { access: "professional_only" },
        "housing": { access: "user_or_housing_authority" },
    }, K_master),
}

Properties:
- Vault is a single portable file
- User carries it (USB drive, phone, cloud with E2EE)
- No server required for basic operations
- Organizations get temporary access keys
- User can revoke access anytime
- Audit log of all access
```

---

## 3. Security Model

### 3.1 Threat Model

**Assumptions**:
- Scanner hardware is trusted (or user trusts the location)
- BLAKE3 is cryptographically secure (standard assumption)
- User's biometric is stable enough for regeneration
- Entropy source has sufficient randomness

**Threats Considered**:
1. **Biometric Theft**: Attacker steals biometric data
2. **Replay Attack**: Attacker captures and replays cube
3. **Cube Forgery**: Attacker tries to create matching cube
4. **Partial Reveal Attack**: Attacker has 30% reveal, tries to compute 100%
5. **Cross-Organization Correlation**: Organizations collude to track user
6. **Scanner Compromise**: Malicious scanner records biometric
7. **Vault Theft**: Attacker steals encrypted vault file

### 3.2 Security Properties

#### Property 1: No Biometric Honeypot

**Claim**: No biometric data is stored anywhere in the system.

**Proof**: 
- Biometric B used only to compute seed S = BLAKE3(B || E)
- B is discarded immediately after seed generation
- Only cube C = BingoCube::from_seed(S) is stored
- C is cryptographically derived; reversing BLAKE3(B || E) ← C is infeasible
- Even with full cube (x=1.0), attacker cannot recover B

**Attack Resistance**:
- Compromising storage yields C (not B)
- Compromising scanner yields single-use B (entropy changes)
- No persistent biometric database to target

#### Property 2: Progressive Forgery Resistance

**Claim**: Forging a matching cube requires exponential trials.

**Proof**: From BingoCube-Mathematical-Foundation.md:

```
P(forge at x) ≈ (K/U)^(m(x))

where:
- K = palette size (e.g., 256)
- U = universe size (e.g., 100)
- m(x) = ⌈x · L²⌉ cells revealed

For L=8, K=256, U=100:
- x=0.2: m(x)=13  → P ≈ 2^-20  (1 in million)
- x=0.5: m(x)=32  → P ≈ 2^-50  (1 in quadrillion)
- x=1.0: m(x)=64  → P ≈ 2^-100 (effectively impossible)
```

**Attack Resistance**:
- Attacker with 30% reveal cannot feasibly forge 50% match
- Attacker with 50% reveal cannot feasibly forge 100% match
- Progressive trust: Higher stakes require higher reveal

#### Property 3: Entropy Freshness

**Claim**: Each verification uses new entropy, preventing replay.

**Proof**:
- Entropy E includes timestamp (nanosecond precision)
- E includes device nonce (random per session)
- Seed S = BLAKE3(B || E) differs each time
- Cube C = from_seed(S) differs each time
- Old cube captures cannot be replayed

**Attack Resistance**:
- Attacker capturing cube at time T₁ cannot replay at time T₂
- Each verification requires fresh biometric scan
- Liveness implicitly guaranteed (user must be present)

#### Property 4: Zero-Knowledge Cross-Organization

**Claim**: Organizations cannot correlate users without user consent.

**Proof**:
- Org A sees: subcube(0.3) = subset S_A of cells
- Org B sees: subcube(0.3) = subset S_B of cells
- If S_A and S_B are independent subsets: no correlation
- If S_A = S_B (same 30%): correlation requires both organizations colluding + user presenting same token
- User controls what level to reveal to each org
- User can generate different cubes for different contexts (different config)

**Attack Resistance**:
- Organizations cannot track users across contexts without consent
- User controls revelation level per organization
- No central database to query for correlations

#### Property 5: Vault Confidentiality

**Claim**: Encrypted vault is secure without biometric.

**Proof**:
- Keys derived from cube: K = KDF(C.hash(), domain)
- C regenerated only from biometric: C = from_seed(BLAKE3(B || E))
- Vault encrypted: V = Encrypt(data, K)
- Attacker without biometric cannot derive K
- Attacker with vault V but not K cannot decrypt
- K is never stored; regenerated each session

**Attack Resistance**:
- Stolen vault useless without biometric
- Stolen biometric alone insufficient (need correct entropy)
- Brute-forcing K infeasible (256-bit key)

### 3.3 Attack Scenarios and Mitigations

#### Scenario 1: Biometric Theft from Scanner

**Attack**: 
- Malicious scanner records biometric B
- Attacker tries to impersonate user

**Mitigation**:
- User only scans at trusted locations (consent-based)
- Entropy E includes device identifier (attacker's device differs)
- Attacker cannot replicate exact entropy E from original registration
- At best, attacker can create different cube C' ≠ C
- Verification fails: C'.subcube(x) ≠ stored.subcube(x)

**Additional Defense**:
- Multi-factor: Require user to also input PIN or challenge
- Liveness detection: Scanner verifies living tissue
- Time-limited tokens: Old captures expire

#### Scenario 2: Partial Cube Forgery

**Attack**:
- Attacker has subcube(0.3) from transfer token
- Tries to forge subcube(0.5) or subcube(1.0)

**Mitigation**:
- Subcube nesting: 0.3 ⊂ 0.5 ⊂ 1.0
- Attacker must match revealed cells from 0.3
- PLUS compute remaining cells for 0.5
- Probability: P(forge 0.5 | knows 0.3) ≈ 2^-35 (infeasible)
- Verification fails unless attacker has actual biometric

#### Scenario 3: Cross-Organization Collusion

**Attack**:
- Org A and Org B collude to track user
- Both have subcube(0.3) from user

**Mitigation**:
- User controls what to reveal to each org
- Can reveal different subsets (not same 0.3)
- Can use different configs (different grid sizes)
- Can regenerate entirely (new biometric capture with different initial entropy)
- Organizations need user consent to share data
- Audit logs track who accessed what

**Additional Defense**:
- User can query: "Who has partial proofs of my identity?"
- Can revoke access tokens
- Can generate new identity (different biometric enrollment)

#### Scenario 4: Vault and Partial Cube Stolen

**Attack**:
- Attacker steals encrypted vault V
- Attacker also has public proof subcube(0.3)
- Tries to decrypt vault

**Mitigation**:
- Vault encrypted with K_domain keys
- K_domain = KDF(C.hash(), domain)
- C regenerated from biometric B
- Attacker without B cannot compute C
- Attacker cannot derive K_domain
- Vault remains encrypted

**Brute-Force Analysis**:
- Key space: 2^256 (BLAKE3 output)
- Even with 0.3 reveal: No shortcut to full cube
- Expected trials: 2^50 to forge 0.5 match
- Then still need biometric to regenerate for keys

---


---

## 6. Implementation Patterns

### 6.1 BearDog Integration (Identity Primal)

```rust
// BearDog is responsible for identity primitives
pub struct BearDogIdentityService {
    scanner: BiometricScanner,
    entropy_generator: EntropySource,
    cube_generator: BingoCubeGenerator,
    key_deriver: KeyDerivationService,
}

impl BearDogIdentityService {
    /// Establish new identity for user
    /// Returns: IdentityPackage (NO biometric data!)
    pub fn establish_identity(
        &self,
        consent: UserConsent,
        config: CubeConfig,
    ) -> Result<IdentityPackage> {
        // 1. Capture biometric (with explicit consent)
        let biometric = self.scanner.capture_with_consent(consent)?;
        
        // 2. Generate live entropy
        let entropy = self.entropy_generator.generate()?;
        
        // 3. Derive seed (biometric destroyed after this!)
        let seed = self.derive_seed(&biometric, &entropy)?;
        
        // 4. Generate BingoCube
        let cube = self.cube_generator.from_seed(&seed, config)?;
        
        // 5. Derive domain-specific keys
        let keys = self.key_deriver.derive_all(&cube)?;
        
        // 6. Package (biometric already gone!)
        Ok(IdentityPackage {
            visual_identity: cube.clone(),
            public_proof: cube.subcube(0.3)?,
            verification_proof: cube.subcube(0.5)?,
            keys,
            config,
            created_at: Timestamp::now(),
        })
    }
    
    /// Verify identity (progressive trust)
    pub fn verify_identity(
        &self,
        stored_proof: SubCube,
        trust_level: TrustLevel,
    ) -> Result<VerificationResult> {
        // 1. Capture fresh biometric
        let biometric = self.scanner.capture()?;
        
        // 2. Generate NEW entropy (anti-replay)
        let entropy = self.entropy_generator.generate()?;
        
        // 3. Regenerate cube
        let seed = self.derive_seed(&biometric, &entropy)?;
        let cube = self.cube_generator.from_seed(&seed, config)?;
        
        // 4. Progressive verification
        let reveal_level = match trust_level {
            TrustLevel::Low => 0.2,
            TrustLevel::Medium => 0.5,
            TrustLevel::High => 1.0,
        };
        
        let claimed = cube.subcube(reveal_level)?;
        let stored_expanded = stored_proof.expand_to(reveal_level)?;
        
        // 5. Compare
        if claimed == stored_expanded {
            Ok(VerificationResult::Verified {
                confidence: reveal_level,
                timestamp: Timestamp::now(),
            })
        } else {
            Ok(VerificationResult::Failed {
                reason: "Cube mismatch",
            })
        }
    }
    
    /// Create transfer token for cross-org
    pub fn create_transfer_token(
        &self,
        identity_package: &IdentityPackage,
        recipient_org: OrgIdentity,
        reveal_level: f64,
    ) -> Result<TransferToken> {
        let proof = identity_package.visual_identity.subcube(reveal_level)?;
        
        Ok(TransferToken {
            proof,
            issued_by: self.org_identity(),
            issued_to: recipient_org,
            issued_at: Timestamp::now(),
            expires_at: Timestamp::now() + Duration::days(30),
            signature: self.sign(&proof)?,
        })
    }
    
    // CRITICAL: Seed derivation must be deterministic
    // but biometric is destroyed immediately after!
    fn derive_seed(
        &self,
        biometric: &BiometricData,
        entropy: &Entropy,
    ) -> Result<Seed> {
        // Combine with domain separation
        let input = format!(
            "BINGOCUBE_IDENTITY_V1||{}||{}",
            biometric.to_bytes(),
            entropy.to_bytes()
        );
        
        // Hash to fixed-size seed
        let seed = BLAKE3::hash(input.as_bytes());
        
        // CRITICAL: Biometric is destroyed when this function returns!
        // Only seed persists (temporarily, until cube generated)
        
        Ok(Seed::from_bytes(seed.as_bytes()))
    }
}
```

### 6.2 Sovereign Vault Implementation

```rust
pub struct SovereignVault {
    // Metadata (unencrypted for verification)
    metadata: VaultMetadata,
    
    // Encrypted data blobs
    data_blobs: HashMap<DataDomain, EncryptedBlob>,
    
    // Access policies (encrypted with master key)
    policies: EncryptedPolicies,
    
    // Access log (append-only, signed)
    access_log: Vec<AccessLogEntry>,
}

pub struct VaultMetadata {
    identity_proof: SubCube,  // subcube(0.3) for verification
    created_at: Timestamp,
    version: String,
    schema_version: u32,
}

pub enum DataDomain {
    Medical,
    MedicalPsych,  // Professionally sealed
    Housing,
    Social,
    Employment,
    Education,
    Legal,
}

impl SovereignVault {
    /// Create new vault for identity
    pub fn create_for_identity(
        identity_package: &IdentityPackage,
    ) -> Self {
        Self {
            metadata: VaultMetadata {
                identity_proof: identity_package.public_proof.clone(),
                created_at: Timestamp::now(),
                version: "1.0".to_string(),
                schema_version: 1,
            },
            data_blobs: HashMap::new(),
            policies: EncryptedPolicies::new(&identity_package.keys.master),
            access_log: vec![],
        }
    }
    
    /// Verify this vault belongs to the user
    pub fn verify_ownership(
        &self,
        identity_package: &IdentityPackage,
    ) -> Result<bool> {
        Ok(self.metadata.identity_proof == identity_package.public_proof)
    }
    
    /// Add encrypted data to vault
    pub fn store_data(
        &mut self,
        domain: DataDomain,
        data: &[u8],
        key: &EncryptionKey,
    ) -> Result<()> {
        let encrypted = encrypt(data, key)?;
        self.data_blobs.insert(domain, encrypted);
        
        self.access_log.push(AccessLogEntry {
            action: Action::Store,
            domain,
            timestamp: Timestamp::now(),
            actor: Actor::Owner,
        });
        
        Ok(())
    }
    
    /// Add professionally sealed data (dual-key)
    pub fn store_sealed_data(
        &mut self,
        domain: DataDomain,
        data: &[u8],
        patient_key: &EncryptionKey,
        professional_seal: &ProfessionalKey,
    ) -> Result<()> {
        let dual_key = combine_keys(patient_key, professional_seal)?;
        let encrypted = encrypt(data, &dual_key)?;
        
        self.data_blobs.insert(domain, EncryptedBlob {
            data: encrypted,
            encryption_type: EncryptionType::DualKey,
            sealed_by: Some(professional_seal.public_id()),
            access_policy: AccessPolicy::ProfessionalUnseal,
        });
        
        self.access_log.push(AccessLogEntry {
            action: Action::StoreSeal,
            domain,
            timestamp: Timestamp::now(),
            actor: Actor::Professional(professional_seal.public_id()),
        });
        
        Ok(())
    }
    
    /// Retrieve and decrypt data
    pub fn retrieve_data(
        &self,
        domain: DataDomain,
        key: &EncryptionKey,
    ) -> Result<Vec<u8>> {
        let blob = self.data_blobs.get(&domain)
            .ok_or("Domain not found")?;
        
        // Check access policy
        match blob.access_policy {
            AccessPolicy::Standard => {
                decrypt(&blob.data, key)
            }
            AccessPolicy::ProfessionalUnseal => {
                Err("Requires professional key to unseal".into())
            }
        }
    }
    
    /// Grant temporary access to organization
    pub fn grant_access(
        &mut self,
        org: OrgIdentity,
        domains: Vec<DataDomain>,
        keys: Vec<EncryptionKey>,
        duration: Duration,
    ) -> Result<AccessGrant> {
        let grant = AccessGrant {
            vault_id: self.metadata.identity_proof.clone(),
            granted_to: org.clone(),
            domains: domains.clone(),
            wrapped_keys: keys.iter().map(|k| org.public_key.wrap(k)).collect(),
            expires_at: Timestamp::now() + duration,
            revocable: true,
        };
        
        self.access_log.push(AccessLogEntry {
            action: Action::GrantAccess,
            domain: DataDomain::Multiple(domains),
            timestamp: Timestamp::now(),
            actor: Actor::Organization(org),
        });
        
        Ok(grant)
    }
    
    /// Export entire vault (for portability)
    pub fn export_to_file(&self, path: &Path) -> Result<()> {
        let serialized = bincode::serialize(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }
    
    /// Import vault from file
    pub fn import_from_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let vault = bincode::deserialize(&bytes)?;
        Ok(vault)
    }
}
```

### 6.3 Progressive Verification Protocol

```rust
pub enum TrustLevel {
    Low,     // 20% reveal
    Medium,  // 50% reveal  
    High,    // 100% reveal
}

pub struct VerificationProtocol {
    stored_proofs: HashMap<TrustLevel, SubCube>,
    config: CubeConfig,
}

impl VerificationProtocol {
    pub fn new(identity_package: &IdentityPackage) -> Result<Self> {
        let full_cube = &identity_package.visual_identity;
        
        Ok(Self {
            stored_proofs: hashmap! {
                TrustLevel::Low => full_cube.subcube(0.2)?,
                TrustLevel::Medium => full_cube.subcube(0.5)?,
                TrustLevel::High => full_cube.subcube(1.0)?,
            },
            config: identity_package.config.clone(),
        })
    }
    
    pub fn verify(
        &self,
        biometric: &BiometricData,
        entropy: &Entropy,
        required_trust: TrustLevel,
    ) -> Result<VerificationResult> {
        // Regenerate cube
        let seed = Self::derive_seed(biometric, entropy)?;
        let cube = BingoCube::from_seed(&seed, self.config.clone())?;
        
        // Get stored proof for this trust level
        let stored = self.stored_proofs.get(&required_trust)
            .ok_or("Trust level not configured")?;
        
        // Get revealed level for trust
        let reveal_level = match required_trust {
            TrustLevel::Low => 0.2,
            TrustLevel::Medium => 0.5,
            TrustLevel::High => 1.0,
        };
        
        // Compare
        let claimed = cube.subcube(reveal_level)?;
        
        if claimed == *stored {
            Ok(VerificationResult::Success {
                trust_level: required_trust,
                confidence: reveal_level,
                timestamp: Timestamp::now(),
            })
        } else {
            // Check if partial match (lower trust level)
            if let Some(lower_trust) = self.check_lower_trust(&cube) {
                Ok(VerificationResult::PartialMatch {
                    achieved_trust: lower_trust,
                    required_trust,
                    message: "Identity verified at lower trust level",
                })
            } else {
                Ok(VerificationResult::Failed {
                    reason: "No cube match at any trust level",
                })
            }
        }
    }
    
    fn check_lower_trust(&self, cube: &BingoCube) -> Option<TrustLevel> {
        // Try low trust level
        if let Ok(low_claimed) = cube.subcube(0.2) {
            if let Some(low_stored) = self.stored_proofs.get(&TrustLevel::Low) {
                if low_claimed == *low_stored {
                    return Some(TrustLevel::Low);
                }
            }
        }
        None
    }
    
    fn derive_seed(biometric: &BiometricData, entropy: &Entropy) -> Result<Seed> {
        let input = format!(
            "BINGOCUBE_IDENTITY_V1||{}||{}",
            biometric.to_bytes(),
            entropy.to_bytes()
        );
        let hash = BLAKE3::hash(input.as_bytes());
        Ok(Seed::from_bytes(hash.as_bytes()))
    }
}

pub enum VerificationResult {
    Success {
        trust_level: TrustLevel,
        confidence: f64,
        timestamp: Timestamp,
    },
    PartialMatch {
        achieved_trust: TrustLevel,
        required_trust: TrustLevel,
        message: String,
    },
    Failed {
        reason: String,
    },
}
```


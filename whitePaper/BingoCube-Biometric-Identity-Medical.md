<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# BingoCube: Biometric-Seeded Identity — Medical & Emergency Use Cases

[Core Concepts](BingoCube-Biometric-Identity-Core.md) · **Part 2 of 3** · [Privacy & Threat Model](BingoCube-Biometric-Identity-Privacy.md)

**Version**: 1.0  
**Date**: December 26, 2025  
**Authors**: ecoPrimals Team  

---

## 4. Use Case: Homeless Services

### 4.1 Problem Statement

**Current Reality**:
- No ID → Cannot access services (shelter, food, medical)
- Paper records → Lost, destroyed, or stolen
- Multiple organizations → Duplicate registrations, no data sharing
- Bureaucracy → Days or weeks to establish eligibility
- Privacy → Centralized databases track vulnerable populations
- Mobility → Moving between cities requires re-registration

**Impact**:
- 40% of homeless have no government ID
- Average 2-3 weeks to establish new identity in new city
- Medical records lost when moving
- Housing applications fail due to incomplete history

### 4.2 BingoCube Solution

#### Registration Flow (Day 1 - Shelter A)

```
1. Arrive at Shelter A (San Francisco)
   Staff: "Welcome! We use biometric identity. No ID needed."
   Staff: "Touch this scanner with your palm."
   
2. Biometric Scan + Identity Generation
   [User touches scanner]
   System: [generates identity cube in <2 seconds]
   System: "This is your identity pattern:"
   
   █ █ █ █ █
   █ █ █ █ █    [Display on screen]
   █ █ ✱ █ █
   █ █ █ █ █
   █ █ █ █ █
   
   System: "Remember this pattern! (optional)"
   User: "That's... actually kind of pretty."

3. Data Vault Creation
   System: [creates encrypted vault]
   Vault contains:
   - Basic info (name, DOB if known)
   - Shelter A services record
   - Encrypted with user's keys
   
4. Instant Access
   Staff: "You're registered! Bed #12, meal at 6pm."
   [No paperwork, no waiting, no bureaucracy]
   
5. Identity Proof Generated
   System gives user:
   - Physical card with QR code (contains subcube(0.3))
   - Optional: Print visual pattern
   - Optional: Email vault file to user (if they have email)
```

#### Return Visit (Day 3 - Same Shelter)

```
1. Return to Shelter A
   [User touches scanner]
   System: [regenerates cube from biometric]
   System: [compares to stored records]
   System: "Welcome back! Bed #12 is available."
   
2. Progressive Verification
   - Scanner captures biometric (NEW entropy)
   - Regenerates cube C'
   - Compares: C'.subcube(0.5) == stored.subcube(0.5)
   - Match! → Access granted
   
3. Instant Recognition
   [No need to explain who they are]
   [No need to show physical ID]
   [No risk of lost paperwork]
   
Time: 2 seconds
```

#### Cross-Organization (Day 5 - Clinic B, Oakland)

```
1. Arrive at Clinic B (Different City)
   Staff: "Are you registered anywhere?"
   User: "Yes, at Shelter A in SF."
   
2. Transfer Token Verification
   [User shows QR code from Shelter A]
   QR contains: subcube(0.3) + Shelter A signature
   
   Clinic B: [scans QR]
   Clinic B: "Please touch our scanner to verify."
   
3. Biometric Verification
   [User touches Clinic B scanner]
   System: [generates cube C']
   System: [compares C'.subcube(0.3) to QR token]
   System: "✅ Verified! You're registered at Shelter A."
   
4. Cross-Organization Trust
   Clinic B to Shelter A: "Send medical intake data?"
   Shelter A to User (via SMS): "Clinic B requests medical data. Allow?"
   User: "Yes, allow."
   
   [Shelter A sends encrypted medical data]
   [User's keys decrypt at Clinic B]
   [Seamless data transfer]
   
5. Service Provided
   Clinic B: "Your records show you need vaccination."
   [Provides service]
   [Updates vault with new medical data]
   [Data follows user, not tied to location]
```

#### New City (Day 15 - Shelter C, San Jose)

```
1. Arrive at New City
   User: "I'm new here, but I was at Shelter A and Clinic B."
   Shelter C: "No problem! Touch the scanner."
   
2. Identity Regeneration
   [User touches scanner]
   System: [generates cube from biometric]
   System: [compares to network of shelters]
   System: "Found you! Records from SF and Oakland."
   
3. Aggregated Services
   Shelter C sees:
   - Housing history (14 nights at Shelter A)
   - Medical history (vaccinated, no current meds)
   - Social services (enrolled in job program)
   - Employment attempts (3 interviews scheduled)
   
4. Continuity of Care
   Shelter C: "We can continue your job program here."
   Shelter C: "Your medical records are up to date."
   Shelter C: "Bed assigned, meal vouchers ready."
   
   [No re-registration]
   [No explaining entire situation]
   [No lost records]
   [Just continuity]
```

### 4.3 Data Vault Contents

```javascript
{
  "identity": {
    "proof_30": "subcube(0.3) - for sharing",
    "visual_pattern": "█ █ █ █ █ ...",
    "created_at": "2025-12-01T08:00:00Z"
  },
  
  "housing": {
    "shelter_a_sf": {
      "check_in": "2025-12-01",
      "check_out": "2025-12-14",
      "nights": 14,
      "notes": "Good resident, helped with kitchen"
    },
    "shelter_c_sj": {
      "check_in": "2025-12-15",
      "status": "active"
    }
  },
  
  "medical": {
    "vaccinations": [
      {"type": "Flu", "date": "2025-12-05", "provider": "Clinic_B"}
    ],
    "medications": [],
    "allergies": ["Penicillin"],
    "last_checkup": "2025-12-05"
  },
  
  "social_services": {
    "employment": {
      "job_program": "Tech Training",
      "interviews": [
        {"company": "X", "date": "2025-12-20", "status": "scheduled"}
      ]
    },
    "benefits": {
      "food_stamps": "pending",
      "medicaid": "enrolled"
    }
  },
  
  "access_log": [
    {"org": "Shelter_A", "action": "create_record", "timestamp": "2025-12-01"},
    {"org": "Clinic_B", "action": "read_medical", "timestamp": "2025-12-05"},
    {"org": "Shelter_C", "action": "read_all", "timestamp": "2025-12-15"}
  ]
}

// All encrypted with user's keys
// User owns the file
// Portable (USB drive, phone, email)
```

### 4.4 Privacy Guarantees

1. **No Central Database**
   - No government registry of homeless individuals
   - No corporate surveillance database
   - Each organization stores only subcube(0.3) + their records
   - Cannot correlate across orgs without user consent

2. **User Control**
   - User decides what to share with each org
   - User can revoke access anytime
   - User can view access logs
   - User can export all data

3. **Progressive Trust**
   - Low-stakes (meal voucher): 20% reveal
   - Medium-stakes (medical care): 50% reveal
   - High-stakes (housing application): 100% reveal
   - User controls revelation level

4. **Biometric Never Stored**
   - No biometric database to breach
   - No honeypot for attackers
   - Biometric used only to generate cube
   - Destroyed immediately after

### 4.5 Organization Benefits

1. **Reduced Fraud**
   - Cryptographic identity (can't forge)
   - Liveness required (user must be present)
   - Progressive verification (high stakes = high confidence)

2. **Reduced Bureaucracy**
   - No paperwork
   - No ID verification
   - Instant registration (2 seconds)
   - Automatic record keeping

3. **Better Continuity of Care**
   - Medical records follow user
   - Housing history preserved
   - Social services coordinated
   - No duplicate registrations

4. **Privacy Compliance**
   - No central database to secure
   - User consent for all data sharing
   - Audit logs for compliance
   - GDPR/HIPAA friendly

### 4.6 Impact Metrics (Projected)

| Metric | Current System | BingoCube System | Improvement |
|--------|----------------|------------------|-------------|
| Registration Time | 30-60 minutes | 2 seconds | **99.9%** faster |
| ID Requirements | Photo ID required | None | **100%** inclusive |
| Cross-Org Data Share | Days/weeks | Instant | **Real-time** |
| Privacy Violations | Central DB risk | No central DB | **Zero honeypot** |
| Fraud Rate | 5-10% | <0.01% | **99%** reduction |
| User Mobility | Re-register each city | Instant recognition | **Seamless** |
| Data Loss Rate | 30% (paper records) | <1% (encrypted digital) | **97%** improvement |

---

## 5. Use Case: Medical Data Sovereignty

### 5.1 Problem Statement

**Current Reality**:
- Medical records owned by providers, not patients
- Scattered across multiple systems (hospital, clinic, pharmacy)
- No patient control over access
- Portability difficult (fax, CDs, manual requests)
- Privacy violations (employees accessing records inappropriately)
- Professional notes (psych, therapy) create ethical tensions

**Ethical Tension - Psychologist Notes**:
- Therapist needs to document honestly
- Patient legally owns records
- BUT: Reading notes can harm therapeutic relationship
- Current solution: Patient doesn't request them (social convention)
- Problem: Not enforceable, depends on trust

### 5.2 BingoCube Solution: Dual-Key Encryption

#### Architecture

```
Standard Medical Data:
- Encrypted with patient's key: K_medical
- Patient can decrypt and read
- Patient can share with new providers

Professional Notes (Psych, Therapy):
- Encrypted with TWO keys: K_medical + K_professional_seal
- Patient verifies they OWN the data (can match subcube)
- Patient CANNOT decrypt alone (missing professional key)
- Patient CAN share with another professional (who can unseal)
- Professional seal ensures clinical context maintained
```

#### Professional Courtesy Pattern

```rust
// Psychologist creates notes
struct PsychologistNotes {
    content: String,
    patient_identity: SubCube,  // subcube(0.3) for verification
    professional_seal: ProfessionalKey,
}

impl PsychologistNotes {
    pub fn seal_for_patient(
        content: String,
        patient_cube: &BingoCube,
        psychologist_key: &ProfessionalKey
    ) -> SealedNotes {
        // Derive patient's medical key
        let patient_key = KDF(patient_cube.hash(), "MEDICAL_PSYCH");
        
        // Combine with professional seal
        let dual_key = combine_keys(patient_key, psychologist_key);
        
        // Encrypt with both keys
        let encrypted = Encrypt(content, dual_key);
        
        SealedNotes {
            data: encrypted,
            patient_proof: patient_cube.subcube(0.3),  // For verification
            sealed_by: psychologist_key.public_id(),
            seal_type: "PROFESSIONAL_THERAPEUTIC",
            access_policy: "PATIENT_OWNS_PROFESSIONAL_UNSEALS",
        }
    }
}

// Patient interacts with sealed notes
impl Patient {
    pub fn verify_i_own_these_notes(&self, notes: &SealedNotes) -> bool {
        // Regenerate identity
        let my_cube = self.regenerate_identity();
        
        // Verify it's mine
        my_cube.subcube(0.3) == notes.patient_proof
    }
    
    pub fn can_i_read_these_notes(&self) -> bool {
        // By design: NO
        // Missing professional seal
        false
    }
    
    pub fn share_with_new_therapist(
        &self,
        notes: &SealedNotes,
        new_therapist: &Therapist
    ) -> Result<TransferPackage> {
        // Verify ownership
        if !self.verify_i_own_these_notes(notes) {
            return Err("Not your notes");
        }
        
        // Package for transfer (still sealed!)
        Ok(TransferPackage {
            sealed_notes: notes.clone(),
            patient_authorization: self.sign_authorization(),
            transfer_to: new_therapist.public_id(),
            timestamp: Timestamp::now(),
        })
    }
}

// New therapist unseals notes
impl Therapist {
    pub fn unseal_transferred_notes(
        &self,
        package: TransferPackage,
        professional_key: &ProfessionalKey
    ) -> Result<String> {
        // Verify patient authorization
        verify_signature(package.patient_authorization)?;
        
        // Verify professional credentials
        verify_professional(self.credentials)?;
        
        // Unseal with professional key
        let notes = decrypt_with_seal(
            package.sealed_notes.data,
            professional_key
        )?;
        
        // Log access
        audit_log("PROFESSIONAL_UNSEAL", self.id, package.patient_id);
        
        Ok(notes)
    }
}
```

### 5.3 Patient Journey

#### Initial Therapy Session

```
1. Patient arrives for first therapy session
   Therapist: "I'll create your medical record."
   Therapist: "Per ethical guidelines, notes are sealed."
   Therapist: "You own them, but they're professionally sealed."
   Patient: "What does that mean?"
   Therapist: "You can verify they're yours, and share with another therapist."
   Therapist: "But you won't read them - maintains therapeutic relationship."
   Patient: "I trust that. Let's proceed."

2. Identity Establishment
   [Patient touches biometric scanner]
   System: [generates identity cube]
   Patient shown pattern:
   
   █ █ █ █ █
   █ █ █ █ █    "This is your medical identity"
   █ █ ✱ █ █
   █ █ █ █ █
   █ █ █ █ █

3. Session Notes Created
   Therapist types session notes:
   "Patient presents with anxiety re: job loss.
    Discussed coping mechanisms.
    Recommended CBT techniques.
    Next session: explore childhood experiences."
   
   System: [encrypts with patient_key + professional_seal]
   System: [stores in patient's vault]
   
   Patient vault now contains:
   {
     "psych_notes_session_1": {
       "encrypted": "...",  // Dual-key encrypted
       "patient_proof": subcube(0.3),
       "sealed_by": "Dr. Smith, PhD, Lic#12345",
       "date": "2025-12-01",
       "can_patient_read": false,
       "can_patient_verify_ownership": true,
       "can_patient_share": true
     }
   }

4. Patient Verification (Optional)
   Patient: "Can I verify those are my notes?"
   System: [regenerates cube from biometric]
   System: [compares to notes.patient_proof]
   System: "✅ These notes belong to you."
   Patient: "Good. I won't read them, but good to know they're mine."
```

#### Moving to New Therapist

```
1. Patient moves to new city, finds new therapist
   Patient: "I have therapy notes from my previous therapist."
   New Therapist: "Great! Can you share them?"
   Patient: "Yes, they're in my medical vault."

2. Identity Verification
   [Patient touches scanner at new clinic]
   System: [regenerates identity cube]
   System: [verifies patient ownership of vault]
   System: "✅ Vault belongs to this patient."

3. Professional Transfer
   Patient: "I authorize Dr. Jones to read my notes from Dr. Smith."
   System: [creates transfer package]
   System: [sends to Dr. Jones with patient signature]
   
   Transfer Package:
   {
     "sealed_notes": "[still encrypted with dual keys]",
     "patient_authorization": "[digital signature]",
     "original_therapist": "Dr. Smith",
     "transferring_to": "Dr. Jones",
     "patient_consent": true,
   }

4. New Therapist Unseals
   Dr. Jones: [verifies professional credentials]
   Dr. Jones: [uses professional key to unseal]
   Dr. Jones: [reads notes from Dr. Smith]
   
   Dr. Jones: "Thank you. This gives me important context."
   Dr. Jones: "I'll add my own sealed notes to your vault."
   
   [Continuity of care maintained]
   [Therapeutic relationship preserved]
   [Patient privacy respected]
```

#### Patient Wants to Read Notes (Edge Case)

```
Patient: "I changed my mind. I want to read my psych notes."

System: "These notes are professionally sealed."
System: "To maintain therapeutic efficacy, we recommend:"
System: "1. Discuss with your current therapist"
System: "2. Request summary (not full notes)"
System: "3. Seek professional guidance"

Patient: "I understand the recommendation, but I insist."

Therapist: "I respect your sovereignty over your data."
Therapist: "Here's what I can do:"
Therapist: "1. I can unseal and read them TO you (with context)"
Therapist: "2. I can unseal and give you a summary"
Therapist: "3. Or I can unseal fully (with professional guidance present)"

Patient chooses option 3.

Therapist: [unseals notes with professional key]
Therapist: [reads notes WITH patient, providing context]
Therapist: [discusses any concerning content]
Therapist: [ensures therapeutic relationship maintained]

[Patient got access]
[But with professional support]
[Relationship preserved]
```

### 5.4 Data Vault Structure

```javascript
{
  "medical_identity": {
    "cube": "[visual pattern]",
    "proof": "subcube(0.5)",  // Higher trust for medical
    "created": "2025-06-01"
  },
  
  "medical_records": {
    "general": {
      "hospital_visits": [
        {
          "date": "2025-08-15",
          "provider": "General Hospital",
          "reason": "Annual checkup",
          "encrypted_with": "K_medical",
          "patient_can_read": true
        }
      ],
      "lab_results": [
        {
          "date": "2025-08-15",
          "test": "Blood panel",
          "results": "[encrypted]",
          "encrypted_with": "K_medical",
          "patient_can_read": true
        }
      ]
    },
    
    "psychological": {
      "sessions": [
        {
          "date": "2025-12-01",
          "provider": "Dr. Smith, PhD",
          "notes": "[encrypted with K_medical + K_professional_seal]",
          "encrypted_with": "DUAL_KEY",
          "patient_can_verify_ownership": true,
          "patient_can_read": false,
          "patient_can_share": true,
          "professional_seal": "Dr. Smith Lic#12345"
        },
        {
          "date": "2025-12-08",
          "provider": "Dr. Smith, PhD",
          "notes": "[encrypted with K_medical + K_professional_seal]",
          "encrypted_with": "DUAL_KEY"
        }
      ],
      
      "unsealing_policy": {
        "who_can_unseal": "Licensed mental health professional",
        "patient_consent_required": true,
        "audit_logged": true,
        "professional_guidance_recommended": true
      }
    }
  },
  
  "access_log": [
    {"provider": "Dr. Smith", "action": "create_sealed_notes", "date": "2025-12-01"},
    {"provider": "Dr. Jones", "action": "unseal_transferred_notes", "date": "2025-12-15"}
  ]
}
```

### 5.5 Ethical Benefits

1. **Patient Sovereignty**
   - Patient owns ALL data (even sealed notes)
   - Patient can verify ownership (cryptographically)
   - Patient controls sharing (grant/revoke access)
   - Patient can export entire vault (portability)

2. **Professional Integrity**
   - Therapist can document honestly (sealed from patient)
   - Clinical context maintained (professional-to-professional)
   - Ethical guidelines respected (not just social convention)
   - Continuity of care enabled (transferable with consent)

3. **Relationship Preservation**
   - Patient doesn't accidentally read harmful content
   - Therapist doesn't self-censor documentation
   - Trust maintained through cryptography (not just policy)
   - Professional courtesy enforced by mathematics

4. **Legal Compliance**
   - Patient legally owns records (satisfies regulations)
   - Professional standards maintained (ethical guidelines)
   - Audit trail for all access (compliance)
   - Consent-based sharing (HIPAA/GDPR compliant)

### 5.6 Comparison to Current System

| Aspect | Current System | BingoCube System |
|--------|----------------|------------------|
| **Ownership** | Provider owns | Patient owns |
| **Portability** | Manual requests, fax, CDs | Instant, encrypted file |
| **Access Control** | Provider decides | Patient decides |
| **Psych Notes** | Social convention | Cryptographic seal |
| **Cross-Provider** | Manual coordination | Zero-knowledge sharing |
| **Privacy** | Central DB, employee access | Encrypted vault, audit log |
| **Identity** | Photo ID, SSN | Biometric-seeded cube |
| **Professional Ethics** | Policy-based | Cryptography-based |

---


---

### 6.4 Cross-Organization Transfer Protocol

```rust
pub struct TransferProtocol {
    source_org: OrgIdentity,
    target_org: OrgIdentity,
}

impl TransferProtocol {
    /// Organization A creates transfer token for Organization B
    pub fn create_token(
        source_org: &OrgIdentity,
        identity_package: &IdentityPackage,
        target_org: &OrgIdentity,
        reveal_level: f64,
        duration: Duration,
    ) -> Result<TransferToken> {
        let proof = identity_package.visual_identity.subcube(reveal_level)?;
        
        let token = TransferToken {
            proof: proof.clone(),
            source_org: source_org.clone(),
            target_org: target_org.clone(),
            issued_at: Timestamp::now(),
            expires_at: Timestamp::now() + duration,
            reveal_level,
            signature: source_org.sign(&proof)?,
        };
        
        Ok(token)
    }
    
    /// Organization B verifies token and user
    pub fn verify_token(
        token: &TransferToken,
        claimed_biometric: &BiometricData,
        config: &CubeConfig,
    ) -> Result<VerificationResult> {
        // 1. Check token validity
        if Timestamp::now() > token.expires_at {
            return Ok(VerificationResult::Failed {
                reason: "Token expired".to_string(),
            });
        }
        
        // 2. Verify source org signature
        token.source_org.verify_signature(&token.proof, &token.signature)?;
        
        // 3. Regenerate cube from biometric
        let entropy = Entropy::generate();
        let seed = derive_seed(claimed_biometric, &entropy)?;
        let cube = BingoCube::from_seed(&seed, config.clone())?;
        
        // 4. Compare at token's reveal level
        let claimed = cube.subcube(token.reveal_level)?;
        
        if claimed == token.proof {
            Ok(VerificationResult::Success {
                trust_level: Self::reveal_to_trust(token.reveal_level),
                confidence: token.reveal_level,
                timestamp: Timestamp::now(),
            })
        } else {
            Ok(VerificationResult::Failed {
                reason: "Cube mismatch with transfer token".to_string(),
            })
        }
    }
    
    /// Request data transfer from source org (with user consent)
    pub fn request_data_transfer(
        source_org: &OrgIdentity,
        target_org: &OrgIdentity,
        user_consent: UserConsent,
        domains: Vec<DataDomain>,
    ) -> Result<DataTransferPackage> {
        // Verify user consent
        user_consent.verify_signature()?;
        user_consent.verify_not_expired()?;
        
        // Source org packages requested data
        let encrypted_data = source_org.get_encrypted_data(domains.clone())?;
        
        // Wrap keys with target org's public key
        let wrapped_keys = domains.iter()
            .map(|d| {
                let key = source_org.get_key_for_domain(d)?;
                target_org.public_key.wrap(&key)
            })
            .collect::<Result<Vec<_>>>()?;
        
        Ok(DataTransferPackage {
            data_blobs: encrypted_data,
            wrapped_keys,
            domains,
            source_org: source_org.clone(),
            target_org: target_org.clone(),
            user_consent: user_consent.signature,
            timestamp: Timestamp::now(),
        })
    }
    
    fn reveal_to_trust(reveal: f64) -> TrustLevel {
        if reveal >= 0.8 { TrustLevel::High }
        else if reveal >= 0.4 { TrustLevel::Medium }
        else { TrustLevel::Low }
    }
}

pub struct TransferToken {
    proof: SubCube,
    source_org: OrgIdentity,
    target_org: OrgIdentity,
    issued_at: Timestamp,
    expires_at: Timestamp,
    reveal_level: f64,
    signature: Signature,
}

pub struct DataTransferPackage {
    data_blobs: Vec<EncryptedBlob>,
    wrapped_keys: Vec<WrappedKey>,
    domains: Vec<DataDomain>,
    source_org: OrgIdentity,
    target_org: OrgIdentity,
    user_consent: Signature,
    timestamp: Timestamp,
}
```

---


---

### 7.1 BearDog (Identity Primal)

**Role**: Provide biometric identity primitives using BingoCube

**Capabilities**:
- `identity.establish` - Create new identity from biometric
- `identity.verify` - Verify identity at trust level
- `identity.transfer_token` - Generate cross-org tokens
- `identity.vault_init` - Initialize sovereign vault

**Example API**:
```rust
// Establish identity
POST /api/identity/establish
Body: {
    "consent": "user_signature",
    "cube_config": {
        "grid_size": 8,
        "palette_size": 256
    }
}
Response: {
    "identity_package": {
        "visual_pattern": "█ █ █ █ █...",
        "public_proof": "[subcube(0.3)]",
        "vault_id": "...",
        "created_at": "..."
    }
}

// Verify identity
POST /api/identity/verify
Body: {
    "stored_proof": "[subcube data]",
    "trust_level": "medium"
}
Response: {
    "verified": true,
    "confidence": 0.5,
    "trust_level": "medium"
}
```

### 7.2 NestGate (Storage Primal)

**Role**: Store and manage sovereign vaults

**Capabilities**:
- `vault.store` - Store encrypted vault
- `vault.retrieve` - Retrieve vault by identity proof
- `vault.update` - Update vault contents
- `vault.grant_access` - Grant temp access to org

**Integration**:
```rust
// User's vault stored encrypted at NestGate
// NestGate CANNOT decrypt (no keys)
// NestGate CAN verify ownership (via identity proof)

impl NestGate {
    pub fn store_vault(
        &self,
        vault: SovereignVault,
        identity_proof: SubCube,
    ) -> Result<VaultId> {
        // Verify vault belongs to identity
        if vault.metadata.identity_proof != identity_proof {
            return Err("Identity mismatch");
        }
        
        // Store vault (encrypted!)
        let vault_id = self.storage.store(vault)?;
        
        // Index by identity proof (for retrieval)
        self.index.insert(identity_proof, vault_id);
        
        Ok(vault_id)
    }
    
    pub fn retrieve_vault(
        &self,
        identity_proof: SubCube,
        bearer_token: BearerToken,
    ) -> Result<SovereignVault> {
        // Verify bearer token
        bearer_token.verify()?;
        
        // Lookup vault
        let vault_id = self.index.get(&identity_proof)
            .ok_or("Vault not found")?;
        
        let vault = self.storage.retrieve(*vault_id)?;
        
        // Return encrypted vault
        // Bearer must have keys to decrypt!
        Ok(vault)
    }
}
```

### 7.3 Songbird (Discovery Primal)

**Role**: Help organizations discover each other for data transfer

**Capabilities**:
- `org.register` - Register organization capabilities
- `org.discover` - Find orgs offering specific services
- `transfer.coordinate` - Coordinate cross-org transfers

**Integration**:
```rust
// User at Org B wants to connect with Org A
// Songbird helps discover Org A

impl Songbird {
    pub fn discover_org_for_user(
        &self,
        user_proof: SubCube,
        service_type: ServiceType,
    ) -> Result<Vec<OrgIdentity>> {
        // Find organizations that have records for this user
        // WITHOUT revealing user's full identity
        
        let orgs = self.registry.find_by_service(service_type)?;
        
        // Organizations can optionally register partial proofs
        // "We have records for users matching [proof_pattern]"
        let matching = orgs.into_iter()
            .filter(|org| org.has_user_matching(user_proof))
            .collect();
        
        Ok(matching)
    }
}
```

### 7.4 ToadStool (Compute Primal)

**Role**: Perform computation on encrypted data (optional)

**Capabilities**:
- `compute.on_encrypted` - FHE/MPC computation
- `compute.aggregate` - Aggregate stats without decryption

**Integration**:
```rust
// Example: Homeless services wants aggregate stats
// WITHOUT decrypting individual records

impl ToadStool {
    pub fn aggregate_stats(
        &self,
        encrypted_vaults: Vec<EncryptedBlob>,
    ) -> Result<AggregateStats> {
        // Homomorphic encryption allows computation on encrypted data
        // Can compute:
        // - Average age of population
        // - Service utilization rates
        // - Geographic distribution
        // WITHOUT decrypting individual records
        
        let stats = self.fhe_engine.aggregate(encrypted_vaults)?;
        
        Ok(AggregateStats {
            total_users: stats.count,
            avg_age: stats.avg_age,
            service_usage: stats.service_rates,
            // Individual identities never revealed!
        })
    }
}
```


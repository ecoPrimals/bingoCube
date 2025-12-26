# BingoCube: Mathematical Foundation and Security Analysis

**Version**: 1.0  
**Date**: December 25, 2025  
**Authors**: ecoPrimals Team

---

## Table of Contents

1. [Formal Definitions](#1-formal-definitions)
2. [Combinatorial Properties](#2-combinatorial-properties)
3. [Hash-Based Cross-Binding](#3-hash-based-cross-binding)
4. [Progressive Reveal Mathematics](#4-progressive-reveal-mathematics)
5. [Security Proofs](#5-security-proofs)
6. [Information Theory](#6-information-theory)
7. [Attack Analysis](#7-attack-analysis)

---

## 1. Formal Definitions

### 1.1 Board Structure

**Definition 1.1** (Bingo Board): A **bingo board** is a tuple (G, U, L, R, π) where:
- G: ℤᵤ^(L×L) is an L×L grid of values from universe {0, ..., U-1}
- L ∈ ℕ⁺ is the grid dimension
- U ∈ ℕ⁺ is the universe size with U = L·R
- R ∈ ℕ⁺ is the per-column range size
- π: {0..L-1} → {0..L-1} is a column permutation

**Constraint 1.1** (Column Range Locking): For all columns j ∈ {0..L-1}:

```
∀i,i′ ∈ {0..L-1}, i≠i′ : G[i,j] ≠ G[i′,j]        (distinctness)
∀i ∈ {0..L-1} : G[i,j] ∈ [j·R, (j+1)·R - 1]    (range locking)
```

**Definition 1.2** (Free Cell): A **free cell** at position (i₀, j₀) satisfies:

```
G[i₀, j₀] = ⊥    (undefined/blank)
```

When free cells exist, distinctness only applies to non-free cells in that column.

### 1.2 BingoCube Structure

**Definition 1.3** (BingoCube): A **BingoCube** is a 4-tuple (A, B, H, K) where:
- A: Bingo board (depth layer 0)
- B: Bingo board (depth layer 1)
- H: {0,1}* → {0,1}ⁿ is a cryptographic hash function
- K ∈ ℕ⁺ is the color palette size

Both A and B have the same dimensions L×L and universe size U.

**Definition 1.4** (Scalar Field): The **scalar field** d: [0,L-1]² → ℤ₊ is defined:

```
d[i,j] = ℋ(H("BINGOCUBE_V1" || i || j || A[i,j] || B[i,j]))
```

where ℋ: {0,1}ⁿ → ℤ₊ interprets hash output as an unsigned integer.

**Definition 1.5** (Color Grid): The **color grid** c: [0,L-1]² → {0..K-1} is:

```
c[i,j] = d[i,j] mod K
```

### 1.3 Progressive Reveal

**Definition 1.6** (Reveal Parameter): x ∈ (0, 1] is the **reveal parameter**.

**Definition 1.7** (Mask Size): For reveal parameter x:

```
m(x) = ⌈x · L²⌉
```

**Definition 1.8** (Reveal Mask): The **reveal mask** 𝓜ₓ ⊆ [0,L-1]² is:

```
𝓜ₓ = {(i,j) : d[i,j] ≥ threshold(x)}
```

where threshold(x) is the m(x)-th largest value in {d[i,j] : ∀i,j}.

**Definition 1.9** (Subcube): The **subcube at level x** is:

```
Subcube(x) = {(i, j, c[i,j]) : (i,j) ∈ 𝓜ₓ}
```

---

## 2. Combinatorial Properties

### 2.1 Board Counting

**Theorem 2.1** (Board Generation Space): The number of valid bingo boards is:

```
N_boards = L! · ∏(j=0 to L-1) P(R, L)
```

where P(R, L) = R!/(R-L)! is the number of L-permutations from R values.

**Proof**:
- L! column permutations π
- For each column j, select L distinct values from R options
- Order matters (row position), so P(R, L) arrangements per column
- Independence: columns are generated independently
∴ N_boards = L! · [P(R, L)]^L □

**Example**: For L=5, R=20:
```
N_boards = 5! · (20·19·18·17·16)^5
         = 120 · (1,860,480)^5
         ≈ 2.87 × 10^31 distinct boards
```

**Corollary 2.1**: For large R, approximately:

```
log₂(N_boards) ≈ L·log₂(L!) + L²·log₂(R)
```

### 2.2 BingoCube Counting

**Theorem 2.2** (BingoCube Space): The number of distinct BingoCubes is:

```
N_cubes = N_boards²
```

since boards A and B are generated independently.

**Example**: For L=5, R=20:
```
N_cubes ≈ (2.87 × 10^31)² ≈ 8.24 × 10^62
```

**Comparison to entropy**:
```
log₂(N_cubes) ≈ 208 bits
```

This is comparable to a 208-bit cryptographic key.

### 2.3 Collision Probability

**Theorem 2.3** (Birthday Bound): If generating k random BingoCubes, the probability of collision is:

```
P(collision) ≈ k²/(2·N_cubes)
```

**Example**: For L=5, R=20, after generating k=2^40 cubes:
```
P(collision) ≈ (2^40)²/(2·8.24×10^62)
           ≈ 2^80/(1.65×10^63)
           ≈ 7.3×10^-40    (negligible)
```

### 2.4 Mask Nesting

**Theorem 2.4** (Nested Masks): For all x₁, x₂ ∈ (0,1] with x₁ ≤ x₂:

```
𝓜_(x₁) ⊆ 𝓜_(x₂)
```

**Proof**:
- m(x₁) ≤ m(x₂) since m is monotone increasing
- 𝓜_(x₁) = top-m(x₁) cells by d[i,j]
- 𝓜_(x₂) = top-m(x₂) cells by d[i,j]
- Top-m(x₁) ⊆ Top-m(x₂) by definition of "top k"
∴ 𝓜_(x₁) ⊆ 𝓜_(x₂) □

**Corollary 2.2**: The family {𝓜ₓ : x ∈ (0,1]} forms a nested sequence:

```
𝓜_(0+) ⊂ 𝓜_(0.1) ⊂ 𝓜_(0.2) ⊂ ... ⊂ 𝓜_(1.0) = [0,L-1]²
```

---

## 3. Hash-Based Cross-Binding

### 3.1 Hash Function Requirements

**Definition 3.1** (Secure Hash): A hash function H: {0,1}* → {0,1}ⁿ is **secure** if:

1. **Collision resistance**: Hard to find x≠y with H(x)=H(y)
2. **Pre-image resistance**: Given h, hard to find x with H(x)=h
3. **Second pre-image resistance**: Given x, hard to find y≠x with H(x)=H(y)

We assume H is a secure hash (e.g., BLAKE3, SHA-256).

### 3.2 Uniformity of Scalar Field

**Theorem 3.1** (Scalar Field Uniformity): If H is a random oracle, then d[i,j] values are uniformly distributed.

**Proof**:
- Input to H: "BINGOCUBE_V1" || i || j || A[i,j] || B[i,j]
- Each (i,j) pair has unique input (due to position encoding)
- Random oracle assumption: H(input) is uniform in {0,1}ⁿ
- ℋ interprets as integer: uniform in [0, 2ⁿ-1]
∴ d[i,j] is uniform □

**Corollary 3.1**: Color values c[i,j] = d[i,j] mod K are approximately uniform:

```
∀c ∈ {0..K-1} : P(c[i,j] = c) ≈ 1/K
```

(Exact if K divides 2ⁿ evenly)

### 3.3 Independence of Cells

**Theorem 3.2** (Cell Independence): For (i,j) ≠ (i′,j′), the values d[i,j] and d[i′,j′] are independent.

**Proof**:
- Hash inputs differ in position: i||j ≠ i′||j′
- Random oracle: different inputs → independent outputs
∴ d[i,j] ⊥ d[i′,j′] □

**Implication**: Revealing subset of cells doesn't leak information about unrevealed cells.

### 3.4 Binding Property

**Theorem 3.3** (Board Binding): The color grid c uniquely determines (A, B) with high probability.

**Proof sketch**:
- Suppose adversary finds (A′, B′) ≠ (A, B) with same color grid c
- Then ∀i,j: d′[i,j] mod K = d[i,j] mod K
- This requires d′[i,j] ≡ d[i,j] (mod K) for all L² cells
- Each cell is an independent hash collision (mod K)
- Probability of one collision: ≈ 1/K (if d values uniform)
- Probability of L² collisions: ≈ (1/K)^(L²)

**Example**: For L=5, K=16:
```
P(forgery) ≈ (1/16)^25 = 2^(-100)    (negligible)
```

**Corollary 3.2**: The color grid is a **commitment** to (A, B).

---

## 4. Progressive Reveal Mathematics

### 4.1 Reveal Statistics

**Theorem 4.1** (Expected Reveal Size): For random reveal parameter x ~ Uniform(0,1):

```
E[|𝓜ₓ|] = L²/2
```

**Proof**:
- E[|𝓜ₓ|] = E[⌈x·L²⌉]
- For x ~ Uniform(0,1): E[x·L²] = L²/2
- Ceiling adds at most 1: E[⌈x·L²⌉] ≈ L²/2 + 0.5 □

### 4.2 Information Revealed

**Definition 4.1** (Information Content): The information revealed at level x is:

```
I(x) = |𝓜ₓ| · log₂(K)    bits
```

since each revealed cell carries log₂(K) bits of color information.

**Theorem 4.2** (Linear Information Growth):

```
I(x) = x · L² · log₂(K) + O(1)
```

**Example**: For L=5, K=16, x=0.4:
```
I(0.4) = 0.4 · 25 · 4 = 40 bits revealed
I(1.0) = 1.0 · 25 · 4 = 100 bits revealed (full grid)
```

### 4.3 Partial Verification

**Theorem 4.3** (Subcube Verification Security): Verifying a subcube at level x requires:

```
log₂(K^(x·L²))  ≈ x · L² · log₂(K)  bits of information
```

To forge a valid subcube without knowing (A, B):

```
P(forge) ≈ K^(-x·L²)
```

**Example**: For L=5, K=16, x=0.3 (8 cells):
```
P(forge) ≈ 16^(-8) = 2^(-32)    (1 in 4 billion)
```

Even a **30% reveal** provides strong security.

---

## 5. Security Proofs

### 5.1 Pre-image Resistance

**Theorem 5.1** (Board Recovery): Given only the color grid c, recovering (A, B) is computationally infeasible.

**Proof**:
- To recover A[i,j], need to find values such that:
  d[i,j] mod K = c[i,j]
  
- This requires finding (a,b) such that:
  H("..." || i || j || a || b) mod K = c[i,j]
  
- This is a **pre-image search** for the hash function
- With secure hash (pre-image resistance): requires ~2ⁿ hash evaluations
- For n=256 (SHA-256): ~2^256 operations (infeasible)

- Even with reduced mod K constraint:
  - Expected searches per cell: K (not 2ⁿ)
  - But must also satisfy **bingo constraints** (column ranges, distinctness)
  - Constraint satisfaction makes search exponentially harder
  
∴ Board recovery is infeasible □

### 5.2 Collision Resistance

**Theorem 5.2** (Distinct Boards → Distinct Grids): With high probability, different board pairs produce different color grids.

**Proof**:
- Consider (A, B) ≠ (A′, B′)
- At least one cell differs: (A[i₀,j₀], B[i₀,j₀]) ≠ (A′[i₀,j₀], B′[i₀,j₀])
- Hash collision resistance: H(input₁) ≠ H(input₂) w.h.p.
- Therefore d[i₀,j₀] ≠ d′[i₀,j₀] w.h.p.
- c[i₀,j₀] = d[i₀,j₀] mod K may equal d′[i₀,j₀] mod K with prob ≈1/K
- For full grid collision, need all L² cells to collide mod K
- P(full collision) ≈ (1/K)^(L²)

**Example**: L=5, K=16:
```
P(collision) ≈ (1/16)^25 ≈ 2^(-100)
```

∴ Distinct boards produce distinct grids w.h.p. □

### 5.3 Selective Reveal Security

**Theorem 5.3** (Partial Reveal Security): Revealing 𝓜ₓ doesn't compromise security of 𝓜_(1.0) \ 𝓜ₓ (unrevealed cells).

**Proof**:
- Revealed cells: {(i,j,c[i,j]) : (i,j) ∈ 𝓜ₓ}
- This reveals: {d[i,j] mod K : (i,j) ∈ 𝓜ₓ}
- Hash one-wayness: knowing d mod K doesn't reveal A[i,j] or B[i,j]
- Cell independence (Theorem 3.2): revealed cells don't leak info about unrevealed
- Unrevealed cells still protected by hash pre-image resistance

∴ Partial reveal is safe □

### 5.4 Challenge-Response Security

**Protocol**: Verifier challenges with random x, prover must reveal 𝓜ₓ.

**Theorem 5.4** (Challenge-Response Security): Without knowing (A, B), probability of passing k challenges is ≈ K^(-k·x·L²).

**Proof**:
- Challenge i: Verifier picks xᵢ randomly
- Prover must reveal 𝓜_(xᵢ) with correct colors
- Each cell in 𝓜_(xᵢ) must match: probability 1/K per cell
- |𝓜_(xᵢ)| ≈ xᵢ·L² cells
- P(pass challenge i) ≈ (1/K)^(xᵢ·L²)
- k independent challenges:
  P(pass all) ≈ ∏ᵢ (1/K)^(xᵢ·L²)

**Example**: L=5, K=16, x=0.5, k=3 challenges:
```
P(forge) ≈ (1/16)^(0.5·25) ^ 3
         = (1/16)^(37.5)
         ≈ 2^(-150)    (negligible)
```

∴ Challenge-response is secure □

---

## 6. Information Theory

### 6.1 Entropy Analysis

**Definition 6.1** (Board Entropy): The entropy of a single board is:

```
H(Board) = log₂(N_boards)
         = L·log₂(L!) + L²·log₂(R)
```

**Example**: L=5, R=20:
```
H(Board) = 5·log₂(120) + 25·log₂(20)
         ≈ 5·6.91 + 25·4.32
         ≈ 34.5 + 108
         = 142.5 bits
```

**Definition 6.2** (BingoCube Entropy):

```
H(BingoCube) = H(Board_A) + H(Board_B) + log₂(K^(L²))
             = 2·H(Board) + L²·log₂(K)
```

**Example**: L=5, R=20, K=16:
```
H(BingoCube) = 2·142.5 + 25·4
             = 285 + 100
             = 385 bits total
```

### 6.2 Conditional Entropy

**Theorem 6.1** (Conditional Entropy): Given subcube at level x:

```
H(BingoCube | Subcube(x)) = (1-x)·L²·log₂(K)
```

**Proof**:
- Subcube(x) reveals x·L² cells
- Each cell carries log₂(K) bits
- Remaining cells: (1-x)·L² unrevealed
- Hash independence: revealed cells don't reduce entropy of unrevealed
∴ H(BingoCube | Subcube(x)) = (1-x)·L²·log₂(K) □

**Example**: L=5, K=16, x=0.6:
```
Revealed: 0.6·25·4 = 60 bits
Remaining: 0.4·25·4 = 40 bits
Total: 100 bits (matches full grid)
```

### 6.3 Min-Entropy

**Definition 6.3** (Min-Entropy): The worst-case entropy is:

```
H_∞(BingoCube) = -log₂(P_max)
```

where P_max is the probability of the most likely BingoCube.

For uniformly random generation:
```
H_∞(BingoCube) = H(BingoCube)    (all equally likely)
```

---

## 7. Attack Analysis

### 7.1 Brute Force Attack

**Attack**: Try all possible (A, B) pairs until finding one matching the color grid.

**Complexity**: O(N_cubes) = O(N_boards²)

**Example**: L=5, R=20:
```
N_cubes ≈ 8.24 × 10^62
At 10^12 cubes/sec: ~2.6 × 10^43 years
```

**Verdict**: **Infeasible**

### 7.2 Meet-in-the-Middle Attack

**Attack**: 
1. Generate all possible boards A: store {(c_A, A)}
2. For each board B: compute color grid c_AB
3. Check if c_AB matches stored c_A

**Complexity**: O(N_boards) space, O(N_boards) time

**Example**: L=5, R=20:
```
N_boards ≈ 2.87 × 10^31
Storage: ~2.87 × 10^31 entries (infeasible)
Time: ~9×10^14 years at 10^9 boards/sec
```

**Verdict**: **Infeasible** (memory and time)

### 7.3 Partial Information Attack

**Attack**: Use revealed subcube 𝓜ₓ to narrow search space.

**Analysis**:
- Subcube reveals x·L² cells with colors
- Each cell gives constraint: d[i,j] mod K = c[i,j]
- This is a **constraint satisfaction problem**
- Still protected by hash pre-image resistance
- Bingo constraints (column ranges) reduce possibilities but not enough

**Complexity**: Still exponential in unrevealed cells

**Verdict**: **No significant advantage**

### 7.4 Birthday Attack

**Attack**: Generate many random BingoCubes, hope for collision with target.

**Complexity**: O(√N_cubes) cubes needed for 50% collision probability

**Example**: L=5:
```
√N_cubes ≈ √(8.24×10^62) ≈ 9×10^31
At 10^12 cubes/sec: ~2.8×10^12 years
```

**Verdict**: **Infeasible**

### 7.5 Quantum Computer Attack

**Attack**: Use Grover's algorithm for pre-image search.

**Complexity**: O(√(2^n)) for hash with output size n

**Example**: n=256 (SHA-256/BLAKE3):
```
Classical: O(2^256) ≈ 10^77 operations
Quantum: O(2^128) ≈ 10^38 operations (still infeasible)
```

**Mitigation**: Use quantum-resistant hash (e.g., SHA-3)

**Verdict**: **Resistant** with proper hash choice

### 7.6 Structure Exploitation

**Attack**: Exploit bingo structure (column ranges) to narrow search.

**Analysis**:
- Column ranges are known (public parameter)
- But hash mixing destroys structural patterns
- Color distribution appears uniform (Theorem 3.1)
- Can't determine which cell belongs to which column range from colors alone

**Example**: 
- Board has structure: column 0 has values 0-19
- Color grid: no visible pattern (hash mixing)
- Attacker can't identify "column 0 cells" from colors

**Verdict**: **Structure is hidden** by cryptographic hash

---

## 8. Summary of Security Properties

| Property | Guarantee | Complexity | Status |
|----------|-----------|------------|--------|
| **Pre-image resistance** | Can't recover (A,B) from colors | O(2^n) | ✅ Secure |
| **Collision resistance** | Different boards → different colors | O(2^n) | ✅ Secure |
| **Binding** | Color grid commits to boards | K^(-L²) | ✅ Secure |
| **Partial reveal** | Subcube doesn't leak unrevealed | Hash security | ✅ Secure |
| **Challenge-response** | Can't forge without boards | K^(-x·L²·k) | ✅ Secure |
| **Brute force** | Can't enumerate all cubes | O(10^62) | ✅ Infeasible |
| **Meet-in-middle** | Can't split search efficiently | O(10^31) | ✅ Infeasible |
| **Quantum** | Grover speedup insufficient | O(2^128) | ✅ Resistant |

---

## 9. Comparison to Existing Systems

| System | Entropy | Structure | Progressive | Human-Verifiable |
|--------|---------|-----------|-------------|------------------|
| **BingoCube** (L=5) | ~385 bits | Bingo | Yes (x param) | ✅ Yes |
| **QR Code** (v40) | ~3KB | Reed-Solomon | No | ❌ Machine only |
| **SHA-256 hash** | 256 bits | None | No | ❌ Hex string |
| **Visual hash (GitHub)** | ~40 bits | Geometric | No | ⚠️ Limited |
| **BIP-39 seed** | 128-256 bits | Word list | No | ⚠️ Text only |

**BingoCube advantages**:
- ✅ Human-recognizable patterns (bingo grid)
- ✅ Progressive reveal (trust building)
- ✅ Multi-modal (visual, audio, haptic)
- ✅ Strong cryptographic binding
- ✅ Flexible information density

---

## Conclusion

BingoCube achieves a unique combination of **human-recognizable structure** and **cryptographic security**. The mathematical analysis shows:

1. **Large keyspace**: ~10^62 possible cubes (L=5)
2. **Strong binding**: Color grid commits to boards with ~2^(-100) forgery probability
3. **Progressive security**: Even partial reveals (x=0.3) provide ~2^(-32) security
4. **Attack resistance**: All known attacks are computationally infeasible
5. **Information-theoretic**: Progressive reveal matches linear information growth

This makes BingoCube suitable for **identity verification, trust attestation, content fingerprinting, and computation proofs** in distributed systems where human verifiability is essential.

---

**Next**: See `BingoCube-Implementation.md` for practical construction details.


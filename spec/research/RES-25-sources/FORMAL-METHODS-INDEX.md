# Formal Methods Integration: Document Index

> [!CAUTION]
> **PARTIALLY SUPERSEDED by RES-27** — Formal methods syntax integrated into `@specforge/software`, not a separate plugin. `specforge gen` deprecated. The test model analysis remains valid.
>
> **What changed:**
> - Separate `@specforge/formal-analysis` plugin → formal methods syntax is part of `@specforge/software`
> - `specforge gen` / codegen references → deprecated; AI agents consume entity graph directly
> - Proof obligation model and test model analysis remain valid

**Expert**: Expert 7 (Testing & Verification Specialist)
**Date**: 2026-03-04
**Research Corpus**: Design by Contract, B-Method, CSP

---

## Overview

This analysis explores how formal methods research (DbC, B-Method, CSP) can transform SpecForge's test model from **"coverage tracking"** to **"proof obligation management"**.

**Current state (SpecForge 1.0)**: Three-layer traceability (intent → linkage → proof) tracks whether tests exist and pass.

**Proposed evolution (SpecForge 2.0)**: Proof obligation discharge model that can **prove correctness**, not just "tests don't crash."

---

## Documents

### 1. Full Expert Analysis (30 KB, 727 lines)
**File**: `EXPERT-7-FORMAL-METHODS-TEST-MODEL.md`

**Contents**:
- Executive summary and core recommendations
- Design by Contract → property-based test generation
- B-Method refinement → test obligation matrices
- CSP trace semantics → trace-based test generation
- Mutation testing integration
- New verify kinds (contract, refinement, trace, deadlock_free, mutation)
- Coverage model evolution (from "has test" to "proof obligation discharge")
- Implementation roadmap (6 phases, 18-23 weeks)
- Risk assessment
- Competitive differentiation
- Before/after examples

**Read this for**: Comprehensive theoretical foundation + strategic rationale

---

### 2. Quick Reference (9 KB, 311 lines)
**File**: `FORMAL-METHODS-QUICK-REFERENCE.md`

**Contents**:
- One-page summary of core recommendations
- Three formal methods → three features table
- Side-by-side current vs proposed syntax
- New verify kinds summary table
- Coverage model comparison
- Implementation phases summary
- Risk mitigation checklist
- Key takeaways

**Read this for**: Executive summary + quick implementation guide

---

### 3. Rust Implementation Examples (30 KB, 695 lines)
**File**: `FORMAL-METHODS-RUST-EXAMPLES.md`

**Contents**:
- Concrete implementation examples for Rust
- Example 1: DbC contracts → property test patterns
- Example 2: Refinement testing (abstract → concrete behaviors)
- Example 3: CSP trace-based testing (valid/invalid traces)
- Example 4: Proof obligation report output format

**Read this for**: Concrete implementation patterns for formal methods in Rust

---

## Key Insights

### 1. Contracts Should Be Executable
**Current**: `contract """prose description"""`
**Proposed**: `contract { requires { ... } ensures { ... } invariants [...] }`

Structured contracts enable:
- Automatic test generation (one test per clause)
- Proof obligation extraction
- Mutation target generation

---

### 2. Coverage Should Measure Correctness
**Current metric**: "% of entities with passing tests"
**Proposed metric**: "% of proof obligations discharged"

| Level | Signal |
|-------|--------|
| UNPROVED | No contract block |
| PARTIAL | 50-99% POs discharged |
| PROVED | 100% POs discharged |
| STRONG | PROVED + mutation ≥80% |
| VERIFIED | STRONG + model checking pass |

---

### 3. Refinement Should Be First-Class
**New field**: `refines abstract_behavior_id`

Enables:
- Abstract specifications (contract only, no tests)
- Concrete implementations (inherit + strengthen abstract contract)
- Refinement proof obligations (precondition weakening, postcondition strengthening)
- Architectural intent tracking (abstract → concrete design)

---

### 4. Scenarios Should Have Formal Semantics
**Current**: `given/when/then "prose string"`
**Proposed**: `given "prose" event event_name(params)`

Enables:
- CSP model generation
- Model checking (deadlock/livelock detection)
- Trace-based test generation (valid traces succeed, invalid fail)
- Trace refinement verification

---

### 5. Mutation Testing Closes the Loop
**Insight**: Contracts define what mutations SHOULD be caught.

For each contract clause, generate a mutation:
- Remove precondition check → test should catch it
- Negate postcondition → test should catch it
- Violate invariant → property test should catch it

**Mutation coverage** = % of contract-derived mutants killed by test suite.

---

## New Verify Kinds

| Verify Kind | What It Tests | Generated From |
|------------|---------------|----------------|
| `verify contract` | All preconditions + postconditions | DbC contract block |
| `verify refinement` | Concrete satisfies abstract | `refines` field |
| `verify trace` | All scenarios are valid CSP traces | CSP-enhanced scenarios |
| `verify deadlock_free` | No partial trace deadlocks | CSP model checking |
| `verify liveness` | Eventual outcomes | CSP temporal properties |
| `verify mutation` | Test suite quality | Contract-derived mutations |

**Backward compatible**: Existing `verify unit/integration/property/load/e2e` unchanged.

---

## Implementation Priority

### Phase 1: DbC Contracts (3-4 weeks) — 🔥 HIGH PRIORITY
- Add `contract { requires/ensures/invariants }` to grammar
- Extract proof obligations
- Generate unit tests from clauses
- Update `specforge trace` to show PO discharge rate

**ROI**: Immediate value, low risk, foundational for all other phases.

### Phase 2: Property-Based Tests (2-3 weeks) — 🔥 HIGH PRIORITY
- Generate property tests from invariants
- Add `verify contract` kind
- Integrate with Proptest (Rust), fast-check (TypeScript)

**ROI**: Natural extension of Phase 1, high test quality improvement.

### Phase 3: Refinement (3-4 weeks) — ⭐ MEDIUM PRIORITY
- Add `refines` field + `abstract` bool
- Generate refinement POs
- Add `verify refinement` kind

**ROI**: Architectural value for complex systems, enables hierarchical specs.

### Phase 4-6: CSP + Mutation (9-12 weeks) — 💡 LOWER PRIORITY
- Event-tagged scenarios
- CSP model generation + FDR4/PAT integration
- Mutation testing integration

**ROI**: Advanced features for high-assurance systems, optional for most users.

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Complexity creep | All formal methods features are **opt-in**. Default behavior unchanged. |
| Steep learning curve | Templates, examples, agent guidance. Formal specs for advanced users only. |
| Tool dependency | CSP model checking optional. Core features work without external tools. |
| False positives | Conservative PO extraction. Under-extract rather than over-extract. |

---

## Competitive Position

**SpecForge 2.0 becomes the ONLY agent-first specification language with formal verification.**

| Feature | SpecForge 1.0 | SpecForge 2.0 | Dafny | TLA+ | Cucumber |
|---------|---------------|---------------|-------|------|----------|
| AI agent specs | ✅ | ✅ | ❌ | ❌ | ❌ |
| Auto test gen | Scaffolds | Full PBT + trace | ❌ | ❌ | ❌ |
| Proof obligations | Manual | Auto-extracted | Auto | Manual | ❌ |
| Refinement | ❌ | ✅ | ✅ | ✅ | ❌ |
| Mutation testing | ❌ | ✅ | ❌ | ❌ | ❌ |
| Model checking | ❌ | ✅ CSP | ❌ | ✅ TLC | ❌ |
| Learning curve | Low | Medium | Very high | Very high | Low |
| Any language | ✅ | ✅ | Dafny only | Separate spec | ✅ |

**Unique selling point**: Lightweight formal specs that AI agents can read AND generate provably correct tests from.

---

## Vision Statement

> SpecForge 2.0: **Proof-carrying specifications for AI agents**

Bridging the gap between:
- **Lightweight specs** (OpenAPI, GraphQL) ← too informal, no verification
- **Heavyweight verification** (Dafny, TLA+) ← too complex, not agent-friendly

**Sweet spot**: Formal enough to prove correctness, lightweight enough for AI agents to consume and generate from.

---

## Next Steps

1. **Review** all three documents (this index + full analysis + quick ref + examples)
2. **Prioritize** Phase 1 (DbC contracts) for immediate implementation
3. **Prototype** contract block syntax in grammar
4. **Validate** with SpecForge's own specs (self-hosting test case)
5. **Iterate** based on real-world usage patterns

---

## Research Sources

### Three Formal Methods Research Documents
1. **Design by Contract** (`research-design-by-contract.md`, 60.5 KB)
   - Bertrand Meyer, Eiffel, preconditions/postconditions/invariants
   - Relationship to testing: contracts as universal test oracles

2. **B-Method** (`research-b-method.md`, 75.7 KB)
   - Jean-Raymond Abrial, abstract machines, refinement
   - Proof obligations: invariant preservation, operation correctness

3. **CSP** (`research-csp.md`, 67.9 KB)
   - Tony Hoare, process algebra, trace semantics
   - Model checking: deadlock/livelock freedom, refinement checking

### SpecForge Specifications
- **RES-14**: Entity testability classification (5 testable entities)
- **RES-15**: Test declaration & traceability (verify + scenario dual syntax)
- **RES-17**: Rust plugin design (JUnit XML → specforge-report.json)

---

## Contact

**Expert 7**: Testing & Verification Specialist
**Specialties**: Property-based testing, mutation testing, formal verification, model checking, test adequacy criteria
**Date**: 2026-03-04

# 10-Expert Synthesis: Formal Methods Improvements for SpecForge

> [!CAUTION]
> **PARTIALLY SUPERSEDED by RES-27** — Key recommendations overridden:
> - `@specforge/formal-analysis` as separate plugin → integrated into `@specforge/software`
> - `contract`/`refinement`/`process` entities → rejected; inline syntax on existing entities
> - The formal methods analysis (DbC, B-Method, CSP) and verify kinds remain valid.

**Date:** March 4, 2026
**Based on:** Design by Contract, B-Method, Communicating Sequential Processes
**Method:** 10 independent expert analyses, synthesized below

---

## The One-Sentence Answer

**SpecForge already embodies all three formal methods implicitly — the improvement is making the compiler *aware* of the formal properties it's already modeling, via 4 new compiler passes, ~20 new validation codes, and inline syntax blocks (`requires`/`ensures`/`maintains`/`sync`) in `@specforge/software`.** ~~(Originally proposed as a separate `@specforge/formal-analysis` plugin — rejected by RES-27.)~~

---

## Expert Panel Summary

| # | Expert Role | Key Contribution | Document |
|---|------------|------------------|----------|
| 1 | **Formal Methods Theorist** | Hoare logic contracts, B-Method proof obligations, CSP process algebra — concrete DSL syntax + 19 new validation codes | `RES-22-formal-methods-improvements.md` |
| 2 | **Compiler/Language Designer** | 4 new compiler passes (contract_check → refinement_verify → process_analyze → proof_obligation), Rust implementation with algorithms | `RES-22-formal-methods-improvements.md` (extended) |
| 3 | **DX & Ergonomics Expert** | Progressive disclosure (Level 0-4), `specforge analyze` CLI, plain-English error messages, LSP ambient intelligence | (inline analysis) |
| 4 | **Safety-Critical Engineer** | SIL/DAL fields, proof obligation tracking, compliance report generation, 6-month path to SIL 3 certification | `safety-critical-analysis.md` |
| 5 | **AI/LLM Agent Specialist** | 90-95% token reduction with formal contracts, $445K-$662K annual enterprise savings, preconditions = highest ROI | `analysis-formal-methods-ai-agents.md` |
| 6 | **Type System Architect** | Refinement types (`amount: Money { > 0 }`), channel typing, contract subtyping, phantom types for entity IDs | `spec/research/RES-20-*.md` (5 documents) |
| 7 | **Testing & Verification** | Proof obligation coverage model, 6 new verify kinds, mutation testing, "UNPROVED → PARTIAL → PROVED → STRONG → VERIFIED" | `EXPERT-7-FORMAL-METHODS-TEST-MODEL.md` |
| 8 | **Product/Market Strategist** | "Never say formal methods" — lead with token economics, progressive formality like TypeScript, OSS core + paid compliance | (inline analysis) |
| 9 | **Distributed Systems Architect** | CSP = microservice choreography, deadlock detection in event graphs, consumer-driven contracts, saga patterns | `expert-9-distributed-systems-formal-methods.md` |
| 10 | **Plugin/Extension Designer** | ~~`@specforge/formal-analysis` as standard plugin~~ **[Superseded: formal methods syntax merged into `@specforge/software` (RES-27)]**, query extensions via Rhai, 9-week implementation | `formal-methods-plugin-design.md` + 4 supporting docs |

---

## Consensus: The 5 Things All Experts Agree On

### 1. DO NOT create separate extensions for DbC, B-Method, CSP

**Unanimous (10/10).** These are three lenses on the same model, not three separate systems.

- DbC = the **contract** of each entity (what it promises)
- B-Method = the **refinement chain** (how abstract specs become concrete)
- CSP = the **communication model** (how entities interact)

~~**One plugin** (`@specforge/formal-analysis`) handles all three, contributing 3 new entity types (contract, refinement, process) and 7 validation rules.~~ **[Superseded: formal methods integrated into `@specforge/software` as inline syntax (RES-27). No new entity types.]**

### 2. Formal features MUST be opt-in (progressive formality)

**Unanimous (10/10).** The TypeScript analogy: `any` works on Day 1, full types come later.

| Level | What It Looks Like | Token Reduction |
|-------|-------------------|-----------------|
| **0: Markdown** | Plain CLAUDE.md files | 0% (baseline) |
| **1: Entity graph** | Behaviors, features, types with prose | 30-50% |
| **2: Contracts** | `requires`/`ensures` fields | 60-75% |
| **3: Invariants** | Formal guarantees + enforcement | 75-86% |
| **4: Proofs** | SMT/TLA+ integration (future) | 90-95% |

### 3. Lead with token economics, not formalism

**Experts 3, 5, 8 (strongest advocates), supported by all.**

> **Never say "formal methods" in marketing. Say "structured specs that cut AI costs 75-86%."**

DbC failed mainstream adoption because of cultural resistance to formalism. SpecForge succeeds because **AI agents are the consumer** — they *prefer* precision.

### 4. `requires`/`ensures` on behaviors is the highest-ROI feature

**Experts 1, 2, 3, 5, 6, 7 (6/10 ranked this #1).** Preconditions alone provide 90% of the value — they map 1:1 to input validation, function signatures, and test assertions.

```spec
behavior create_user {
  contract "Create a user with unique email."

  requires {
    email_format     "email matches RFC 5322"
    no_existing_user "count(users where email = input.email) == 0"
  }

  ensures {
    user_created  "exists(users where id = result.userId)"
    unique_id     "count(users where id = result.userId) == 1"
  }

  verify unit "insert user, retrieve by ID, assert equal"
}
```

### 5. Deadlock detection in event graphs is the CSP "killer feature"

**Experts 1, 2, 4, 7, 9 (5/10).** CSP's greatest practical value isn't process algebra notation — it's **automated deadlock/livelock detection** over SpecForge's existing event/channel/producer/consumer model.

```
error[E017]: Potential deadlock in event chain
  payment_service → PaymentInitiated → order_service
                                        ↑               ↓
                                        └────────────────┘
  order_service → OrderConfirmed → payment_service

  Fix: Add timeout, use async messaging, or introduce coordinator
```

---

## The Concrete Improvement Plan

### New DSL Syntax (3 additions)

**1. Contract blocks on behaviors** (DbC)
```spec
behavior authenticate_user {
  requires {
    valid_email    "email matches RFC 5322"
    non_empty_pass "password.length >= 8"
  }
  ensures {
    session_created "result.session_token != null when authenticated"
    audit_logged    "audit_log_entry_created == true"
  }
  maintains {
    rate_limit "rate_limit_not_exceeded(username)"
  }
}
```

**2. Refinement annotations** (B-Method)
```spec
behavior authenticate_abstract {
  abstract true
  contract "Verify user identity and create session."
}

behavior authenticate_jwt {
  refines authenticate_abstract
  requires { valid_jwt "token is well-formed JWT" }
  ensures  { claims_valid "all required claims present" }
}
```

**3. Synchronization constraints on events** (CSP)
```spec
event order_placed {
  trigger place_order
  channel "orders.placed"
  consumers [charge_payment, begin_fulfillment]

  sync {
    barrier [charge_payment, begin_fulfillment]
    timeout 30s "Order processing must complete or rollback"
  }
}
```

### New Compiler Passes (4 additions)

```
parse → resolve → build_graph → validate
  ↓
contract_check       # DbC: precondition satisfiability, postcondition reachability
  ↓
refinement_verify    # B-Method: abstract→concrete chain completeness
  ↓
process_analyze      # CSP: deadlock/livelock detection via Tarjan SCC
  ↓
proof_obligation     # Generate verification conditions
  ↓
codegen
```

### New Validation Codes (~20 additions)

**Errors (E017-E023):**
| Code | Description | Source |
|------|-------------|--------|
| E017 | Deadlock: circular event dependency | CSP |
| E018 | Channel type mismatch (producer/consumer payload) | CSP |
| E019 | Unmatched producer (no consumer for event) | CSP |
| E020 | Contract guarantee cannot be verified | DbC |
| E021 | Precondition strengthening violation (Liskov) | DbC |
| E022 | Behavior does not satisfy feature requirements | B-Method |
| E023 | Incomplete refinement chain | B-Method |

**Warnings (W020-W029):**
| Code | Description | Source |
|------|-------------|--------|
| W020 | Potential livelock (infinite retry without backoff) | CSP |
| W021 | Starvation risk (unfair port access) | CSP |
| W022 | Unverifiable contract condition | DbC |
| W023 | Postcondition may be unreachable | DbC |
| W024 | Redundant precondition | DbC |
| W025 | Unverified proof obligation (no test) | B-Method |
| W026 | Deep refinement chain (>4 levels) | B-Method |
| W027 | Port contract stricter than behavior contract | DbC |
| W028 | Invariant has no formal property specification | DbC |
| W029 | Unbounded channel buffer | CSP |

**Info (I007-I009):**
| Code | Description | Source |
|------|-------------|--------|
| I007 | Proof obligation verified by test | B-Method |
| I008 | Deadlock freedom verified | CSP |
| I009 | Formal analysis available (`specforge analyze`) | All |

### New CLI Commands

```bash
specforge analyze contracts      # Check requires/ensures satisfiability
specforge analyze refinement     # Verify abstract→concrete chain
specforge analyze concurrency    # Deadlock/livelock detection
specforge analyze coverage       # Invariant enforcement mapping
specforge analyze interfaces     # Port↔behavior contract compatibility
specforge analyze all            # Run all analyses
specforge analyze all --strict   # Fail on any violation (for CI)
specforge analyze all --json     # Machine-readable output
```

### New Coverage Model

**Current:** "Has passing test" (binary)
```
Entity      | Intent   | Tests                  | Status
create_user | 2 verify | tests/create_user.test | PASS 2/2
```

**Proposed:** "Proof obligation discharge rate" (graduated)
```
Entity      | POs   | Discharge | Mutation | Verdict
create_user | 7/7   | 100%      | 6/7 86%  | STRONG

Proof Obligations:
  ✅ Precond: email is valid          → test: invalid_email_rejected
  ✅ Precond: email not in use        → test: duplicate_email_rejected
  ✅ Postcond: user created           → test: user_created
  ✅ Postcond: ID matches             → test: id_correct
  ✅ Invariant: unique_email          → test: email_uniqueness_property
```

**Verdict levels:** UNPROVED → PARTIAL → PROVED → STRONG → VERIFIED

### New Verify Kinds (6 additions)

| Kind | What It Tests | Entity |
|------|--------------|--------|
| `verify contract` | All requires + ensures hold | behavior |
| `verify refinement` | Concrete satisfies abstract | behavior |
| `verify trace` | Scenarios are valid CSP traces | capability, behavior |
| `verify deadlock_free` | No partial trace deadlocks | event |
| `verify liveness` | Eventual outcomes guaranteed | event |
| `verify mutation` | Test suite kills contract-derived mutants | behavior, invariant |

### Plugin Architecture

**One plugin, not three:**

```json
{
  "name": "@specforge/formal-analysis",
  "version": "0.1.0",
  "contributes": {
    "entities": ["contract", "refinement", "process"],
    "validators": true,
    "commands": ["analyze"]
  }
}
```

**New host function:** `specforge.invoke_query(name, args)` — runs graph algorithms (Tarjan SCC, reachability) at native Rust speed via Rhai scripts, not in Wasm.

---

## Implementation Roadmap

| Phase | What | Duration | Priority |
|-------|------|----------|----------|
| **1. DbC Contracts** | `requires`/`ensures` grammar + contract_check pass + E020-E021, W022-W024 | 3-4 weeks | CRITICAL |
| **2. Property Tests** | Auto-gen from invariants, `verify contract` kind | 2-3 weeks | HIGH |
| **3. Deadlock Detection** | process_analyze pass + Tarjan SCC + E017-E019, W020-W021 | 3-4 weeks | HIGH |
| **4. Refinement** | `refines`/`abstract` fields + refinement_verify pass + E022-E023, W025-W026 | 3-4 weeks | MEDIUM |
| **5. Plugin Packaging** | `@specforge/formal-analysis` Wasm plugin + query extensions | 2-3 weeks | MEDIUM |
| **6. LSP Integration** | Inline contract status, refinement tooltips, deadlock hints | 2-3 weeks | MEDIUM |

**Total: 16-21 weeks (4-5 months)**

**Recommendation:** Start Phase 1 (DbC contracts) immediately — highest ROI, lowest risk, 90% of the token-reduction value.

---

## Market Positioning (Expert 8 Consensus)

> **"The specification compiler for AI-native development. Reduce agent token waste by 75-86%. Validate what agents read before they write code."**

**Never say:** "Formal methods", "Design by Contract", "process algebra"
**Always say:** "Structured specs", "compiler-validated requirements", "AI context optimization"

**Target sequence:** AI-heavy startups (Y1) → Rust community (Y2) → Safety-critical (Y3+)
**Pricing:** Open-source formality, paid scale/collaboration/compliance

---

## Files Index

### Research Foundation (3 files)
- `research-design-by-contract.md` — DbC comprehensive research (~11.5K words)
- `research-b-method.md` — B-Method comprehensive research (~15K words)
- `research-csp.md` — CSP comprehensive research (~11.5K words)

### Expert Analyses (15+ files)
- `RES-22-formal-methods-improvements.md` — Expert 1+2: Theory + compiler design
- `safety-critical-analysis.md` — Expert 4: Safety-critical certification path
- `analysis-formal-methods-ai-agents.md` — Expert 5: AI agent token economics
- `spec/research/RES-20-type-system-evolution.md` — Expert 6: Type system (main, 1836 lines)
- `spec/research/RES-20-executive-summary.md` — Expert 6: Type system executive summary
- `spec/research/RES-20-syntax-reference.md` — Expert 6: DSL syntax reference
- `spec/research/RES-20-type-system-architecture.md` — Expert 6: Architecture diagrams
- `EXPERT-7-FORMAL-METHODS-TEST-MODEL.md` — Expert 7: Test model transformation
- `FORMAL-METHODS-RUST-EXAMPLES.md` — Expert 7: Rust code generation examples
- `FORMAL-METHODS-QUICK-REFERENCE.md` — Expert 7: Quick reference
- `expert-9-distributed-systems-formal-methods.md` — Expert 9: Distributed systems
- `formal-methods-plugin-design.md` — Expert 10: Plugin architecture
- `formal-methods-architecture.md` — Expert 10: Architecture diagrams
- `formal-methods-implementation-spec.md` — Expert 10: Implementation spec
- `formal-methods-README.md` — Expert 10: Navigation guide

### This Document
- `10-EXPERTS-FORMAL-METHODS-SYNTHESIS.md` — Consolidated synthesis (you are here)

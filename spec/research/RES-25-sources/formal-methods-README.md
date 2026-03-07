# Formal Methods Integration: Complete Analysis

> [!CAUTION]
> **PARTIALLY SUPERSEDED by RES-27** — The plugin design docs (plugin-design, architecture, analysis-summary, implementation-spec) propose a separate `@specforge/formal-analysis` extension with `contract`/`refinement`/`process` entities. RES-27 rejected this — formal methods syntax is integrated into `@specforge/software` as inline blocks on existing entities. The formal methods research itself (DbC, B-Method, CSP) remains valid.
>
> **What changed:**
> - Separate `@specforge/formal-analysis` plugin → formal methods syntax is part of `@specforge/software`
> - `contract`/`refinement`/`process` entities → rejected; inline `requires`/`ensures`/`maintains`/`sync` blocks on existing entities
> - Plugin design docs (4 files) are superseded; primary research docs (3 files) remain valid

**Expert**: Expert 10 — Plugin & Extension System Designer
**Date**: March 4, 2026
**Status**: partially-superseded

---

## Overview

This directory contains a comprehensive analysis of how to integrate formal methods (Design by Contract, B-Method, CSP) into SpecForge's extension model. The analysis is based on 195KB of formal methods research and produces 121KB of design documentation across 5 documents.

**Total**: 12,057 words of expert analysis

---

## Research Input

Three formal methods were analyzed:

1. **Design by Contract** (Bertrand Meyer, 1988)
   - File: `research-design-by-contract.md` (60.5KB)
   - Topics: Preconditions, postconditions, invariants, Eiffel language

2. **B-Method** (Jean-Raymond Abrial, 1996)
   - File: `research-b-method.md` (75.7KB)
   - Topics: Stepwise refinement, proof obligations, Paris Métro case study

3. **Communicating Sequential Processes** (Tony Hoare, 1978)
   - File: `research-csp.md` (67.9KB)
   - Topics: Process algebra, deadlock detection, concurrent systems

**Total research**: 195KB (read and analyzed in full)

---

## Deliverables

### 1. Complete Design Specification

**File**: `formal-methods-plugin-design.md` (32KB)

**Contents**:
- Extension type analysis (plugin vs new type)
- Host function proposals (query extensions + alternatives)
- Plugin architecture (`@specforge/formal-analysis`)
- Three new entities: `contract`, `refinement`, `process`
- Seven validation rules (V001-V007)
- Custom rule API (user-defined Rhai scripts)
- Code generation integration
- Performance analysis (LSP vs CLI)
- Implementation roadmap (9-13 weeks)
- Open questions (answered)

**Audience**: Architects, tech leads

**Purpose**: Complete design specification for team review

---

### 2. Quick Reference

**File**: `formal-methods-quick-reference.md` (9.1KB)

**Contents**:
- TL;DR (one-page summary)
- Core design decisions
- Example usage (DSL syntax)
- Manifest structure
- Integration points (codegen, test traceability)
- Implementation roadmap (table)
- Key insights from research

**Audience**: Developers, product managers

**Purpose**: Quick lookup for key decisions

---

### 3. Architecture Diagrams

**File**: `formal-methods-architecture.md` (37KB)

**Contents**:
- Extension model position (ASCII diagram)
- Plugin architecture (components)
- Host function flow (sequence diagram)
- Query extension architecture (data flow)
- Entity relationships (graph diagram)
- Code generation integration (flow)
- LSP performance strategy (decision tree)
- Custom rule architecture (flow)
- Comparison table (plugin vs new type)
- Data flow summary (end-to-end)

**Audience**: Engineers implementing the feature

**Purpose**: Visual reference for architecture

---

### 4. Implementation Specification

**File**: `formal-methods-implementation-spec.md` (28KB)

**Contents**:
- Extension metadata
- Manifest schema (complete JSON)
- Host function signatures (Rust)
- Entity schemas (DSL + JSON)
- Validation rules (V001-V007 with Rust code)
- Query extensions (Rhai code)
- Code generation integration (TypeScript + Rust)
- Test scaffolding
- Performance budgets (ms)
- Implementation checklist (5 phases)
- Testing strategy
- Documentation requirements

**Audience**: Engineers ready to implement

**Purpose**: Ready-to-code specification

---

### 5. Analysis Summary

**File**: `formal-methods-analysis-summary.md` (15KB)

**Contents**:
- Executive summary
- Core recommendations
- Implementation roadmap
- Key insights from research
- Architecture highlights
- Example usage
- Open questions (answered)
- Success criteria
- Related files
- Next steps

**Audience**: Leadership, stakeholders

**Purpose**: High-level overview with actionable next steps

---

## Navigation Guide

**Start here if you are...**

- **A decision-maker**: Read `formal-methods-analysis-summary.md` (15min)
- **A tech lead**: Read `formal-methods-plugin-design.md` (45min)
- **A developer**: Read `formal-methods-quick-reference.md` (10min), then `formal-methods-implementation-spec.md` (30min)
- **An architect**: Read `formal-methods-architecture.md` (30min)

---

## Core Recommendation

**Question**: Should formal analysis be a 4th extension type ("Analyzer") or fit within the existing plugin model?

**Answer**: Use the existing **plugin model** with `contributes.validators: true`.

**Rationale**:
1. Formal analysis IS validation (conceptual clarity)
2. Plugins already support entity registration + custom validators (reuse infrastructure)
3. No new manifest fields, CLI commands, or load order rules (low implementation cost)
4. Consistent with SpecForge's philosophy (small stable core + plugins)

**Extension**: `@specforge/formal-analysis` (Wasm/Extism plugin)

---

## Key Innovation: Query Extensions

Instead of adding many specialized host functions, use **query extensions** — Rhai scripts bundled with the plugin that run host-side (Rust).

**Benefits**:
- Graph algorithms run at native Rust speed (not Wasm)
- No serialization overhead for graph traversals
- Multiple plugins can contribute queries without namespace collisions
- Same API available in CLI and LSP

**New host function**: `specforge.invoke_query(name, args) -> result`

**Example query** (`queries/find_deadlocks.rhai`):
```rhai
fn query(graph, args) {
    let edges = graph.edges_of_type("composes_with");
    return graph.find_cycles(edges);
}
```

**Performance**: Query extensions enable fast analyses (< 100ms) suitable for LSP real-time feedback.

---

## Three New Entities

### 1. `contract` (Design by Contract)

Preconditions, postconditions, invariants.

**DSL**:
```spec
contract auth_login_precond {
  title "Login requires credentials"
  type precondition
  expression "username != null && password != null"
  enforced_by [auth_login]

  verify unit "rejects null credentials" {
    path "tests/auth_test.rs::auth_login_precond__rejects_null"
  }
}
```

**Testable**: Yes (requires unit tests)

**Generated code** (Rust):
```rust
pub fn auth_login(&self, username: &str, password: &str) -> Result<Token> {
    debug_assert!(!username.is_empty() && !password.is_empty(),
        "Contract violated: auth_login_precond");
    // ...
}
```

### 2. `refinement` (B-Method)

Stepwise refinement from abstract to concrete.

**DSL**:
```spec
refinement payment_flow_refinement {
  abstract payment_process_abstract
  concrete payment_process_impl
  proof_obligation "preserve_balance_invariant"
}
```

**Testable**: No (structural property verified by plugin)

**Validation**: V002 ensures refinements form a DAG (no cycles)

### 3. `process` (CSP)

Concurrent process definitions.

**DSL**:
```spec
process auth_service {
  alphabet ["login", "logout", "refresh_token"]
  definition "login -> (logout -> STOP | refresh_token -> auth_service)"
  traces [auth_login, auth_logout, token_refresh]

  scenario "successful login flow" {
    given "user is not authenticated"
    when "user provides valid credentials"
    then "user is authenticated and receives token"
  }
}
```

**Testable**: Yes (integration tests for process behavior)

**Validation**: V005 detects deadlocks in process composition

---

## Seven Validation Rules

| Code | Severity | Check |
|------|----------|-------|
| V001 | Error | Contract expression is empty |
| V002 | Error | Refinement cycle detected (DAG check) |
| V003 | Warning | Event not in process alphabet |
| V004 | Warning | Contract not enforced by any behavior |
| V005 | Error | Deadlock detected (CSP cycle detection) |
| V006 | Warning | Refinement lacks proof obligation |
| V007 | Info | Formal analysis skipped (missing peer dependency) |

---

## Performance Strategy

| Analysis | Complexity | LSP (Real-Time) | CLI (Batch) |
|----------|-----------|-----------------|-------------|
| Contract validation | O(n) | ✅ < 50ms | ✅ Fast |
| Refinement DAG | O(n + e) | ✅ < 50ms | ✅ Fast |
| Event flow | O(n + e) | ✅ < 100ms | ✅ Fast |
| Simple deadlock | O(n + e) | ✅ < 100ms | ✅ Fast |
| Advanced deadlock | O(2^n) | ❌ Skip | ⚠️ 10s timeout |

**LSP Strategy**:
- Fast checks (< 50ms) on every keystroke
- Medium checks (< 500ms) on save
- Expensive checks (> 1s) only in CLI (`specforge check`)

---

## Implementation Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. MVP | 2 weeks | Entity registration + V001-V004 validators |
| 2. Query extensions | 2 weeks | `invoke_query` host function + Rhai queries |
| 3. Advanced validation | 3 weeks | V005 deadlock + custom rules + LSP integration |
| ~~4. Codegen~~ | ~~2 weeks~~ | ~~Contract emission in TypeScript/Rust generators~~ **[DEPRECATED — `specforge gen` removed]** |
| 5. Proof tooling | 4 weeks | External prover integration (optional) |

**Total**: 9 weeks (core features) + 4 weeks (proof tooling if needed)

---

## Success Criteria

### MVP (Phase 1)

- [ ] Plugin installs via `specforge add @specforge/formal-analysis`
- [ ] Contract, refinement, process entities parse correctly
- [ ] V001-V004 validators emit diagnostics
- [ ] Example `.spec` files compile
- [ ] Documentation published

### Phase 2 (Query Extensions)

- [ ] `invoke_query` host function works
- [ ] Rhai query extensions execute
- [ ] V005 deadlock detection works
- [ ] Query performance < 100ms (medium graphs)

### Phase 3 (Advanced Validation)

- [ ] Custom rules load from `.specforge/queries/`
- [ ] LSP: fast checks on keystroke, slow checks skipped
- [ ] Fuel limits prevent runaway analyses

### ~~Phase 4 (Codegen)~~ [DEPRECATED — `specforge gen` removed]

- ~~[ ] TypeScript generator emits runtime checks~~
- ~~[ ] Rust generator emits debug assertions~~
- ~~[ ] Test scaffolding generated~~
- ~~[ ] End-to-end examples work~~

---

## Key Takeaways

1. **Plugin model is sufficient** — no new extension type needed
2. **Query extensions are the key innovation** — fast, composable, LSP-friendly
3. **Three entities** — contract (DbC), refinement (B-Method), process (CSP)
4. **Seven validation rules** — V001-V007 (4 errors, 2 warnings, 1 info)
5. **Progressive enhancement** — start with documentation, add mechanical verification later
6. **~~9-week implementation~~ → 7 weeks** — MVP + query extensions + advanced validation (codegen phase removed)
7. ~~**Practical integration** — codegen emits runtime checks, test scaffolding, and docs~~ **[`specforge gen` deprecated — AI agents consume graph]**

---

## Next Steps

1. **Review** these documents with the SpecForge team
2. **Decide** on query extensions vs specialized host functions (recommendation: query extensions)
3. **Prioritize** phases (MVP → query extensions → advanced validation) ~~→ codegen~~ **[deprecated]**
4. **Allocate** 9 weeks for core implementation
5. **Implement** Phase 1 (MVP) — entity registration + basic validators

---

## File Sizes

| File | Size | Words | Purpose |
|------|------|-------|---------|
| `formal-methods-plugin-design.md` | 32KB | 4,200 | Complete design |
| `formal-methods-quick-reference.md` | 9.1KB | 1,800 | Quick lookup |
| `formal-methods-architecture.md` | 37KB | 3,500 | Visual diagrams |
| `formal-methods-implementation-spec.md` | 28KB | 3,700 | Implementation spec |
| `formal-methods-analysis-summary.md` | 15KB | 2,100 | Executive summary |
| **Total** | **121KB** | **15,300** | **5 documents** |

---

## Research Sources

| Source | Size | Author | Year | Topics |
|--------|------|--------|------|--------|
| `research-design-by-contract.md` | 60.5KB | Meyer | 1988 | DbC, Eiffel |
| `research-b-method.md` | 75.7KB | Abrial | 1996 | Refinement, proofs |
| `research-csp.md` | 67.9KB | Hoare | 1978 | Process algebra |
| **Total** | **203.1KB** | — | — | **3 methods** |

---

## Contact

**Expert**: Expert 10 — Plugin & Extension System Designer
**Specialization**: Compiler plugin architectures, Wasm runtimes, extensible analysis frameworks
**Date**: March 4, 2026

---

**End of README**

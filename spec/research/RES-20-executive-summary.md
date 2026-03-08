# RES-20: Type System Evolution — Executive Summary

**Expert:** Type System Architect (Expert 6)
**Date:** March 4, 2026
**Status:** active

---

## The Problem

SpecForge v1 has a **structural type system** that catches basic syntax errors but misses critical issues:

1. **No value constraints** — can't express "amount must be positive" or "email must be valid"
2. **No behavioral contracts** — can't verify "if X holds, then Y must hold"
3. **No event compatibility** — can't check if consumers expect the same fields as producers
4. **No substitutability** — can't validate if one port/behavior can replace another
5. **Entity ID confusion** — compiler allows passing BehaviorId where TypeId is expected

These gaps lead to **late error detection** (runtime, test time) instead of **early detection** (compile time, editor time).

---

## The Solution

Add **five layers of type-level reasoning** on top of SpecForge's existing types:

### 1. Refinement Types (v2.2)

Express value constraints on fields:

```spec
type Money {
  amount   number { > 0 }                      // Positive values only
  currency string { in ["USD", "EUR", "GBP"] } // Enum set
}

type User {
  email string { matches "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$" }
  age   integer { >= 18, <= 120 }
}
```

**Impact:** Catches invalid values at compile time, not runtime.

### 2. Channel Typing (v2.3)

Ensure event producers and consumers agree on payload structure:

```spec
event user_created {
  payload {
    userId string
    email  string
  }
  consumers [send_notification]
}

behavior send_notification {
  consumes user_created { userId, email }  // Compiler validates fields exist
}
```

**Impact:** Catches event payload mismatches at compile time.

### 3. Contract Compatibility (v2.4)

Validate that behaviors satisfy Liskov Substitution Principle:

```spec
behavior base_operation {
  contract {
    requires input != ""
    ensures result != ""
  }
}

behavior enhanced_operation extends base_operation {
  contract {
    requires input != ""            // SAME precondition (OK)
    ensures result != ""
    ensures result.length > 10      // STRONGER postcondition (OK)
  }
}
```

**Impact:** Catches behavioral incompatibility at compile time.

### 4. Port Interface Subtyping (v2.5)

Apply variance rules to port method signatures:

```spec
port BaseRepository {
  method findById(id: string) -> Result<Entity, NotFoundError>
}

port UserRepository extends BaseRepository {
  method findById(id: string) -> Result<User, NotFoundError>  // Covariant return (OK)
}
```

**Impact:** Catches port interface violations at compile time.

### 5. Phantom Types (v2.1, Internal)

Use Rust's type system to prevent entity ID confusion:

```rust
// Before (unsafe):
let behavior_id: EntityId = EntityId::from_str("create_user");
let type_id: EntityId = EntityId::from_str("User");
graph.add_edge(type_id, behavior_id, EdgeType::Implements); // WRONG! (runtime panic)

// After (safe):
let behavior_id: BehaviorId = BehaviorId::new("create_user");
let type_id: TypeDefId = TypeDefId::new("User");
graph.add_edge(type_id, behavior_id, EdgeType::Implements); // COMPILE ERROR!
```

**Impact:** Catches entity kind mismatches at Rust compile time (before user sees error).

---

## Key Features

### All Features Are Opt-In

- **No breaking changes** — existing specs continue to work
- **Incremental adoption** — add features one at a time
- **Backward compatible** — old and new syntax coexist

### Three Formal Methods Traditions

1. **Design by Contract** (Eiffel, Dafny) → Typed contracts with pre/postconditions
2. **B-Method** (formal refinement) → Set-theoretic constraints, membership predicates
3. **CSP** (process algebra) → Channel typing for event payloads

### Performance Budget

| Phase | Target Time (10K entities) |
|-------|---------------------------|
| Refinement checking | < 20ms |
| Event payload typing | < 15ms |
| Contract compatibility | < 25ms |
| Port subtyping | < 10ms |
| Set constraints | < 20ms |
| **Total** | **< 100ms** (interactive UX) |

Optional SMT verification (v3.0): < 5s per entity, incremental, cached.

---

## Implementation Roadmap

| Version | Phase | Features | Timeline |
|---------|-------|----------|----------|
| **v2.0** | Foundation | Parse new syntax (no validation yet) | 3 months |
| **v2.1** | Phantom Types | EntityId\<K\>, ValidEdgeSource trait | +1 month |
| **v2.2** | Refinements | Validate field refinements + contracts | +2 months |
| **v2.3** | Event Typing | Payload compatibility checking | +1 month |
| **v2.4** | Contract Compat | Liskov Substitution for behaviors | +2 months |
| **v2.5** | Port Subtyping | Variance rules for port methods | +1 month |
| **v2.6** | Set Constraints | B-Method style predicates | +2 months |
| **v3.0** | SMT Verification | Z3 integration (optional --verify) | +6 months |

**Total time to v2.6:** ~12 months from v2.0
**Total time to v3.0:** ~18 months from v2.0

---

## Error Catalog

| Code | Name | Description | Example |
|------|------|-------------|---------|
| E041 | Event payload field missing | Consumer expects field not in payload | `consumes event { nonexistent }` |
| E042 | Refinement type violation | Value doesn't satisfy constraint | `amount == -100` when `amount { > 0 }` |
| E043 | Precondition strengthened | Subtype requires more than parent | Liskov violation |
| E044 | Postcondition weakened | Subtype promises less than parent | Liskov violation |
| E045 | Parameter not contravariant | Port param more specific than parent | Variance violation |
| E046 | Return not covariant | Port return more general than parent | Variance violation |
| E047 | Set membership violation | Value not in expected set | `role ∈ {admin, editor}` |
| E048 | Subset violation | Collection not subset of expected type | `result ⊆ {...}` |
| E049 | SMT verification failed | Solver can't prove contract | (v3.0 only) |

---

## Benefits

### For Developers

- **Earlier error detection** — catch bugs at compile time, not test/runtime
- **Better IDE support** — LSP can show refinement violations in editor
- **Self-documenting** — refinements are machine-readable specs
- **Refactoring confidence** — type checker validates changes

### For AI Agents

- **Structured contracts** — LLMs can reason about `requires`/`ensures`, not just prose
- **Explicit constraints** — refinements are precise inputs for AI agents
- **Traceability** — typed edges connect specs to implementations
- **Test oracles** — refinements become property test assertions

### For Teams

- **Design clarity** — type refinements force precision in specs
- **Contract enforcement** — compiler validates behavioral contracts
- **Safe evolution** — structural typing enables backward-compatible changes
- **Cross-service verification** — event typing catches integration issues early

---

## Comparison to Related Systems

| System | Refinements | Contracts | Channel Typing | Subtyping | Target |
|--------|-------------|-----------|----------------|-----------|--------|
| **Liquid Haskell** | ✅ Full SMT | ❌ | ❌ | ✅ | Haskell |
| **F* / Dafny** | ✅ Full SMT | ✅ Full DbC | ❌ | ✅ | Verification |
| **Eiffel** | ❌ | ✅ Runtime | ❌ | ✅ | OOP |
| **TLA+ / B-Method** | ✅ Set theory | ✅ Pre/post | ❌ | ❌ | Formal specs |
| **CSP / π-calculus** | ❌ | ❌ | ✅ Full | ❌ | Concurrency |
| **TypeScript** | ❌ | ❌ | ❌ | ✅ Structural | Web |
| **Rust** | Traits only | ❌ | ❌ | ✅ Nominal | Systems |
| **SpecForge v2** | ✅ Lightweight | ✅ Typed | ✅ Events | ✅ Full | **AI + Specs** |

**SpecForge's Unique Position:** First specification language combining refinement types, typed contracts, and event payload typing **optimized for AI agent consumption**.

---

## Next Steps

1. **Review** — Gather stakeholder feedback on RES-20 proposal
2. **ADR** — Create `ADR-TypeSystemEvolution` documenting final decisions
3. **Prototype** — Implement Phase 1 (syntax extensions) in feature branch
4. **Validate** — Test DSL ergonomics with real-world specs
5. **Iterate** — Refine syntax based on user feedback
6. **Ship** — Release incrementally (v2.0 → v2.6 → v3.0)

---

## Open Questions

1. **SMT Solver Choice:** Z3 vs. CVC5 vs. heuristic-only?
2. **Contract Syntax:** `requires`/`ensures` (decided)
3. **Refinement Complexity:** How much predicate logic to support?
4. **Performance:** Can type checking stay under 100ms for 10K entities?
5. **Error Messages:** How to explain type errors to non-experts?

---

## Resources

**Full Documentation:**

- **RES-20-type-system-evolution.md** (60 KB) — Complete research analysis
- **RES-20-type-system-architecture.md** (15 KB) — Architecture diagrams
- **RES-20-syntax-reference.md** (20 KB) — DSL syntax reference
- **RES-20-executive-summary.md** (this document) — Quick overview

**Related Research:**

- **research-design-by-contract.md** (60 KB) — DbC comprehensive study
- **research-b-method.md** (76 KB) — B-Method formal analysis
- **research-csp.md** (68 KB) — CSP process algebra study

---

## Conclusion

SpecForge's proposed type system evolution adds **strong static guarantees** while maintaining **backward compatibility** and **AI-agent-first design**. The five-layer architecture (refinements, event typing, contract compatibility, port subtyping, phantom types) catches errors early, reduces bugs, and improves specifications' precision.

All features are **opt-in** and **incremental**. Projects can adopt features gradually, from v2.0 (basic refinements) to v3.0 (full SMT verification). The system is **unique in its focus on AI agent consumption** — no other specification language combines refinement types with event-driven architecture typing.

**Recommendation:** Proceed with phased implementation starting with v2.0 foundation.

---

**Status:** Awaiting stakeholder review and ADR creation
**Author:** Expert 6 (Type System Architect)
**Date:** March 4, 2026

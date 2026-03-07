# RES-20: Type System Architecture Diagram

## Type System Layers

```
┌────────────────────────────────────────────────────────────────────┐
│                    SPECFORGE TYPE SYSTEM (v2.0+)                   │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 5: Formal Verification (OPTIONAL, v3.0)               │ │
│  │                                                              │ │
│  │  • SMT solver integration (Z3)                             │ │
│  │  • Contract verification (pre/post proof)                  │ │
│  │  • Refinement proof obligations                            │ │
│  │  • Enabled with --verify flag                              │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                              ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 4: Subtyping & Compatibility (v2.4-2.5)              │ │
│  │                                                              │ │
│  │  • Contract subtyping (Liskov Substitution)                │ │
│  │    - Precondition weakening                                 │ │
│  │    - Postcondition strengthening                            │ │
│  │  • Port interface subtyping                                 │ │
│  │    - Parameter contravariance                               │ │
│  │    - Return covariance                                      │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                              ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 3: Channel Typing (v2.3)                              │ │
│  │                                                              │ │
│  │  • Event payload compatibility                              │ │
│  │    - Producer payload type                                  │ │
│  │    - Consumer payload projection                            │ │
│  │    - Structural subtyping rules                             │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                              ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 2: Refinement Types (v2.2)                            │ │
│  │                                                              │ │
│  │  • Value constraints on fields                              │ │
│  │    - Numeric bounds: { > 0, <= 100 }                       │ │
│  │    - String patterns: { matches "regex" }                   │ │
│  │    - Set membership: { in ["a", "b", "c"] }                │ │
│  │    - Derived constraints: { == expr }                       │ │
│  │  • Set-theoretic constraints (B-Method)                     │ │
│  │    - Subset relations: result ⊆ {...}                      │ │
│  │    - Cardinality bounds: |result| >= 0                      │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                              ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 1: Structural Types (v1.0, BASELINE)                  │ │
│  │                                                              │ │
│  │  • Primitive types: string, number, boolean, timestamp      │ │
│  │  • Named types: type User { ... }                           │ │
│  │  • Field annotations: @readonly, @unique, @optional         │ │
│  │  • Port method signatures: Result<T, E>                     │ │
│  │  • Entity references: typed graph edges                     │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                              ↓                                     │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Layer 0: Phantom Types (INTERNAL, v2.1)                     │ │
│  │                                                              │ │
│  │  • EntityId<K: EntityKindMarker> (Rust compile-time safety)│ │
│  │  • BehaviorId, TypeDefId, PortId, EventId, ...             │ │
│  │  • ValidEdgeSource<T, E> trait (graph edge validation)      │ │
│  │  • Zero runtime cost, pure compile-time checks              │ │
│  └──────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘
```

> **Note (RES-26):** Layer 0 phantom types assume static `EntityKind` variants. Under zero-entity core, entity kinds are dynamically registered from extensions via `InternedId`, not compile-time phantom types.

## Type Checking Pipeline

```
┌──────────┐
│  Parser  │  Produces: AST with type annotations
└────┬─────┘
     │
     v
┌──────────┐
│ Resolver │  Produces: Typed graph (entities + edges with typed IDs)
└────┬─────┘
     │
     v
┌───────────────────────────────────────────────────────────────┐
│                    TYPE CHECKER (NEW)                          │
│                                                                │
│  Phase 1: Build TypeContext from graph                        │
│           - Extract all types, ports, behaviors, events       │
│           - Build refinement/contract index                    │
│                                                                │
│  Phase 2: Check refinement types                              │
│           - Validate refinement syntax                         │
│           - Check field usage against refinements              │
│           - Integrate with contract clauses                    │
│                                                                │
│  Phase 3: Check event payloads                                │
│           - Match producer payload vs consumer expectations    │
│           - Validate structural compatibility                  │
│           - Emit E030 on field mismatches                      │
│                                                                │
│  Phase 4: Check contract compatibility                        │
│           - Validate behavior substitutability                 │
│           - Check precondition weakening                       │
│           - Check postcondition strengthening                  │
│           - Emit E032/E033 on Liskov violations                │
│                                                                │
│  Phase 5: Check port subtyping                                │
│           - Validate parameter contravariance                  │
│           - Validate return covariance                         │
│           - Emit E034/E035 on variance violations              │
│                                                                │
│  Phase 6: Check set constraints                               │
│           - Validate set membership predicates                 │
│           - Check subset relations                             │
│           - Emit E036/E037 on set violations                   │
│                                                                │
│  Phase 7 (Optional): SMT verification                         │
│           - Translate contracts to SMT-LIB                     │
│           - Query Z3 solver                                    │
│           - Emit E038 on proof failures                        │
└───────────────────────────────────────────────────────────────┘
     │
     v
┌──────────┐
│Validator │  Validates: reference integrity, testability, etc.
└────┬─────┘
     │
     v
┌──────────┐
│ Emitter  │  Produces: Diagnostics, graph, reports
└──────────┘
```

## Type System Feature Matrix

| Feature | Syntax Example | Check Phase | Error Code | Version |
|---------|---------------|-------------|------------|---------|
| **Refinement Types** | `amount: number { > 0 }` | Phase 2 | E031, W020 | v2.2 |
| **Event Payload** | `consumes event { field1, field2 }` | Phase 3 | E030 | v2.3 |
| **Contract Subtyping** | `requires`/`ensures` clauses | Phase 4 | E032, E033 | v2.4 |
| **Port Subtyping** | `port X extends Y` | Phase 5 | E034, E035 | v2.5 |
| **Set Constraints** | `where result ⊆ {...}` | Phase 6 | E036, E037 | v2.6 |
| **SMT Verification** | `--verify` flag | Phase 7 | E038, W022 | v3.0 |
| **Phantom Types** | `EntityId<K>` (internal) | Rust compiler | (compile error) | v2.1 |

## Example: Full Type System in Action

```spec
// ============================================================================
// LAYER 1: Structural types (baseline)
// ============================================================================
type Money {
  amount   number          // Structural type
  currency string          // Structural type
}

// ============================================================================
// LAYER 2: Refinement types (value constraints)
// ============================================================================
type Money {
  amount   number { > 0 }                      // Refinement: positive
  currency string { in ["USD", "EUR", "GBP"] } // Refinement: enum set
}

// ============================================================================
// LAYER 3: Channel typing (event payload compatibility)
// ============================================================================
event payment_processed {
  trigger charge_payment
  payload {
    orderId       string { len > 0 }  // Refinement + structural
    amount        number { > 0 }       // Refinement
    currency      string
  }
  consumers [update_order_status, send_receipt]
}

behavior send_receipt {
  // Layer 3: Compiler checks that these fields exist in payload
  consumes payment_processed { orderId, amount, currency }

  contract {
    // Layer 2: Compiler validates refinements are satisfied
    requires event.amount > 0       // Redundant (refinement already ensures)
    requires event.orderId != ""

    ensures receipt_sent(event.orderId)
  }
}

// ============================================================================
// LAYER 4: Contract subtyping (behavioral substitutability)
// ============================================================================
behavior base_charge_payment {
  contract {
    requires customer.paymentMethod != ""
    ensures payment.status == "completed"
  }
}

behavior retry_charge_payment extends base_charge_payment {
  contract {
    // Layer 4: Compiler checks Liskov Substitution
    requires customer.paymentMethod != ""  // SAME precondition (OK)
    ensures payment.status == "completed"  // SAME postcondition
    ensures payment.attempts <= 3          // STRONGER postcondition (OK)
  }
}

// ============================================================================
// LAYER 4: Port interface subtyping
// ============================================================================
port BaseRepository {
  direction outbound
  method findById(id: string) -> Result<Entity, NotFoundError>
}

port PaymentRepository extends BaseRepository {
  direction outbound
  // Layer 4: Compiler validates covariant return (Payment <: Entity)
  method findById(id: string) -> Result<Payment, NotFoundError>
}

// ============================================================================
// LAYER 2: Set constraints (B-Method style)
// ============================================================================
port PaymentRepository {
  method findByStatus(status: PaymentStatus) -> Result<Payment[], never>
    // Layer 2: Set constraint - result is subset of matching payments
    where result ⊆ {p ∈ Payment | p.status == status}
}

// ============================================================================
// LAYER 0: Phantom types (internal Rust safety, no DSL syntax)
// ============================================================================
// In Rust implementation:
// let behavior_id: BehaviorId = BehaviorId::new("charge_payment");
// let type_id: TypeDefId = TypeDefId::new("Money");
// graph.add_edge(behavior_id, type_id, EdgeType::UsesType);  // ✅ OK
// graph.add_edge(type_id, behavior_id, EdgeType::Implements); // ❌ Compile error!

// ============================================================================
// LAYER 5: Optional SMT verification (v3.0)
// ============================================================================
// With --verify flag enabled, compiler invokes Z3 to prove:
//   ∀ event: PaymentProcessedPayload.
//     event.amount > 0  // Proven by refinement type
//   ∀ behavior extends base.
//     behavior.precondition ⟹ base.precondition  // Proven by SMT
```

## Error Detection Comparison

| Error Type | Without Type System | With Type System |
|------------|---------------------|------------------|
| **Negative money amount** | Runtime error / Test failure | E031 at compile time (refinement violation) |
| **Event field typo** | Runtime error / Undefined behavior | E030 at compile time (payload field missing) |
| **Behavior violates Liskov** | Silent correctness bug | E032 at compile time (precondition strengthened) |
| **Port incompatible substitution** | Runtime type error | E034/E035 at compile time (variance violation) |
| **Entity ID wrong kind** | Runtime panic or wrong results | Rust compile error (phantom type mismatch) |
| **Contract contradiction** | Logic bug, hard to debug | E038 (optional, with --verify flag) |

## Adoption Strategy

```
v1.0 (Current)              v2.2                v2.6                v3.0
    │                        │                   │                   │
    │ Structural types       │ + Refinements     │ + All layers      │ + SMT verification
    │ Entity references      │ + Phantom IDs     │ + Event typing    │ + Full proofs
    │ Port signatures        │                   │ + Contract compat │
    │                        │                   │ + Port subtyping  │
    │                        │                   │ + Set constraints │
    │                        │                   │                   │
    ▼                        ▼                   ▼                   ▼
┌─────────┐              ┌─────────┐         ┌─────────┐         ┌─────────┐
│No type  │              │Basic    │         │Advanced │         │Formal   │
│checking │───────────▶  │type     │───────▶ │type     │───────▶ │verification│
│beyond   │  3 months    │checking │ 6 months│checking │ 6 months│(optional)│
│syntax   │              │         │         │         │         │          │
└─────────┘              └─────────┘         └─────────┘         └─────────┘
```

## Related Systems Comparison

```
┌────────────────┬─────────┬──────────┬─────────┬─────────┬─────────┐
│ System         │Refine   │Contracts │Channel  │Subtyping│Target   │
├────────────────┼─────────┼──────────┼─────────┼─────────┼─────────┤
│ Liquid Haskell │ ✅ Full │ ❌       │ ❌      │ ✅      │Haskell  │
│ F* / Dafny     │ ✅ Full │ ✅ Full  │ ❌      │ ✅ Full │Verified │
│ Eiffel         │ ❌      │ ✅ Runtime│ ❌      │ ✅ OO   │OOP      │
│ TLA+ / B       │ ✅ Sets │ ✅ Pre/Post│ ❌     │ ❌      │Formal   │
│ CSP / π-calc   │ ❌      │ ❌       │ ✅ Full │ ❌      │Concurrency│
│ TypeScript     │ ❌      │ ❌       │ ❌      │ ✅ Struct│Web      │
│ Rust           │ Traits  │ ❌       │ ❌      │ ✅ Nominal│Systems │
│ SpecForge v2   │ ✅ Light│ ✅ Typed │ ✅ Events│ ✅ Full │AI+Specs │
└────────────────┴─────────┴──────────┴─────────┴─────────┴─────────┘
```

## Performance Budget

| Phase | Target Time (10K entities) | Strategy |
|-------|---------------------------|----------|
| Phase 1: TypeContext build | < 10ms | Single-pass graph walk |
| Phase 2: Refinement checking | < 20ms | Syntax validation only (no SMT) |
| Phase 3: Event payload | < 15ms | Structural comparison (hash-based) |
| Phase 4: Contract compat | < 25ms | Heuristic implication (no SMT) |
| Phase 5: Port subtyping | < 10ms | Nominal check with cache |
| Phase 6: Set constraints | < 20ms | Syntax validation only |
| **Total (without SMT)** | **< 100ms** | **Interactive UX** |
| Phase 7: SMT verification (opt-in) | < 5s per entity | Incremental, cached results |

## Open Questions for Stakeholders

1. **Syntax Bikeshedding**:
   - `requires`/`ensures` vs. `pre`/`post` vs. `given`/`then`?
   - `{ > 0 }` vs. `where amount > 0` for refinements?
   - `consumes event { fields }` vs. `consumes event(fields)` vs. other?

2. **SMT Integration**:
   - Always Z3? Or pluggable solver (CVC5, etc.)?
   - Default timeout: 5s? 10s? User-configurable?
   - Cache SMT results across runs? (incremental verification)

3. **Error Verbosity**:
   - Show SMT solver output on failure? Or abstract it?
   - Suggest fixes for type errors? (LSP quick-fix integration)

4. **Backward Compatibility**:
   - Is opt-in sufficient? Or need feature flags per-file?
   - Should v1 specs emit "consider upgrading" hints?

5. **Performance**:
   - Is 100ms type-check budget realistic for 10K entities?
   - Should type checking be skippable in CI? (lint-only mode)

---

**See also**: `RES-20-type-system-evolution.md` (full research document)

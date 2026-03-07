# Formal Methods Integration: Quick Reference

> [!CAUTION]
> **SUPERSEDED by RES-27** — `@specforge/formal-analysis` as separate plugin rejected. Formal methods syntax is part of `@specforge/software` as inline blocks on existing entities. No new entity types.

**Expert**: Expert 10 — Plugin & Extension System Designer
**Date**: March 4, 2026
**Full Design**: See `formal-methods-plugin-design.md`

---

## TL;DR

~~**Recommendation**: Formal analysis (DbC, B-Method, CSP) should be implemented as a **standard plugin** (`@specforge/formal-analysis`), NOT a new extension type.~~ **[Superseded: formal methods syntax integrated into `@specforge/software` (RES-27)]**

**Key insight**: Formal verification is validation. The syntax (`requires`/`ensures`/`maintains`/`sync`) attaches inline to existing `@specforge/software` entities.

---

## Core Design Decisions

### 1. Extension Type: Plugin (Not a 4th Extension Type)

✅ **USE**: Existing plugin model with `contributes.validators: true`
❌ **AVOID**: New "Analyzer" extension type

**Rationale**:
- Formal analysis = custom validation
- Plugins already support entity registration + validation
- Reuses existing infrastructure (manifests, CLI, host functions)
- Consistent with SpecForge's philosophy (small core + plugins)

### 2. New Entities (3)

| Entity | Testable | Purpose |
|--------|----------|---------|
| `contract` | ✅ Yes | DbC: preconditions, postconditions, invariants |
| `refinement` | ❌ No | B-Method: abstract → concrete implementation proofs |
| `process` | ✅ Yes | CSP: concurrent process definitions |

### 3. Host Functions Strategy

**Phase 1 (MVP)**: Use existing `query_graph` (full graph JSON)
**Phase 2 (optimization)**: Add `invoke_query` for Rhai query extensions
**Phase 3 (if needed)**: Add specialized functions (`query_subgraph`, `find_cycles`, `filter_nodes`)

**Key innovation**: Query extensions (Rhai scripts) run host-side (Rust) for performance.

### 4. Validation Rules (7)

| Code | Severity | Check |
|------|----------|-------|
| V001 | Error | Contract expression is empty |
| V002 | Error | Refinement cycle detected |
| V003 | Warning | Event not in process alphabet |
| V004 | Warning | Contract not enforced |
| V005 | Error | Deadlock detected (CSP) |
| V006 | Warning | Refinement lacks proof obligation |
| V007 | Info | Formal analysis skipped (missing plugin) |

### 5. Performance Model

| Analysis | Complexity | LSP | CLI |
|----------|-----------|-----|-----|
| Contract validation | O(n) | ✅ Real-time | ✅ Batch |
| Refinement DAG check | O(n + e) | ✅ Real-time | ✅ Batch |
| Event flow tracing | O(n + e) | ✅ Real-time | ✅ Batch |
| Simple deadlock | O(n + e) | ✅ Real-time | ✅ Batch |
| Advanced deadlock | O(2^n) | ❌ Too slow | ⚠️ Timeout |

**Strategy**: Fast checks on every keystroke, medium checks on save, slow checks only in CLI.

---

## Example Usage

### ~~Install Plugin~~ (SUPERSEDED)

```bash
# SUPERSEDED — formal methods are part of @specforge/software, no separate install needed
# specforge add @specforge/formal-analysis
specforge add @specforge/software  # includes formal methods syntax
```

### Declare Entities

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

refinement payment_flow_refinement {
  abstract payment_process_abstract
  concrete payment_process_impl
  proof_obligation "preserve_balance_invariant"
}

process auth_service {
  alphabet ["login", "logout", "refresh_token"]
  definition "login -> (logout -> STOP | refresh_token -> auth_service)"
  traces [auth_login, auth_logout, token_refresh]
}
```

### Custom Validation Rules

Users can add domain-specific rules via Rhai scripts:

```rhai
// .specforge/queries/custom_rule.rhai

fn validate(graph) {
    let payment_behaviors = graph.filter_nodes(#{
        kind: "behavior",
        name_contains: "payment"
    });

    for behavior in payment_behaviors {
        let contracts = graph.outgoing_edges(behavior.id, "guards");
        let has_balance_invariant = contracts.any(|c|
            c.field("type") == "invariant" &&
            c.field("expression").contains("balance")
        );

        if !has_balance_invariant {
            emit_diagnostic("warning", "CUSTOM001",
                `Payment behavior '${behavior.id}' missing balance invariant`,
                behavior.span
            );
        }
    }
}
```

---

## ~~Manifest Structure~~ (SUPERSEDED)

> [!CAUTION]
> **Superseded by RES-27.** There is no separate `@specforge/formal-analysis` extension. Formal methods syntax is part of `@specforge/software`.

```json
// SUPERSEDED — no separate extension exists
{
  "extension": "@specforge/formal-analysis",
  "manifest_version": "2",
  "version": "0.1.0",
  "wasm": "formal_analysis.wasm",

  "contributes": {
    "entities": true,
    "validators": true
  },

  "entity_kinds": [
    {
      "name": "contract",
      "testable": true,
      "fields": [
        {"name": "type", "type": "enum", "values": ["precondition", "postcondition", "invariant"]},
        {"name": "expression", "type": "string", "required": true},
        {"name": "enforced_by", "type": "reference_list"}
      ]
    }
  ],

  "query_extensions": [
    {
      "name": "event_flow",
      "file": "queries/event_flow.rhai"
    },
    {
      "name": "find_deadlocks",
      "file": "queries/find_deadlocks.rhai"
    }
  ],

  "sandbox": {
    "max_fuel": 50000000
  }
}
```

---

## Integration Points

### 1. Code Generation

Generators emit runtime checks from contracts:

**TypeScript**:
```typescript
export function auth_login(username: string, password: string) {
  // @specforge-contract: auth_login_precond
  if (username == null || password == null) {
    throw new ContractViolation("username != null && password != null");
  }
  // ... implementation
}
```

**Rust**:
```rust
impl AuthService {
    /// # Precondition
    /// `username != null && password != null`
    pub fn auth_login(&self, username: &str, password: &str) -> Result<Token> {
        debug_assert!(!username.is_empty() && !password.is_empty(),
            "Contract violated: auth_login_precond");
        // ... implementation
    }
}
```

### 2. Test Traceability

`contract` and `process` are testable entities — they link to tests:

```spec
contract auth_login_precond {
  verify unit "rejects null credentials" {
    path "tests/auth_test.rs::auth_login_precond__rejects_null"
  }
}
```

Test coverage chain: `capability → behavior → contract → tests`

### 3. Cross-Plugin References

Formal entities reference core and product entities:

```spec
capability payment_processing {
  contracts [payment_invariant]  // product → formal
  behaviors [process_payment]    // product → core
}

contract payment_invariant {
  enforced_by [process_payment]  // formal → core
}
```

---

## Implementation Roadmap

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. MVP | 2 weeks | Entity registration + basic validators |
| 2. Query extensions | 2 weeks | Rhai query support + performance optimization |
| 3. Advanced validation | 3 weeks | Deadlock detection + custom rules + LSP integration |
| 4. Codegen | 2 weeks | Contract emission in TypeScript/Rust generators |
| 5. Proof tooling | 4 weeks | External prover integration (optional) |

**Total**: 9 weeks (core features) + 4 weeks (proof tooling if needed)

---

## Key Insights from Research

### Design by Contract (Meyer, 1988)

- **Contracts as first-class entities** — preconditions, postconditions, invariants
- **Inheritance**: Subclass contracts must be compatible (Liskov substitution)
- **Runtime vs static**: SpecForge uses contracts for documentation + validation hints (not mechanically verified)

### B-Method (Abrial, 1996)

- **Stepwise refinement** — abstract machines refined into concrete implementations
- **Proof obligations** — each refinement step generates obligations (not mechanically checked in SpecForge)
- **Industrial success** — Paris Métro Line 14 (driverless train) verified with B-Method

### CSP (Hoare, 1978)

- **Process algebra** — concurrent systems as communicating processes
- **Deadlock detection** — CSP allows reasoning about liveness properties
- **Trace semantics** — processes defined by their observable behavior

---

## Why This Design Works

1. **Composable**: Users opt in by installing the plugin (no mandatory formal verification).
2. **Performant**: Query extensions run in Rust, not Wasm (low overhead).
3. **Consistent**: Reuses existing plugin infrastructure (no new concepts).
4. **Extensible**: Custom rules via Rhai scripts (domain-specific analyses).
5. **Practical**: Integrates with code generation and test traceability.
6. **Progressive**: Start with documentation, add mechanical verification later.

---

## Open Questions

1. **Should refinement be testable?** (Start with `false`, add if users request equivalence tests)
2. **Should we parse contract expressions?** (Start with strings, add DSL in v0.6.0 if needed)
3. **Multiple formal plugins?** (Support via load-order priority, emit `W023` on collision)

---

## Related Documents

- Full design: `/Users/u1070457/Projects/Perso/specforge/formal-methods-plugin-design.md`
- Research:
  - `/Users/u1070457/Projects/Perso/specforge/research-design-by-contract.md`
  - `/Users/u1070457/Projects/Perso/specforge/research-b-method.md`
  - `/Users/u1070457/Projects/Perso/specforge/research-csp.md`
- Extension model: `/Users/u1070457/Projects/Perso/specforge/docs/extension-model.md`

---

**End of Quick Reference**

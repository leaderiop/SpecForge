# Formal Methods Plugin: Architecture Diagrams

> [!CAUTION]
> **SUPERSEDED by RES-27** — Architecture diagrams showing "CORE (8 entities)" and codegen pipeline are outdated. Zero-entity core (RES-26) eliminates hardcoded entities. Formal methods integrated into `@specforge/software`, not a separate plugin.
>
> **What changed:**
> - "CORE (8 entities)" → zero-entity core (RES-26); all entities come from extensions
> - `@specforge/formal-analysis` as separate plugin box → formal methods are part of `@specforge/software`
> - GENERATORS column (`@specforge/gen-ts`, `gen-rust`, `gen-py`) → `specforge gen` deprecated; AI agents consume graph directly
> - Plugin/Provider/Generator three-column model → replaced by contribution-based extension model (RES-23)

**Date**: March 4, 2026
**Status**: superseded
**Expert**: Expert 10 — Plugin & Extension System Designer

---

## 1. Extension Model Position

**[SUPERSEDED: zero-entity core (RES-26) — core has 0 built-in entities, all from extensions]**

```
┌─────────────────────────────────────────────────────────────────┐
│                    SPECFORGE CORE (8 entities) [SUPERSEDED]      │
│  spec · invariant · behavior · feature · event · type · port · ref│
├──────────────────────┬──────────────────┬───────────────────────┤
│   PLUGINS (entities) │ PROVIDERS (refs) │ GENERATORS (output)   │
│                      │                  │                       │
│  @specforge/product  │  @specforge/gh   │  @specforge/gen-ts    │
│  @specforge/         │  @specforge/jira │  @specforge/gen-rust  │
│    governance        │  @specforge/figma│  @specforge/gen-py    │
│                      │                  │                       │
│  @specforge/         │                  │                       │
│    formal-analysis ◄─┼──────────────────┼───────────────────────┤
│    (NEW)             │                  │  Queries contracts    │
│                      │                  │  Emits runtime checks │
└──────────────────────┴──────────────────┴───────────────────────┘
```

**Position**: `@specforge/formal-analysis` is a **plugin** (not a new extension type).

---

## 2. Plugin Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│               @specforge/formal-analysis (Wasm Plugin)           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐   │
│  │ Entity         │  │ Validators     │  │ Query           │   │
│  │ Registration   │  │                │  │ Extensions      │   │
│  ├────────────────┤  ├────────────────┤  ├─────────────────┤   │
│  │ - contract     │  │ V001: Empty    │  │ event_flow.rhai │   │
│  │ - refinement   │  │       expr     │  │ check_ref.rhai  │   │
│  │ - process      │  │ V002: Cycle    │  │ deadlock.rhai   │   │
│  │                │  │ V003: Alphabet │  │ contracts.rhai  │   │
│  │ Edge types:    │  │ V004: Unenf    │  │                 │   │
│  │ - guards       │  │ V005: Deadlock │  │ (runs in Rust   │   │
│  │ - refines      │  │ V006: No proof │  │  host, not Wasm)│   │
│  │ - composes     │  │ V007: Skipped  │  │                 │   │
│  └────────────────┘  └────────────────┘  └─────────────────┘   │
│                                                                  │
│  Host Functions Used:                                            │
│  - specforge.query_graph()       → Get full graph JSON          │
│  - specforge.emit_diagnostic()   → Report validation errors     │
│  - specforge.register_entity()   → Add contract/refinement/proc │
│  - specforge.register_edge()     → Add guards/refines/composes  │
│  - specforge.invoke_query()      → Run Rhai query extensions    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Host Function Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        SPECFORGE CLI/LSP                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Parse .spec files                                            │
│  2. Build graph (entities + edges)                               │
│  3. Load plugins (Wasm modules)                                  │
│  4. Initialize plugins:                                          │
│     ┌────────────────────────────────────────────────┐           │
│     │  Plugin: @specforge/formal-analysis            │           │
│     │  ┌──────────────────────────────────────────┐  │           │
│     │  │  initialize()                            │  │           │
│     │  │  ├─ register_entity("contract")          │  │           │
│     │  │  ├─ register_entity("refinement")        │  │           │
│     │  │  ├─ register_entity("process")           │  │           │
│     │  │  ├─ register_edge("guards")              │  │           │
│     │  │  ├─ register_edge("refines")             │  │           │
│     │  │  └─ register_edge("composes")            │  │           │
│     │  └──────────────────────────────────────────┘  │           │
│     └────────────────────────────────────────────────┘           │
│  5. Re-parse with new entity kinds                               │
│  6. Re-link graph with new edges                                 │
│  7. Run validators:                                              │
│     ┌────────────────────────────────────────────────┐           │
│     │  Plugin: @specforge/formal-analysis            │           │
│     │  ┌──────────────────────────────────────────┐  │           │
│     │  │  validate()                              │  │           │
│     │  │  ├─ query_graph() → JSON                 │  │           │
│     │  │  ├─ foreach contract:                    │  │           │
│     │  │  │    if expr.empty():                   │  │           │
│     │  │  │      emit_diagnostic("V001")          │  │           │
│     │  │  ├─ invoke_query("find_deadlocks")       │  │           │
│     │  │  │    ↓ (Rhai executes in Rust host)     │  │           │
│     │  │  │    ↑ Returns cycle list               │  │           │
│     │  │  └─ if cycles: emit_diagnostic("V005")   │  │           │
│     │  └──────────────────────────────────────────┘  │           │
│     └────────────────────────────────────────────────┘           │
│  8. Collect diagnostics                                          │
│  9. Display errors/warnings                                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Query Extension Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Wasm Plugin (Rust)                             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  validate() {                                             │  │
│  │    let graph = query_graph();  ───┐                       │  │
│  │    let result = invoke_query(     │                       │  │
│  │      "find_deadlocks",            │                       │  │
│  │      json!({})                    │                       │  │
│  │    );  ◄────────────────────────┐ │                       │  │
│  │  }                              │ │                       │  │
│  └─────────────────────────────────┼─┼───────────────────────┘  │
└────────────────────────────────────┼─┼──────────────────────────┘
                                     │ │
                            ┌────────┘ └──────────┐
                            │                     │
                            ▼                     │
┌─────────────────────────────────────────────────┼──────────────┐
│                  Extism Host (Rust)             │              │
│  ┌───────────────────────────────────────────┐  │              │
│  │  host_fn: specforge.invoke_query()       │  │              │
│  │  ├─ Extract query name: "find_deadlocks" │  │              │
│  │  ├─ Lookup in query_registry             │  │              │
│  │  ├─ Load Rhai script ─────────────────────┼──┘              │
│  │  │    (queries/find_deadlocks.rhai)      │                 │
│  │  ├─ Create Rhai Engine                   │                 │
│  │  ├─ Inject graph helpers:                │                 │
│  │  │    - graph.outgoing_edges()           │                 │
│  │  │    - graph.incoming_edges()           │                 │
│  │  │    - graph.filter_nodes()             │                 │
│  │  │    - graph.has_cycle()                │                 │
│  │  ├─ Execute Rhai script:                 │                 │
│  │  │    ┌─────────────────────────────────┐│                 │
│  │  │    │ fn query(graph, args) {         ││                 │
│  │  │    │   let edges = graph.edges_of(   ││                 │
│  │  │    │     "composes"                  ││                 │
│  │  │    │   );                            ││                 │
│  │  │    │   return graph.has_cycle(edges);││                 │
│  │  │    │ }                               ││                 │
│  │  │    └─────────────────────────────────┘│                 │
│  │  └─ Return result as JSON ───────────────┼─────────────────┤
│  └───────────────────────────────────────────┘                 │
│                                                                 │
│  Query extensions run in Rust (fast), not Wasm (slow).         │
│  Avoids serialization overhead for graph traversals.           │
└─────────────────────────────────────────────────────────────────┘
```

**Performance win**: Graph algorithms (BFS, DFS, cycle detection) run at native Rust speed, not Wasm speed.

---

## 5. Entity Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                         CORE ENTITIES                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ behavior │  │  event   │  │invariant │  │   type   │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
│       │             │              │             │              │
└───────┼─────────────┼──────────────┼─────────────┼──────────────┘
        │             │              │             │
        │ produces    │ consumes     │ uses_type   │
        │             │              │             │
        ▼             ▼              ▼             ▼
┌─────────────────────────────────────────────────────────────────┐
│              FORMAL ANALYSIS ENTITIES (Plugin)                   │
│                                                                  │
│  ┌──────────────┐   ┌───────────────┐   ┌────────────────┐     │
│  │  contract    │   │  refinement   │   │    process     │     │
│  ├──────────────┤   ├───────────────┤   ├────────────────┤     │
│  │ type: pre/   │   │ abstract: ref │   │ alphabet: []   │     │
│  │   post/inv   │   │ concrete: ref │   │ definition: "" │     │
│  │ expression   │   │ proof_obl: "" │   │ traces: []     │     │
│  │ enforced_by  │   └───────┬───────┘   └────┬───────────┘     │
│  └──────┬───────┘           │                │                 │
│         │                   │                │                 │
│         │ guards            │ refines        │ composes_with   │
│         │                   │                │                 │
│         ▼                   ▼                ▼                 │
│    ┌─────────┐         ┌─────────┐     ┌─────────┐           │
│    │behavior │         │behavior │     │ process │           │
│    │  (core) │         │  (core) │     │ (self)  │           │
│    └─────────┘         └─────────┘     └─────────┘           │
│                                                                │
└────────────────────────────────────────────────────────────────┘

Cross-plugin edges:
- contract --[guards]--> behavior (formal → core)
- refinement --[refines]--> behavior (formal → core)
- process --[traces]--> behavior (formal → core)
- behavior --[produces]--> event (core → core)
- behavior --[consumes]--> event (core → core)
```

---

## 6. Code Generation Integration

```
┌─────────────────────────────────────────────────────────────────┐
│                  Generator: @specforge/gen-rust                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Read graph JSON                                              │
│  2. Find all behaviors                                           │
│  3. For each behavior:                                           │
│     ┌──────────────────────────────────────────────────────┐    │
│     │  let contracts = graph.incoming_edges(               │    │
│     │    behavior.id,                                      │    │
│     │    "guards"  ◄── Edge from formal-analysis plugin    │    │
│     │  );                                                  │    │
│     │                                                      │    │
│     │  for contract in contracts:                         │    │
│     │    if contract.type == "precondition":              │    │
│     │      emit_doc_comment(contract.expression);         │    │
│     │      emit_debug_assert(contract.expression);        │    │
│     └──────────────────────────────────────────────────────┘    │
│                                                                  │
│  4. Emit Rust code:                                              │
│     ┌──────────────────────────────────────────────────────┐    │
│     │  impl AuthService {                                  │    │
│     │      /// # Precondition                              │    │
│     │      /// `username != null && password != null`      │    │
│     │      pub fn auth_login(                              │    │
│     │          &self,                                      │    │
│     │          username: &str,                             │    │
│     │          password: &str                              │    │
│     │      ) -> Result<Token> {                            │    │
│     │          debug_assert!(                              │    │
│     │              !username.is_empty() &&                 │    │
│     │              !password.is_empty(),                   │    │
│     │              "Contract violated: auth_login_precond" │    │
│     │          );                                          │    │
│     │          // ... implementation                       │    │
│     │      }                                               │    │
│     │  }                                                   │    │
│     └──────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Key**: Generators query formal-analysis entities via graph edges. No direct plugin-to-plugin communication needed.

---

## 7. LSP Performance Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                      LSP Event: textDocument/didChange           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Incremental parse (only changed files)                       │
│  2. Update graph (add/remove nodes/edges)                        │
│  3. Run validators:                                              │
│                                                                  │
│     ┌────────────────────────────────────────────┐              │
│     │  Fast Checks (< 50ms)  ✅ Run always       │              │
│     ├────────────────────────────────────────────┤              │
│     │  - V001: Empty contract expression         │              │
│     │  - V004: Unenforced contract               │              │
│     │  - V006: Missing proof obligation          │              │
│     └────────────────────────────────────────────┘              │
│                                                                  │
│     ┌────────────────────────────────────────────┐              │
│     │  Medium Checks (< 500ms)  ⚠️ Run on save   │              │
│     ├────────────────────────────────────────────┤              │
│     │  - V002: Refinement cycle (topo sort)      │              │
│     │  - V003: Process alphabet consistency      │              │
│     └────────────────────────────────────────────┘              │
│                                                                  │
│     ┌────────────────────────────────────────────┐              │
│     │  Slow Checks (> 1s)  ❌ Skip in LSP        │              │
│     ├────────────────────────────────────────────┤              │
│     │  - V005: Advanced deadlock detection       │              │
│     │    (only run in CLI: `specforge check`)    │              │
│     └────────────────────────────────────────────┘              │
│                                                                  │
│  4. Return diagnostics to LSP client                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Plugin checks execution context via graph metadata:
  graph.metadata.execution_context == "lsp" | "cli"
```

---

## 8. Custom Rule Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      USER PROJECT                                │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  .specforge/queries/custom_payment_rule.rhai               │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │  fn validate(graph) {                                │  │ │
│  │  │    let payment_behaviors = graph.filter_nodes(#{     │  │ │
│  │  │      kind: "behavior",                               │  │ │
│  │  │      name_contains: "payment"                        │  │ │
│  │  │    });                                               │  │ │
│  │  │                                                      │  │ │
│  │  │    for behavior in payment_behaviors {              │  │ │
│  │  │      let contracts = graph.incoming_edges(          │  │ │
│  │  │        behavior.id, "guards"                        │  │ │
│  │  │      );                                             │  │ │
│  │  │      if !contracts.any(|c|                          │  │ │
│  │  │        c.type == "invariant" &&                     │  │ │
│  │  │        c.expression.contains("balance")            │  │ │
│  │  │      ) {                                            │  │ │
│  │  │        emit_diagnostic(                            │  │ │
│  │  │          "warning", "CUSTOM001",                   │  │ │
│  │  │          "Payment missing balance invariant",      │  │ │
│  │  │          behavior.span                             │  │ │
│  │  │        );                                          │  │ │
│  │  │      }                                             │  │ │
│  │  │    }                                               │  │ │
│  │  │  }                                                 │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  specforge.json:                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  {                                                         │ │
│  │    "formal_analysis": {                                   │ │
│  │      "custom_rules": [                                    │ │
│  │        ".specforge/queries/custom_payment_rule.rhai"      │ │
│  │      ]                                                    │ │
│  │    }                                                      │ │
│  │  }                                                        │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              @specforge/formal-analysis Plugin                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  validate() {                                              │ │
│  │    // Run built-in validators (V001-V007)                  │ │
│  │    run_builtin_validators();                               │ │
│  │                                                            │ │
│  │    // Load custom rules from config                        │ │
│  │    let config = query_graph().metadata.formal_analysis;    │ │
│  │    for rule_path in config.custom_rules {                  │ │
│  │      let rhai_script = load_file(rule_path);               │ │
│  │      invoke_query_inline(rhai_script);                     │ │
│  │    }                                                       │ │
│  │  }                                                         │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

**Key**: Custom rules have same graph query API as plugin-bundled rules. No special privileges needed.

---

## 9. Comparison: Plugin vs New Extension Type

```
┌────────────────────────┬─────────────────┬──────────────────────┐
│ Feature                │ Plugin Model    │ New Extension Type   │
├────────────────────────┼─────────────────┼──────────────────────┤
│ Registers entities     │ ✅ Yes          │ ✅ Yes               │
│ Custom validation      │ ✅ Yes          │ ✅ Yes               │
│ Emits diagnostics      │ ✅ Yes          │ ✅ Yes               │
│ Reads graph            │ ✅ Yes          │ ✅ Yes               │
│ Query extensions       │ ✅ Yes (Rhai)   │ ✅ Yes (Rhai)        │
│ LSP integration        │ ✅ Yes          │ ✅ Yes               │
│ Code generation        │ ✅ Yes (via     │ ✅ Yes (same)        │
│                        │    graph edges) │                      │
├────────────────────────┼─────────────────┼──────────────────────┤
│ Implementation cost    │ ✅ Low (reuse)  │ ❌ High (new infra)  │
│ Conceptual clarity     │ ✅ "Plugin that │ ❌ "Separate thing"  │
│                        │    validates"   │                      │
│ User mental model      │ ✅ Simple       │ ❌ More complex      │
│ Manifest schema        │ ✅ Existing     │ ❌ New fields        │
│ CLI commands           │ ✅ specforge    │ ❌ specforge         │
│                        │    add/remove   │    add-analyzer?     │
│ Load order             │ ✅ Standard     │ ❌ New rules         │
│ Peer dependencies      │ ✅ Works        │ ⚠️ Need adaptation   │
└────────────────────────┴─────────────────┴──────────────────────┘

Verdict: Plugin model wins on all practical criteria.
```

---

## 10. Data Flow Summary

```
┌─────────────┐
│  .spec file │
└──────┬──────┘
       │ Parse
       ▼
┌─────────────┐     ┌──────────────────────────────┐
│   Parser    │────▶│  Initial graph (core only)   │
└─────────────┘     └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │  Load @specforge/            │
                    │    formal-analysis           │
                    │  ┌──────────────────────┐    │
                    │  │ register_entity():   │    │
                    │  │ - contract           │    │
                    │  │ - refinement         │    │
                    │  │ - process            │    │
                    │  └──────────────────────┘    │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │  Re-parse with new entities  │
                    │  (contract, refinement, etc) │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │  Complete graph              │
                    │  (core + formal entities)    │
                    └──────────────┬───────────────┘
                                   │
          ┌────────────────────────┼────────────────────────┐
          │                        │                        │
          ▼                        ▼                        ▼
┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│  Plugin:        │    │  Generator:      │    │  CLI output:     │
│  validate()     │    │  gen_rust()      │    │  specforge check │
│  ├─ V001-V007   │    │  ├─ Query        │    │  ├─ 0 errors     │
│  ├─ emit_diag   │    │  │   contracts   │    │  ├─ 2 warnings   │
│  └─ invoke_     │    │  ├─ Emit runtime │    │  └─ Success      │
│     query()     │    │  │   checks      │    │                  │
│                 │    │  └─ Emit tests   │    │                  │
└─────────────────┘    └──────────────────┘    └──────────────────┘
```

---

**End of Architecture Diagrams**

# Formal Methods Integration: Plugin Architecture Design

> [!CAUTION]
> **SUPERSEDED by RES-27** — Entity design (`contract`/`refinement`/`process`) and separate `@specforge/formal-analysis` plugin rejected. Formal methods syntax is integrated into `@specforge/software` as inline blocks (`requires`/`ensures`/`maintains`/`sync`) on existing entities. No new entity types.
>
> **What changed:**
> - Separate `@specforge/formal-analysis` plugin → formal methods syntax is part of `@specforge/software`
> - `contract`/`refinement`/`process` entities → rejected; inline `requires`/`ensures`/`maintains`/`sync` blocks on existing entities
> - `.rhai` query extensions → replaced by Wasm plugin host functions (`specforge.query_graph`)
> - `register_entity` for new formal types → not needed; existing entities gain formal syntax

**Expert**: Expert 10 — Plugin & Extension System Designer
**Date**: March 4, 2026
**Version**: 1.0
**Status**: superseded

---

## Executive Summary

After reviewing the three formal methods research documents (Design by Contract, B-Method, CSP) and SpecForge's extension model, I recommend **NO new extension type**. Formal analysis fits cleanly within the existing **plugin + validator** model, with one architectural addition: **query extension files** (already present in the manifest schema but not yet utilized).

**Key recommendation**: `@specforge/formal-analysis` should be a standard **plugin** that:
1. Registers new entities (`contract`, `refinement`, `process`) via `register_entity`
2. Provides graph query functions via `.rhai` query extensions (domain-specific mini-DSL)
3. Emits validation diagnostics via `emit_diagnostic`
4. Contributes custom validation rules via the `validators: true` contribution flag

This design is **composable**, **performant**, and **consistent** with SpecForge's Terraform-inspired extension model.

---

## 1. Extension Type Analysis

### Should formal analysis be a 4th extension type?

**No.** Here's why:

| Criterion | Plugin Model | New "Analyzer" Type |
|-----------|--------------|---------------------|
| **Registers entities** | ✅ Yes (`contract`, `refinement`, `process`) | ✅ Yes |
| **Validates constraints** | ✅ Yes (validators contribution) | ✅ Yes |
| **Emits diagnostics** | ✅ Yes (`emit_diagnostic`) | ✅ Yes |
| **Reads graph** | ✅ Yes (`query_graph`) | ✅ Yes |
| **New host functions needed** | ⚠️ Some (query extensions) | ⚠️ Same |
| **Conceptual clarity** | ✅ "Plugin that validates" | ❌ "Separate thing that validates" |
| **Implementation cost** | ✅ Low (reuse plugin infra) | ❌ High (new manifest fields, CLI commands, load order) |

**Verdict**: Use the existing **plugin** extension type with `contributes.validators: true`.

### Why plugins are sufficient

1. **Entity registration**: Formal methods need new block types (`contract`, `refinement`, `process`) — plugins already support this via `register_entity`.
2. **Custom validation**: Formal analysis is validation at its core — plugins already support custom validators.
3. **Graph queries**: The missing piece is **efficient graph traversal** — solved via query extensions (see Section 3).
4. **Composability**: Users enable/disable formal analysis by installing/removing the plugin, just like `@specforge/governance`.

---

## 2. New Host Functions

### 2.1 Query Extensions (Alternative to New Host Functions)

Instead of adding host functions like `specforge.query_event_flow`, I recommend **query extension files** — small Rhai scripts bundled with the plugin that provide domain-specific graph queries.

**Rationale**:
- **Performance**: Query logic runs in Rust (host side), not Wasm. Avoids serialization overhead.
- **Composability**: Multiple plugins can contribute query extensions without host function namespace collisions.
- **Type safety**: Rhai scripts are validated at plugin load time.
- **LSP integration**: Query extensions can be invoked from both CLI and LSP contexts.

**Manifest declaration**:
```json
{
  "extension": "@specforge/formal-analysis",
  "contributes": {
    "entities": true,
    "validators": true
  },
  "query_extensions": [
    {
      "name": "event_flow",
      "file": "queries/event_flow.rhai"
    },
    {
      "name": "check_refinement",
      "file": "queries/check_refinement.rhai"
    },
    {
      "name": "get_contracts",
      "file": "queries/get_contracts.rhai"
    },
    {
      "name": "find_deadlocks",
      "file": "queries/find_deadlocks.rhai"
    }
  ]
}
```

**Query extension API (Rhai)**:
```rhai
// queries/event_flow.rhai
// Returns all events reachable from a given behavior

fn query(graph, args) {
    let behavior_id = args["behavior_id"];
    let visited = [];
    let queue = graph.outgoing_edges(behavior_id, "produces");

    while queue.len() > 0 {
        let event = queue.pop();
        if visited.contains(event.id) { continue; }
        visited.push(event.id);

        let consumers = graph.incoming_edges(event.id, "consumes");
        for consumer in consumers {
            let produced = graph.outgoing_edges(consumer.id, "produces");
            queue.extend(produced);
        }
    }

    return visited;
}
```

**Wasm plugin invocation**:
```rust
// Inside the Wasm plugin (Rust)
let result = specforge::invoke_query("event_flow", json!({
    "behavior_id": "auth_login"
}));

let events: Vec<String> = serde_json::from_str(&result)?;
```

**Host function**:
```rust
// New host function: specforge.invoke_query(name, args_json) -> result_json
fn build_invoke_query(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.invoke_query",
        [extism::ValType::I64, extism::ValType::I64], // name, args
        [extism::ValType::I64], // result
        user_data,
        |plugin, inputs, outputs, ud| {
            // 1. Extract query name + args from Wasm memory
            let name = read_string(plugin, inputs[0])?;
            let args = read_string(plugin, inputs[1])?;

            // 2. Find query extension in registry
            let ctx = ud.get()?;
            let query_ext = ctx.lock().unwrap().query_registry.get(&name)
                .ok_or_else(|| extism::Error::msg(format!("Unknown query: {}", name)))?;

            // 3. Execute Rhai script with graph access
            let result = query_ext.execute(&ctx.lock().unwrap().graph_json, &args)?;

            // 4. Return result as JSON
            let handle = plugin.memory_new(&result)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}
```

### 2.2 Alternative: Specialized Host Functions

If query extensions prove too limited, add these host functions:

#### `specforge.query_subgraph(root_id, max_depth, edge_types) -> json`
Query a subgraph from a root node, traversing specific edge types up to a depth limit.

**Signature**:
```rust
Input: {
    "root_id": "auth_login",
    "max_depth": 3,
    "edge_types": ["produces", "consumes"],
    "direction": "outgoing" // or "incoming" or "both"
}

Output: {
    "nodes": [
        {"id": "auth_login", "kind": "behavior", "title": "User login"},
        {"id": "login_event", "kind": "event", "title": "Login completed"}
    ],
    "edges": [
        {"from": "auth_login", "to": "login_event", "type": "produces"}
    ]
}
```

**Use case**: Trace event flows, build dependency chains, find reachability.

#### `specforge.find_cycles(edge_types) -> json`
Find cycles in the graph when traversing specific edge types.

**Signature**:
```rust
Input: {
    "edge_types": ["consumes", "produces"]
}

Output: {
    "cycles": [
        ["auth_login", "login_event", "update_session", "session_event", "auth_login"]
    ]
}
```

**Use case**: Detect deadlocks (CSP), circular refinements (B-Method).

#### `specforge.filter_nodes(predicate) -> json`
Filter nodes by kind, field values, or custom predicates.

**Signature**:
```rust
Input: {
    "kind": "contract",
    "fields": {
        "type": "precondition"
    }
}

Output: {
    "nodes": [
        {"id": "auth_precond", "kind": "contract", "type": "precondition"},
        {"id": "payment_precond", "kind": "contract", "type": "precondition"}
    ]
}
```

**Use case**: Find all contracts of a given type, all invariants with severity > 8.

### 2.3 Recommendation

**Phase 1 (MVP)**: Use `query_graph` (full graph JSON) + Wasm-side graph algorithms.
**Phase 2 (optimization)**: Add query extensions via Rhai for hot paths.
**Phase 3 (if needed)**: Add specialized host functions for complex analyses.

---

## 3. Plugin Design: `@specforge/formal-analysis`

### 3.1 Manifest

```json
{
  "extension": "@specforge/formal-analysis",
  "manifest_version": "2",
  "version": "0.1.0",
  "description": "Formal verification support: DbC, B-Method, CSP",
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
    },
    {
      "name": "refinement",
      "testable": false,
      "fields": [
        {"name": "abstract", "type": "reference", "target": "behavior"},
        {"name": "concrete", "type": "reference", "target": "behavior"},
        {"name": "proof_obligation", "type": "string"}
      ]
    },
    {
      "name": "process",
      "testable": true,
      "fields": [
        {"name": "alphabet", "type": "string_list"},
        {"name": "definition", "type": "string", "required": true},
        {"name": "traces", "type": "reference_list"}
      ]
    }
  ],

  "dynamic_edge_types": [
    {
      "name": "refines",
      "source_kind": "behavior",
      "target_kind": "behavior"
    },
    {
      "name": "composes_with",
      "source_kind": "process",
      "target_kind": "process"
    },
    {
      "name": "guards",
      "source_kind": "contract",
      "target_kind": "behavior"
    }
  ],

  "query_extensions": [
    {
      "name": "event_flow",
      "file": "queries/event_flow.rhai",
      "description": "Trace event flow from a behavior"
    },
    {
      "name": "check_refinement",
      "file": "queries/check_refinement.rhai",
      "description": "Verify refinement correctness (B-Method)"
    },
    {
      "name": "find_deadlocks",
      "file": "queries/find_deadlocks.rhai",
      "description": "Detect deadlocks in process composition (CSP)"
    },
    {
      "name": "get_contracts",
      "file": "queries/get_contracts.rhai",
      "description": "Find contracts for a behavior"
    }
  ],

  "peer_dependencies": {
    "@specforge/product": "^0.1.0"
  },

  "sandbox": {
    "allowed_domains": [],
    "max_fuel": 50000000
  }
}
```

### 3.2 Registered Entities

#### `contract`
Design by Contract — preconditions, postconditions, invariants.

```spec
contract auth_login_precond {
  title "Login requires credentials"
  type precondition
  expression "username != null && password != null"
  enforced_by [auth_login]

  verify unit "should reject null credentials" {
    path "tests/auth_test.rs::auth_login_precond__rejects_null"
  }
}
```

**Fields**:
- `type`: `precondition` | `postcondition` | `invariant`
- `expression`: Human-readable contract (not parsed, for documentation + codegen hints)
- `enforced_by`: List of behaviors that must enforce this contract

**Testability**: Yes — contracts are runtime checks that require tests.

#### `refinement`
B-Method — stepwise refinement of abstract behaviors into concrete implementations.

```spec
refinement payment_flow_refinement {
  title "Payment flow implementation"
  abstract payment_process_abstract
  concrete payment_process_impl
  proof_obligation "preserve_balance_invariant"

  description """
  Refines the abstract payment process into a concrete implementation
  that maintains the balance invariant at each step.
  """
}
```

**Fields**:
- `abstract`: Reference to abstract behavior
- `concrete`: Reference to concrete behavior
- `proof_obligation`: Description of what must be proven (not mechanically checked)

**Testability**: No — refinement is a structural property verified by the plugin.

#### `process`
CSP — process algebra for concurrent system modeling.

```spec
process auth_service {
  title "Authentication service"
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

**Fields**:
- `alphabet`: Valid events for this process
- `definition`: CSP expression (not parsed, for documentation)
- `traces`: Behaviors that implement this process

**Testability**: Yes — process definitions should have integration tests.

### 3.3 Validation Rules

The plugin registers these validators (invoked during `specforge check`):

#### V001: Contract expression is not empty
```rust
for contract in graph.nodes_of_kind("contract") {
    if contract.field("expression").is_empty() {
        emit_diagnostic("error", "V001", "Contract expression cannot be empty", contract.span);
    }
}
```

#### V002: Refinement forms a DAG (no cycles)
```rust
let refinement_edges = graph.edges_of_type("refines");
if has_cycle(refinement_edges) {
    emit_diagnostic("error", "V002", "Refinement cycle detected", span);
}
```

#### V003: Process alphabet is consistent with events
```rust
for process in graph.nodes_of_kind("process") {
    let alphabet = process.field("alphabet");
    let traces = process.field("traces");
    for trace in traces {
        let produced_events = graph.outgoing_edges(trace, "produces");
        for event in produced_events {
            if !alphabet.contains(&event.id) {
                emit_diagnostic("warning", "V003",
                    format!("Event '{}' not in process alphabet", event.id), event.span);
            }
        }
    }
}
```

#### V004: Contract enforced by at least one behavior
```rust
for contract in graph.nodes_of_kind("contract") {
    let enforced_by = contract.field("enforced_by");
    if enforced_by.is_empty() {
        emit_diagnostic("warning", "V004", "Contract not enforced by any behavior", contract.span);
    }
}
```

#### V005: Deadlock detection (CSP)
```rust
// Use query_extension("find_deadlocks") to detect cycles in process composition
let result = invoke_query("find_deadlocks", json!({}));
let cycles: Vec<Vec<String>> = serde_json::from_str(&result)?;
if !cycles.is_empty() {
    emit_diagnostic("error", "V005", "Deadlock detected in process composition", span);
}
```

### 3.4 Diagnostic Codes

The plugin emits diagnostics using these codes (plugin-scoped):

| Code | Severity | Message |
|------|----------|---------|
| `V001` | Error | Contract expression is empty |
| `V002` | Error | Refinement cycle detected |
| `V003` | Warning | Event not in process alphabet |
| `V004` | Warning | Contract not enforced by any behavior |
| `V005` | Error | Deadlock detected in process composition |
| `V006` | Warning | Refinement lacks proof obligation |
| `V007` | Info | Formal analysis skipped (missing peer dependency) |

---

## 4. Composability

### 4.1 User Control

Users opt into formal analysis by installing the plugin:

```bash
specforge add @specforge/formal-analysis
```

The plugin is then declared in `specforge.json`:

```json
{
  "plugins": [
    "@specforge/product",
    "@specforge/formal-analysis"
  ]
}
```

### 4.2 Selective Analysis

Users can disable specific checks via configuration:

```json
{
  "plugins": [
    "@specforge/formal-analysis"
  ],
  "formal_analysis": {
    "enabled_checks": ["contract", "refinement"],
    "disabled_checks": ["deadlock"]
  }
}
```

Plugin reads this config via `query_graph` (config is part of the graph JSON).

### 4.3 Cross-Plugin Interaction

Formal analysis entities reference core and product entities:

```spec
contract payment_invariant {
  type invariant
  expression "balance >= 0"
  enforced_by [process_payment]  // core: behavior
}

capability payment_processing {
  contracts [payment_invariant]  // product: capability -> formal: contract
  behaviors [process_payment]
}
```

Cross-plugin edge: `capability --[guards]--> contract` (soft edge, `I004` if plugin not installed).

---

## 5. Performance

### 5.1 LSP (Real-Time) vs CLI (Batch)

| Analysis | Complexity | LSP | CLI |
|----------|-----------|-----|-----|
| **Contract validation** | O(n) | ✅ Yes | ✅ Yes |
| **Refinement DAG check** | O(n + e) | ✅ Yes (topological sort) | ✅ Yes |
| **Event flow tracing** | O(n + e) | ✅ Yes (BFS/DFS) | ✅ Yes |
| **Deadlock detection (simple)** | O(n + e) | ✅ Yes (cycle detection) | ✅ Yes |
| **Deadlock detection (advanced)** | O(2^n) | ❌ No (exponential) | ⚠️ Configurable timeout |
| **Full refinement proof** | Solver-dependent | ❌ No | ⚠️ External tool |

**LSP strategy**:
- Run **fast checks** (O(n), O(n log n)) on every keystroke (< 50ms budget).
- Run **medium checks** (O(n²)) on save (< 500ms budget).
- Run **slow checks** (exponential) only on explicit `specforge check` in CLI.

**Implementation**:
```rust
// Plugin checks its fuel budget and skips expensive analyses in LSP context
fn validate(graph_json: &str, context: &str) -> Result<Vec<Diagnostic>> {
    let is_lsp = context.contains("lsp");
    let mut diagnostics = vec![];

    // Always run fast checks
    diagnostics.extend(validate_contracts(graph_json)?);
    diagnostics.extend(validate_refinement_dag(graph_json)?);

    // Only run expensive checks in CLI mode
    if !is_lsp {
        diagnostics.extend(detect_deadlocks_advanced(graph_json)?);
    }

    Ok(diagnostics)
}
```

### 5.2 Caching Strategy

**CLI**:
- AOT-compiled Wasm (Extism `serialize()` API)
- Graph query results cached (unchanged subgraphs don't re-run)

**LSP**:
- Warm Wasm instances (reuse engine across edits)
- Incremental validation (only re-check changed entities + dependents)

---

## 6. Custom Proof Obligations

### 6.1 User-Defined Rules via Rhai

Users can define domain-specific formal rules by writing Rhai query extensions in their project:

**Project structure**:
```
project/
  specforge.json
  .specforge/
    queries/
      custom_rule.rhai
  spec/
    behaviors/
      auth.spec
```

**specforge.json**:
```json
{
  "formal_analysis": {
    "custom_rules": [
      ".specforge/queries/custom_rule.rhai"
    ]
  }
}
```

**`.specforge/queries/custom_rule.rhai`**:
```rhai
// Custom rule: every payment behavior must have a balance invariant
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

### 6.2 Plugin API for Custom Rules

The `@specforge/formal-analysis` plugin provides helper functions for Rhai scripts:

```rhai
// Graph query helpers
graph.filter_nodes(predicate)
graph.outgoing_edges(node_id, edge_type)
graph.incoming_edges(node_id, edge_type)
graph.find_paths(from, to, max_depth)
graph.has_cycle(edge_types)

// Diagnostic emission
emit_diagnostic(severity, code, message, span)

// Field access
node.field(name)
node.has_field(name)
node.kind
node.id
node.span
```

---

## 7. Integration with Code Generation

### 7.1 Contract Codegen

Language generators can query contracts and emit runtime checks:

**TypeScript generator** (`@specforge/gen-typescript`):
```typescript
// Queries @specforge/formal-analysis entities via graph JSON
const behavior = graph.findNode("auth_login");
const contracts = graph.outgoingEdges(behavior.id, "guards");

for (const contract of contracts) {
  if (contract.type === "precondition") {
    emit(`
      export function auth_login(username: string, password: string) {
        // @specforge-contract: ${contract.id}
        if (username == null || password == null) {
          throw new ContractViolation("${contract.expression}");
        }
        // ... implementation
      }
    `);
  }
}
```

**Rust generator** (`@specforge/gen-rust`):
```rust
// Use contract expressions as doc comments + debug_assert!
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

### 7.2 Test Scaffolding

`contract` and `process` are testable entities — generators emit test stubs:

```rust
#[cfg(test)]
mod auth_login_precond {
    #[test]
    #[specforge::test("auth_login_precond")]
    fn rejects_null_credentials() {
        // TODO: Implement contract test
    }
}
```

---

## 8. Comparison with Other Approaches

### 8.1 Why Not a Separate "Analyzer" Extension Type?

| Aspect | Plugin Model | New Extension Type |
|--------|--------------|-------------------|
| **Implementation** | Reuse existing plugin infra | Build new manifest schema, CLI commands, load order |
| **Consistency** | Formal analysis = validation plugin | New concept to learn |
| **Composability** | Enable/disable via plugin list | Need new `analyzers` field in config |
| **Host functions** | Reuse `query_graph`, `emit_diagnostic` | Need new namespace (`specforge.analyze.*`) |
| **Performance** | Same as plugins (Wasm AOT cache) | Same |
| **Ecosystem** | Plugins can contribute analyzers | Analyzers are separate from plugins |

**Verdict**: Plugin model is simpler and more consistent.

### 8.2 Why Not Builtin Core Feature?

Formal analysis should NOT be in the core compiler because:
1. **Niche**: Most users won't use formal methods (adds bloat to core).
2. **Evolving**: Formal methods research is active — plugins allow iteration.
3. **Domain-specific**: Different industries need different formal analyses (aviation ≠ fintech).
4. **Optional complexity**: Formal verification should be opt-in, not mandatory.

### 8.3 Why Not Subprocess JSON-RPC?

SpecForge's extension model uses **Wasm/Extism only** (no subprocess plugins). Reasons:
1. **Performance**: Wasm is faster than subprocess IPC for short-lived analyses.
2. **Portability**: Single `.wasm` binary works on all platforms (no Python/Node.js dependency).
3. **Security**: Wasm sandbox is stricter than OS process isolation.
4. **Distribution**: Wasm modules are easier to distribute (npm, GitHub Releases).

---

## 9. Implementation Roadmap

### Phase 1: MVP (v0.1.0) — 2 weeks
- [ ] Implement `contract`, `refinement`, `process` entity registration
- [ ] Implement V001-V004 validators (contract, refinement DAG, process alphabet)
- [ ] Test with example `.spec` files using formal entities
- [ ] Document in `docs/plugins/formal-analysis.md`

### Phase 2: Query Extensions (v0.2.0) — 2 weeks
- [ ] Implement `invoke_query` host function
- [ ] Write Rhai query extensions (`event_flow`, `check_refinement`, `find_deadlocks`)
- [ ] Benchmark query performance (LSP vs CLI)
- [ ] Add caching for query results

### Phase 3: Advanced Validation (v0.3.0) — 3 weeks
- [ ] Implement V005 (deadlock detection with timeout)
- [ ] Add custom rule support (user-provided Rhai scripts)
- [ ] Implement fuel limits for expensive analyses
- [ ] Add LSP integration (fast checks on keystroke, slow checks on save)

### Phase 4: Codegen Integration (v0.4.0) — 2 weeks
- [ ] Extend TypeScript generator to emit contract runtime checks
- [ ] Extend Rust generator to emit contract debug assertions
- [ ] Generate test scaffolding for `contract` and `process` entities
- [ ] Document codegen integration in language generator guides

### Phase 5: Proof Tooling (v0.5.0) — 4 weeks (optional)
- [ ] Integrate with external proof checkers (Coq, Isabelle, Z3)
- [ ] Generate proof obligations from refinements
- [ ] Validate proofs in CI pipeline
- [ ] Document proof workflow in `docs/formal-verification.md`

**Total**: 13 weeks (9 weeks for MVP + query + validation + codegen, 4 weeks for proof tooling)

---

## 10. Open Questions

### 10.1 Should refinement be testable?

**Current**: `testable: false` (refinement is structural, verified by plugin logic).
**Alternative**: `testable: true` if users write equivalence tests (abstract ≡ concrete).

**Recommendation**: Start with `testable: false`, add `testable: true` if users request it.

### 10.2 Should we parse contract expressions?

**Current**: Expressions are free-form strings (documentation + codegen hints).
**Alternative**: Parse expressions into AST for static checking.

**Recommendation**: Start with strings (lower barrier). Add optional expression DSL in v0.6.0 if needed.

### 10.3 Should we support multiple formal methods plugins?

**Scenario**: User installs both `@specforge/formal-analysis` and `@community/tla-plus`.

**Resolution**:
- Both plugins can register entities (no conflict if entity names differ).
- If entity names collide, use load-order priority (manifest version 2 supports `priority` field).
- Emit `W023` (load-order conflict resolution) if collision occurs.

**Recommendation**: Support multiple plugins, document conflict resolution in `docs/extension-model.md`.

---

## 11. Conclusion

**Recommendation summary**:
1. Use **existing plugin model** (no new extension type).
2. Add **query extensions** via Rhai for efficient graph traversal.
3. Register entities: `contract`, `refinement`, `process`.
4. Provide validators: V001-V007 (contract checks, refinement DAG, deadlock detection).
5. Enable custom rules via user-provided Rhai scripts.
6. Integrate with code generators (emit runtime checks + test scaffolding).
7. Optimize for LSP (fast checks) vs CLI (batch analyses).

This design is:
- ✅ **Composable** — users enable/disable via plugin installation.
- ✅ **Performant** — query extensions run in Rust, not Wasm.
- ✅ **Consistent** — reuses existing plugin infrastructure.
- ✅ **Extensible** — supports custom rules via Rhai.
- ✅ **Practical** — integrates with codegen and test traceability.

The formal methods integration is a **plugin**, not a new extension type. This aligns with SpecForge's philosophy: **small stable core + extensible plugins**.

---

## Appendix A: Host Function Signatures

### `specforge.invoke_query(name, args) -> result`

**Purpose**: Invoke a Rhai query extension registered by a plugin.

**Input**:
```json
{
  "name": "event_flow",
  "args": {
    "behavior_id": "auth_login",
    "max_depth": 5
  }
}
```

**Output**:
```json
{
  "events": ["login_event", "session_event", "audit_event"]
}
```

**Rust signature**:
```rust
fn build_invoke_query(user_data: extism::UserData<HostContext>) -> extism::Function {
    extism::Function::new(
        "specforge.invoke_query",
        [extism::ValType::I64, extism::ValType::I64], // name, args
        [extism::ValType::I64], // result
        user_data,
        |plugin, inputs, outputs, ud| {
            let name = read_json::<String>(plugin, inputs[0])?;
            let args = read_json::<serde_json::Value>(plugin, inputs[1])?;

            let ctx = ud.get()?;
            let ctx_guard = ctx.lock().unwrap();

            let query_ext = ctx_guard.query_registry.get(&name)
                .ok_or_else(|| extism::Error::msg(format!("Unknown query: {}", name)))?;

            let result = query_ext.execute(&ctx_guard.graph_json, &args)?;

            let handle = write_json(plugin, &result)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}
```

---

## Appendix B: Example Plugin Code

### Plugin Entry Point (Rust)

```rust
use extism_pdk::*;
use serde_json::json;

#[plugin_fn]
pub fn initialize() -> FnResult<()> {
    // Register entity kinds
    specforge::register_entity(json!({
        "name": "contract",
        "testable": true,
        "fields": [
            {"name": "type", "type": "enum", "values": ["precondition", "postcondition", "invariant"]},
            {"name": "expression", "type": "string", "required": true},
            {"name": "enforced_by", "type": "reference_list"}
        ]
    }))?;

    // Register edge types
    specforge::register_edge(json!({
        "name": "guards",
        "source_kind": "contract",
        "target_kind": "behavior"
    }))?;

    Ok(())
}

#[plugin_fn]
pub fn validate() -> FnResult<Vec<u8>> {
    let graph_json = specforge::query_graph()?;
    let graph: SpecGraph = serde_json::from_str(&graph_json)?;

    // V001: Contract expression is not empty
    for contract in graph.nodes_of_kind("contract") {
        if contract.field("expression").as_str().unwrap_or("").is_empty() {
            specforge::emit_diagnostic(json!({
                "severity": "error",
                "code": "V001",
                "message": "Contract expression cannot be empty",
                "file": contract.file,
                "line": contract.span.start_line,
                "column": contract.span.start_column
            }))?;
        }
    }

    // V002: Refinement forms a DAG
    let refinement_edges = graph.edges_of_type("refines");
    if has_cycle(&refinement_edges) {
        specforge::emit_diagnostic(json!({
            "severity": "error",
            "code": "V002",
            "message": "Refinement cycle detected"
        }))?;
    }

    Ok(vec![])
}

fn has_cycle(edges: &[(String, String)]) -> bool {
    // Topological sort algorithm
    // Returns true if cycle exists
    todo!()
}
```

### Query Extension (Rhai)

```rhai
// queries/event_flow.rhai

fn query(graph, args) {
    let behavior_id = args["behavior_id"];
    let max_depth = args.get("max_depth", 10);

    let visited = #{};
    let result = [];

    fn traverse(node_id, depth) {
        if depth > max_depth || visited.contains(node_id) {
            return;
        }
        visited[node_id] = true;

        let events = graph.outgoing_edges(node_id, "produces");
        for event in events {
            result.push(event.id);

            let consumers = graph.incoming_edges(event.id, "consumes");
            for consumer in consumers {
                traverse(consumer.id, depth + 1);
            }
        }
    }

    traverse(behavior_id, 0);

    return #{
        "events": result
    };
}
```

---

## Appendix C: Validation Code Reference

| Code | Severity | Description |
|------|----------|-------------|
| `V001` | Error | Contract expression is empty |
| `V002` | Error | Refinement cycle detected |
| `V003` | Warning | Event not in process alphabet |
| `V004` | Warning | Contract not enforced by any behavior |
| `V005` | Error | Deadlock detected in process composition |
| `V006` | Warning | Refinement lacks proof obligation |
| `V007` | Info | Formal analysis skipped (missing peer dependency) |

---

## Appendix D: Entity Schema Reference

### `contract`

```spec
contract <id> {
  title       <string>          // Optional (auto-derived from ID)
  type        <enum>            // precondition | postcondition | invariant
  expression  <string>          // Contract formula (free-form)
  enforced_by [<behavior_id>]   // Behaviors that enforce this contract

  verify <kind> "<name>" {      // Optional test declarations
    path "<file>::<function>"
  }

  scenario "<title>" {          // Optional scenarios
    given "<step>"
    when  "<step>"
    then  "<step>"
  }
}
```

### `refinement`

```spec
refinement <id> {
  title            <string>      // Optional
  abstract         <behavior_id> // Abstract behavior
  concrete         <behavior_id> // Concrete implementation
  proof_obligation <string>      // What must be proven (free-form)
}
```

### `process`

```spec
process <id> {
  title      <string>          // Optional
  alphabet   [<string>]        // Valid event names
  definition <string>          // CSP expression (free-form)
  traces     [<behavior_id>]   // Behaviors implementing this process

  scenario "<title>" {         // Optional scenarios
    given "<step>"
    when  "<step>"
    then  "<step>"
  }
}
```

---

**End of Document**

# RES-21e: SpecForge Architecture Constraints for Extensibility

**Expert perspective:** SpecForge codebase architect — analysis of what CAN and CANNOT be extended
**Decision outcome:** Wasm/Extism adopted as unified plugin runtime. Extension surface area analyzed below with Wasm-specific annotations.

---

## 1. Current Architecture: What's Hardcoded

### Entity Types — Compile-Time Enum

```rust
// crates/specforge-common/src/lib.rs
pub enum EntityKind {
    Spec, Invariant, Behavior, Feature, Event, Type, Port, Ref,  // Core 8
    Capability, Deliverable, Roadmap, Library, Glossary,          // Product 5
    Decision, Constraint, FailureMode,                            // Governance 3
}
```

All 16 entity kinds are Rust enum variants. Adding a new kind requires recompiling. Rust's exhaustive match enforcement means every `match entity_kind { ... }` must handle all variants.

### Tree-sitter Grammar — Compiled to C

```javascript
// crates/tree-sitter-specforge/grammar.js
_block: ($) => choice(
    $.spec_block,
    $.invariant_block,
    $.behavior_block,
    $.feature_block,
    $.event_block,
    $.type_block,
    $.port_block,
    $.ref_block,
    $.capability_block,
    $.deliverable_block,
    $.roadmap_block,
    $.library_block,
    $.glossary_block,
    $.decision_block,
    $.constraint_block,
    $.failure_mode_block,
    $.define_block,  // <-- ONLY generic extension point
),
```

Tree-sitter compiles `grammar.js` to `parser.c` which is statically linked into the Rust binary. **Cannot be modified at runtime.** This constraint applies equally to all runtimes — Wasm, Lua, Rhai, or subprocess. Wasm plugins operate downstream of parsing, on the already-parsed graph.

### Edge Types — Compile-Time Enum

```rust
pub enum EdgeType {
    References, Implements, Produces, Consumes,  // etc.
    // All 20 edge types hardcoded
}
```

### Validation Passes — Hardcoded Functions

```rust
// crates/specforge-validator/src/passes.rs
pub fn validate(files: &[SpecFile], graph: &SpecGraph, config: &CompilerConfig) {
    check_orphan_behaviors(graph, &mut bag);      // W001
    check_unused_invariants(graph, &mut bag);      // W003
    // ... dozens of hardcoded validation functions
}
```

### Plugin Manifests — Hardcoded Constructors

```rust
// crates/specforge-common/src/plugin_manifest.rs
impl PluginManifest {
    pub fn product() -> Self { /* hardcoded */ }
    pub fn governance() -> Self { /* hardcoded */ }
    pub fn for_module(module: Module) -> Option<Self> {
        match module {
            Module::Core => None,
            Module::Product => Some(Self::product()),
            Module::Governance => Some(Self::governance()),
        }
    }
}
```

**Note:** As Wasm plugins mature, `PluginManifest` will be loaded from `.wasm` module exports rather than hardcoded constructors. The hardcoded versions serve as the bootstrap for official plugins.

## 2. What IS Extensible

### `define` Blocks (Limited)

The grammar includes a generic `define_block` that allows custom entity types:

```
define epic {
    title = "Epic tracking"
    fields {
        title: string
        stories: reference_list -> feature
    }
}
```

**Limitations:**
- Parsed as generic key-value pairs (no entity-specific syntax)
- No custom validation rules (Wasm plugins fill this gap)
- No custom edge types (Wasm plugins fill this gap)
- Second-class citizens in the graph

### Generators (Fully Extensible via Wasm)

Wasm plugins call `specforge.emit_file(path, content)` to produce output files. Host validates paths and enforces sandboxing.

### Providers (Fully Extensible via Wasm)

Wasm plugins call `specforge.http_get(url)` for external service validation. Host mediates all network access.

## 3. Extension Surface Area Map (Updated for Wasm)

| Component | Can Wasm Extend? | Mechanism | Notes |
|-----------|-----------------|-----------|-------|
| **Tree-sitter grammar** | NO | — | Compile-time, immutable |
| **Entity kinds** | Via `define` + Wasm validation | `specforge.register_entity` | Parsed as `define` blocks, Wasm adds validation |
| **Edge types** | Via Wasm registration | `specforge.register_edge` | Custom edges between entities |
| **AST types** | NO | — | Upstream of Wasm |
| **Validation passes** | YES | `specforge.query_graph` + `specforge.emit_diagnostic` | Primary Wasm use case |
| **Graph queries** | YES (read-only) | `specforge.query_graph` | All plugins share same graph |
| **Code generation** | YES | `specforge.emit_file` | Host validates output paths |
| **Ref providers** | YES | `specforge.http_get` | Host mediates network access |
| **LSP completions** | YES (via contributions) | Plugin manifest `completions` | Merged with built-in completions |
| **LSP diagnostics** | YES | Same as validation | Diagnostics merged into LSP stream |
| **MCP tools** | YES | Plugin manifest `tools` | Dynamically registered |

## 4. The `define` + Wasm Enhancement Path

`define` blocks handle parsing (what tree-sitter can do). Wasm plugins handle everything else (validation, edges, generation).

### `define` Block (Parsing Layer)
```
define epic {
    title = "Epic tracking"
    testable = false

    fields {
        title: string @required
        points: integer @optional
        stories: reference_list -> feature
    }
}
```

### Wasm Plugin (Validation + Logic Layer)
```rust
// In Rust Wasm plugin
#[plugin_fn]
pub fn validate(input: Json<GraphSnapshot>) -> FnResult<Json<Vec<Diagnostic>>> {
    let mut diagnostics = vec![];
    for entity in input.entities_of_kind("epic") {
        let stories = entity.field_ref_list("stories");
        if stories.is_empty() {
            diagnostics.push(Diagnostic {
                code: "W050",
                severity: Warning,
                message: format!("Epic '{}' has no stories", entity.name),
                span: entity.span.clone(),
            });
        }
    }
    Ok(Json(diagnostics))
}
```

This two-layer approach gives full extensibility:
- `define` handles **what the parser sees** (fields, types, references)
- Wasm handles **what the compiler enforces** (validation, custom edges, generation)

## 5. Wasm Plugin Lifecycle

```
1. specforge reads specforge.json → discovers plugins
2. Validates peer_dependencies for all declared Wasm plugins
3. Topologically sorts: core → @specforge/product → @specforge/governance → third-party
4. CLI: AOT-compile .wasm modules (cached in .specforge/cache/)
5. LSP/MCP: keep warm Extism engine instances
6. Load each plugin, call initialize() → registers entities, edges, host function bindings
7. Parse .spec files (tree-sitter — no plugins involved)
8. Build graph (core + define entities)
9. Call validate() on each plugin → collect diagnostics
10. For generators: call generate() → each plugin emits files via specforge.emit_file
11. Merge all diagnostics with built-in validation
12. Output results
```

## 6. What About LSP and MCP?

### LSP Extensions

Wasm plugins contribute to the LSP via the same host functions:
- **Diagnostics**: `specforge.emit_diagnostic` — merged into LSP diagnostic stream
- **Completions**: plugin manifest declares completion triggers, host calls plugin for items
- **Code actions**: plugin manifest declares code action kinds

The LSP keeps warm Wasm engines — zero per-edit startup cost.

### MCP Extensions

Wasm plugins register MCP tools via their manifest:

```json
{
    "tools": [
        {
            "name": "specforge_validate_epics",
            "description": "Validate epic entities",
            "inputSchema": { ... }
        }
    ]
}
```

The MCP server dynamically registers these tools. When invoked, the host calls the plugin's Wasm function.

## 7. Architectural Summary

```
┌─────────────────────────────────────────────┐
│  Tree-sitter Parser (compile-time, static)  │
│  Handles: 16 built-in blocks + define       │
│  NOT extensible at runtime                  │
└──────────────────┬──────────────────────────┘
                   │ AST
                   ▼
┌─────────────────────────────────────────────┐
│  Graph Builder (Rust, compile-time)         │
│  Builds nodes + edges from AST             │
│  define entities get generic graph nodes    │
└──────────────────┬──────────────────────────┘
                   │ Graph
                   ▼
┌─────────────────────────────────────────────┐
│  Wasm Plugin Layer (Extism runtime)         │
│  Handles: validation, generation, providers │
│  Host functions: query_graph, emit_*,       │
│    register_entity, register_edge, http_get │
│  Sandboxed: hardware-enforced isolation     │
└─────────────────────────────────────────────┘
```

The boundary is clear: **tree-sitter and the graph builder are static**. Everything downstream — validation, generation, ref resolution, LSP contributions, MCP tools — is extensible via Wasm plugins.

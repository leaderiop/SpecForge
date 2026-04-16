# Extension Protocol

The Extension Protocol defines how extensions communicate with the SpecForge host through a Wasm-based bidirectional interface, replacing the static `manifest.json` approach with a live negotiation model.

## Overview

Extensions are Wasm modules that the host loads, interrogates, and invokes at runtime. The protocol has four phases: **handshake**, **describe**, **operate**, and **disconnect**. The host drives all interactions. Extensions never call the host unprompted.

This design upholds Principle 2 (the compiler knows nothing about your domain) by ensuring that all domain vocabulary enters the compiler through a single, uniform protocol. The host does not know what an extension will contribute until it asks.

```
Host                                Extension (.wasm)
 |                                        |
 |-- load wasm binary ------------------>|
 |                                        |
 |-- __handshake(host_version) --------->|
 |<-- name, version, protocol_version, --|
 |    contribution_flags, sandbox_policy  |
 |                                        |
 |-- __describe("entities") ------------>|
 |<-- entity kind descriptors ------------|
 |                                        |
 |-- __describe("edges") --------------->|
 |<-- edge type descriptors --------------|
 |                                        |
 |-- __describe("validation_rules") ---->|
 |<-- validation rule descriptors --------|
 |                                        |
 |-- (register contributions) ---------->|  (internal)
 |                                        |
 |-- cmd__validate(args) --------------->|  (on demand)
 |<-- result -----------------------------|
 |                                        |
 |-- (disconnect) ---------------------->|  (graceful)
```

## Discovery Protocol

### Handshake

The handshake is the first call the host makes after loading a Wasm binary. It establishes identity, compatibility, and contribution scope.

**Export:** `__handshake`

**Input:**

```json
{
  "host_version": "1.2.0"
}
```

**Output:**

```json
{
  "name": "@specforge/software",
  "version": "1.0.0",
  "protocol_version": "1.0.0",
  "ext_short": "software",
  "host_api_version": "1.0.0",
  "incremental": true,
  "query_scope": "all",
  "starter_template": "templates/behavior.spec",
  "migration_hook": "migrate_v1_to_v2",
  "reserved_keywords": ["spec", "ref"],
  "contribution_flags": {
    "entities": true,
    "validators": true,
    "renderers": false,
    "providers": false,
    "collectors": true,
    "prompts": false,
    "parsers": false,
    "grammars": false,
    "body_parsers": true
  },
  "sandbox_policy": {
    "max_memory_mb": 256,
    "max_execution_ms": 5000,
    "network_access": false,
    "file_system_access": false
  },
  "peer_dependencies": [
    { "name": "@specforge/product", "version": "^1.0", "optional": false }
  ]
}
```

The `contribution_flags` object tells the host which categories this extension contributes to. The host uses these flags to decide which `__describe` calls to make. An extension that sets `entities: false` will never receive a `__describe("entities")` call.

The host checks `protocol_version` for compatibility. If the extension declares a protocol version the host does not support, the host emits a diagnostic and skips the extension.

### Describe

After the handshake, the host calls `__describe(category)` for each category the extension flagged as `true`. The host decides which categories it needs based on context:

- The **CLI** skips MCP tool schemas (it uses CLI command descriptors instead)
- The **MCP server** skips DOT visualization colors (it uses MCP tool schemas instead)
- The **LSP** requests entity kinds with full LSP metadata (semantic tokens, icons)
- **All contexts** request entities, edges, fields, and validation rules

**Export:** `__describe`

**Input:**

```json
{
  "category": "entities"
}
```

**Output (varies by category):**

The response structure depends on the requested category. Each category returns an array of descriptors specific to that category.

### Describe Categories

The protocol defines 11 contribution categories:

| Category | What it returns | When the host requests it |
|----------|----------------|--------------------------|
| `entities` | Entity kind descriptors (keyword, fields, LSP metadata, DOT metadata) | Always |
| `edges` | Edge type descriptors (label, source/target kind, visual style) | Always |
| `fields` | Shared field descriptors applied to all entity kinds | Always |
| `enhancements` | Field enhancements on other extensions' entity kinds | Always |
| `validation_rules` | Declarative and custom validation rule descriptors | Always |
| `surfaces` | CLI commands, MCP tools, MCP resources | CLI, MCP |
| `grammars` | Grammar contribution descriptors | When `grammars` flag is true |
| `body_parsers` | Body parser descriptors | When `body_parsers` flag is true |
| `collectors` | Collector descriptors with auto-detection config | When `collectors` flag is true |
| `passes` | Compiler pass descriptors with ordering constraints | When extension declares passes |
| `feature_flags` | Feature flag descriptors with allowed values and defaults | Always |

### Category: entities

Returns entity kind descriptors. Each descriptor declares a DSL keyword, its fields, and metadata for LSP and DOT rendering.

```json
{
  "category": "entities",
  "items": [
    {
      "name": "Behavior",
      "keyword": "behavior",
      "description": "Behavioral contract for a single operation",
      "testable": true,
      "singleton": false,
      "supports_verify": true,
      "allowed_verify_kinds": ["smoke", "contract"],
      "incremental": true,
      "has_body_parser": false,
      "open_fields": false,
      "semantic_token": "function",
      "lsp_icon": "Method",
      "dot_shape": "box",
      "dot_color": "#1565C0",
      "dot_fillcolor": "#E3F2FD",
      "fields": [
        {
          "name": "contract",
          "field_type": "block",
          "required": true,
          "description": "The behavioral contract"
        },
        {
          "name": "invariants",
          "field_type": "reference_list",
          "edge": "BehaviorEnforcesInvariant",
          "target_kind": "invariant",
          "description": "Invariants this behavior enforces"
        }
      ]
    }
  ]
}
```

### Category: edges

Returns edge type descriptors. Each descriptor declares a labeled relationship between entity kinds.

```json
{
  "category": "edges",
  "items": [
    {
      "label": "BehaviorEnforcesInvariant",
      "description": "This behavior enforces these invariants",
      "source_kind": "behavior",
      "target_kind": "invariant",
      "edge_style": "dashed",
      "edge_color": "#C62828",
      "edge_arrowhead": "normal"
    }
  ]
}
```

### Category: fields

Returns shared field descriptors. Shared fields are applied to all entity kinds declared by this extension (and can be overridden per-kind).

```json
{
  "category": "fields",
  "items": [
    {
      "name": "tags",
      "field_type": "string_list",
      "description": "Freeform labels"
    }
  ]
}
```

### Category: enhancements

Returns field enhancements that add fields to entity kinds owned by other extensions. This is the mechanism for cross-extension composition.

```json
{
  "category": "enhancements",
  "items": [
    {
      "target_kind": "module",
      "source_extension": "@specforge/product",
      "fields": [
        {
          "name": "ports",
          "field_type": "reference_list",
          "edge": "ModuleConsumesPort",
          "target_kind": "port",
          "description": "Port interfaces this module consumes"
        }
      ]
    }
  ]
}
```

### Category: validation_rules

Returns both declarative rules (pattern-based, evaluated by the host) and custom rules (Wasm-backed, evaluated by calling extension exports).

```json
{
  "category": "validation_rules",
  "items": [
    {
      "code": "W001",
      "severity": "warning",
      "message_template": "behavior '{id}' does not implement any feature",
      "check": "no_outgoing_edges",
      "target_kind": "behavior",
      "edge_type": "BehaviorImplementsFeature"
    },
    {
      "code": "W009",
      "severity": "warning",
      "message_template": "{kind} '{id}' has verify kind '{value}' not in allowed set {allowed}",
      "check": "custom",
      "wasm_function": "validate__verify_kind_allowlist"
    }
  ]
}
```

Declarative check types:

| Check | Behavior |
|-------|----------|
| `no_incoming_edges` | Warns when entity has no incoming edges of the specified type |
| `no_outgoing_edges` | Warns when entity has no outgoing edges of the specified type |
| `missing_field_when_flag_set` | Warns when a field is empty but a condition is met |
| `field_value_constraint` | Warns when a field value violates a pattern or enum constraint |
| `cycle_detection` | Errors when edges of the specified type form a cycle |
| `file_exists` | Errors when a file-reference field points to a nonexistent file |
| `custom` | Delegates to a `validate__*` Wasm export |

### Category: surfaces

Returns CLI command, MCP tool, and MCP resource descriptors. CLI commands auto-promote to MCP tools. Each surface declares its arguments, sandbox overrides, and export name.

```json
{
  "category": "surfaces",
  "items": {
    "commands": [
      {
        "id": "validate",
        "title": "Run validation",
        "description": "Run product validation rules",
        "category": "analysis",
        "export": "cmd__validate",
        "args": [
          { "name": "path", "arg_type": "path", "required": true, "description": "Path to spec root" },
          { "name": "lint", "arg_type": "enum", "required": false, "description": "Lint profile", "default": "default" }
        ],
        "sandbox": { "fs_read": true }
      }
    ],
    "mcp_tools": [
      {
        "name": "model",
        "description": "Generate entity model",
        "category": "visualization",
        "export": "mcp__model",
        "input_schema": { "type": "object", "properties": { "format": { "type": "string" } } }
      }
    ],
    "mcp_resources": [
      {
        "uri": "specforge://entities/{kind}",
        "name": "entity_list",
        "description": "List entities by kind",
        "mime_type": "application/json",
        "export": "mcp__entity_list"
      }
    ]
  }
}
```

### Category: grammars

Returns grammar contributions. Each contribution associates a Tree-sitter grammar Wasm module with an entity kind.

```json
{
  "category": "grammars",
  "items": [
    {
      "entity_kind": "type",
      "grammar_wasm_path": "type_grammar.wasm",
      "export_name": "parse_type"
    }
  ]
}
```

### Category: body_parsers

Returns body parser contributions. Each contribution associates a Wasm export with an entity kind for parsing body content.

```json
{
  "category": "body_parsers",
  "items": [
    {
      "entity_kind": "type",
      "export_name": "parse__type_fields"
    }
  ]
}
```

### Category: collectors

Returns collector contributions with auto-detection configuration.

```json
{
  "category": "collectors",
  "items": [
    {
      "name": "rust",
      "input_formats": ["junit-xml", "json"],
      "export": "collect__rust",
      "auto_detect": {
        "file_patterns": ["**/target/**/junit.xml"],
        "env_vars": ["CARGO_TARGET_DIR"]
      }
    }
  ]
}
```

### Category: passes

Returns compiler pass descriptors. Each pass declares ordering constraints relative to the built-in resolve phase and other passes.

```json
{
  "category": "passes",
  "items": [
    {
      "name": "condition_check",
      "after": "resolve",
      "description": "Validate structured condition consistency"
    },
    {
      "name": "layering_verify",
      "after": "condition_check",
      "description": "Verify specification layering constraints"
    }
  ]
}
```

### Category: feature_flags

Returns feature flag descriptors. Each flag declares allowed values and a default.

```json
{
  "category": "feature_flags",
  "items": [
    {
      "name": "warning_level",
      "values": ["default", "strict"],
      "default": "default",
      "description": "Controls which warnings are emitted"
    }
  ]
}
```

## Host API

The protocol is bidirectional. While the host drives the conversation (calling exports on the Wasm module), the extension can call back into the host via imported functions. This is how extensions read the graph, emit diagnostics, and resolve references during validation and command execution.

### Imported Host Functions

Extensions import these functions from the host:

| Function | Signature | Purpose |
|----------|-----------|---------|
| `query` | `(pattern: &str) -> Vec<Entity>` | Query entities by kind, field values, or graph pattern |
| `emit_diagnostic` | `(severity: Severity, code: &str, msg: &str)` | Emit a diagnostic to the host's diagnostic collection |
| `resolve_ref` | `(id: &str) -> Option<EntityRef>` | Resolve an entity ID to a typed reference |
| `read_file` | `(path: &str) -> Option<String>` | Read a file from the project (subject to sandbox policy) |

### query

Query the entity graph using a pattern string. Returns matching entities with their fields and edges.

```
query("kind:behavior")
  -> all behavior entities

query("kind:behavior AND field:priority=high")
  -> behaviors where priority is high

query("edges_from:login_flow AND edge_type:BehaviorImplementsFeature")
  -> features connected to login_flow via Implements edges
```

### emit_diagnostic

Emit a diagnostic from within a validator, command, or compiler pass. The host collects all diagnostics and presents them according to the current output mode (terminal, JSON, LSP).

```
emit_diagnostic(Warning, "W001", "behavior 'create_user' does not implement any feature")
```

### resolve_ref

Resolve a string identifier to an entity reference. Returns `None` if the entity is not in the graph (which may indicate the owning extension is not installed).

```
resolve_ref("user_management")
  -> Some(EntityRef { kind: "feature", extension: "@specforge/product", ... })

resolve_ref("nonexistent")
  -> None
```

### read_file

Read a file from the project filesystem. Subject to the extension's sandbox policy. Returns `None` if the file does not exist or the sandbox denies access.

```
read_file("behaviors/auth.spec")
  -> Some("behavior login { ... }")

read_file("/etc/passwd")
  -> None  (sandbox denied)
```

### Host API Versioning

The host API is versioned via the `host_api_version` field in the handshake response. The host checks this version and provides backward-compatible function signatures.

| Host API Version | Functions Available |
|-----------------|-------------------|
| `1.0.0` | `query`, `emit_diagnostic`, `resolve_ref`, `read_file` |

Future host API versions will add new functions without removing existing ones. An extension requesting `host_api_version: "1.0.0"` will always receive the 1.0.0 function signatures, even on a host that supports 2.0.0.

### Sandbox Policy

The sandbox policy declared in the handshake controls what an extension can access. The host enforces these limits at the Wasm runtime level.

```json
{
  "max_memory_mb": 256,
  "max_execution_ms": 5000,
  "network_access": false,
  "file_system_access": false,
  "allowed_domains": [],
  "allowed_paths": [],
  "allowed_output_extensions": []
}
```

| Policy | Default | Effect |
|--------|---------|--------|
| `max_memory_mb` | 256 | Maximum Wasm linear memory allocation |
| `max_execution_ms` | 5000 | Maximum wall-clock time per export call |
| `network_access` | false | Whether `fetch`-style host functions are available |
| `file_system_access` | false | Whether `read_file` host function is available |
| `allowed_domains` | [] | If network enabled, restrict to these domains |
| `allowed_paths` | [] | If filesystem enabled, restrict to these path prefixes |
| `allowed_output_extensions` | [] | Restrict file writes to these extensions |

Individual surfaces (CLI commands, MCP tools) can override the extension-level sandbox policy. This allows a generally sandboxed extension to grant filesystem read access to a specific command that needs it.

## Hot Plug and Unplug

Extensions can connect and disconnect at runtime without restarting the host. This enables live extension management in the LSP and MCP server contexts.

### Connect

Connection follows the full lifecycle: load, handshake, describe, register.

```
1. Host loads .wasm binary from extension directory
2. Host calls __handshake(host_version)
3. Host validates protocol_version compatibility
4. Host checks peer_dependencies are satisfied
5. Host calls __describe(category) for each flagged category
6. Host registers contributions in KindRegistry, FieldRegistry, EdgeRegistry
7. Extension is now "connected"
8. Host calls cmd__*, validate__*, mcp__* exports as needed
```

### Disconnect

When an extension disconnects (removed, crashed, or explicitly unloaded), the host performs graceful degradation:

**Entities** from the disconnected extension become untyped nodes in the graph. They retain their IDs, fields, and connections, but lose:
- Field validation (unknown fields accepted)
- Type checking on reference targets
- Entity-specific DOT/LSP metadata

**Edges** declared by the disconnected extension remain in the graph as untyped connections. They retain source and target, but lose label semantics.

**Validation rules** from the disconnected extension stop firing. No false positives from rules that reference entity kinds that no longer have type information.

**Surfaces** (CLI commands, MCP tools, MCP resources) from the disconnected extension are removed from the registry. Calls to those surfaces return "extension not available" errors.

**Enhancements** contributed to other extensions' entity kinds are removed. Enhanced fields on those entity kinds revert to unrecognized fields (accepted but not validated).

### Reconnect

When a previously disconnected extension reconnects, the host repeats the full lifecycle from step 1. All entities and edges from the extension regain their type information. Validation rules resume. Surfaces become available again.

This matches SpecForge's existing soft reference philosophy. An entity referencing another entity from an uninstalled extension produces an `I004` info diagnostic rather than an error. The same principle applies at runtime: disconnection degrades gracefully, reconnection restores full semantics.

### Disconnect Diagnostics

When the host detects entities or references that belong to a disconnected extension, it emits `I004`:

```
info[I004]: Unknown entity 'create_user' in field 'behaviors'
  |
3 |   behaviors [create_user]
  |              ^^^^^^^^^^^ extension '@specforge/software' not connected
  |
  = help: The extension may have been disconnected. Reconnect it to restore validation.
```

## Lifecycle Summary

```
                    ┌─────────────────────────────────────────┐
                    │               HOST                       │
                    │                                         │
   ┌────────────┐  │  ┌──────────┐   ┌──────────────────┐   │
   │  .wasm     │──┼─>│  LOAD    │──>│  __handshake()   │   │
   │  binary    │  │  └──────────┘   │  -> metadata     │   │
   └────────────┘  │                 │  -> flags        │   │
                    │                 │  -> sandbox      │   │
                    │                 │  -> peers        │   │
                    │                 └────────┬─────────┘   │
                    │                          │              │
                    │                          v              │
                    │                 ┌──────────────────┐   │
                    │                 │  __describe()    │   │
                    │                 │  per category    │   │
                    │                 │  -> entities     │   │
                    │                 │  -> edges        │   │
                    │                 │  -> rules        │   │
                    │                 │  -> surfaces     │   │
                    │                 └────────┬─────────┘   │
                    │                          │              │
                    │                          v              │
                    │                 ┌──────────────────┐   │
                    │                 │  REGISTER        │   │
                    │                 │  KindRegistry    │   │
                    │                 │  FieldRegistry   │   │
                    │                 │  EdgeRegistry    │   │
                    │                 └────────┬─────────┘   │
                    │                          │              │
                    │                          v              │
                    │                 ┌──────────────────┐   │
                    │                 │  CONNECTED       │   │
                    │                 │  cmd__*          │   │
                    │                 │  validate__*     │   │
                    │                 │  mcp__*          │   │
                    │                 │  parse__*        │   │
                    │                 │  collect__*      │   │
                    │                 └────────┬─────────┘   │
                    │                          │              │
                    │                          v              │
                    │                 ┌──────────────────┐   │
                    │                 │  DISCONNECT      │   │
                    │                 │  - entities ->   │   │
                    │                 │    untyped nodes │   │
                    │                 │  - rules stop   │   │
                    │                 │  - surfaces     │   │
                    │                 │    removed      │   │
                    │                 └────────┬─────────┘   │
                    │                          │              │
                    │                          v              │
                    │                 ┌──────────────────┐   │
                    │                 │  RECONNECT       │   │
                    │                 │  (repeat from    │   │
                    │                 │   LOAD)          │   │
                    │                 └──────────────────┘   │
                    │                                         │
                    └─────────────────────────────────────────┘
```

## Wasm Export Naming Conventions

All extension exports follow a strict naming convention that the host uses to dispatch calls:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `__handshake` | Protocol handshake | `__handshake` |
| `__describe` | Category description | `__describe` |
| `cmd__` | CLI command execution | `cmd__validate` |
| `validate__` | Custom validation logic | `validate__verify_kind_allowlist` |
| `mcp__` | MCP tool or resource execution | `mcp__model` |
| `parse__` | Body parser execution | `parse__type_fields` |
| `collect__` | Collector execution | `collect__rust` |

## Comparison with manifest.json

The Extension Protocol replaces the static `manifest.json` approach while maintaining the same contribution model.

| Aspect | manifest.json (v2) | Extension Protocol |
|--------|-------------------|-------------------|
| Discovery | Parse JSON file at startup | `__handshake` + `__describe` at load time |
| Contribution model | Same 18 contribution categories | Same 18 contribution categories |
| Host API | Not available | Bidirectional: `query`, `emit_diagnostic`, `resolve_ref`, `read_file` |
| Hot plug | Requires restart | Connect/disconnect at runtime |
| Context-aware loading | All metadata loaded always | Host requests only needed categories |
| Validation | Declarative only | Declarative + custom Wasm validators |
| Type safety | JSON schema validation | Protocol-typed responses |

The contribution categories, entity kind descriptors, edge type descriptors, and all other metadata structures remain identical. The protocol wraps them in a negotiation layer that enables runtime flexibility and bidirectional communication.

## Design Principles

The Extension Protocol embodies three SpecForge principles:

**Principle 2 (zero domain knowledge in core):** The host never hardcodes knowledge of what extensions will contribute. It discovers everything through `__handshake` and `__describe`.

**Principle 7 (extensions over built-ins):** The protocol is the sole mechanism for adding domain vocabulary. There is no alternative path that bypasses it.

**Principle 8 (seconds to value):** The protocol supports lazy loading. The host only calls `__describe` for categories it needs in the current context, avoiding unnecessary Wasm execution during startup.

# Extension SDK

The `specforge-extension-sdk` crate provides the types, host API bindings, and attribute macros that extension authors use to build SpecForge extensions as Wasm modules.

## Overview

An extension is a standalone Rust crate that compiles to `wasm32-wasi`. The SDK is the only dependency it needs. The SDK provides:

- **Protocol types** -- entity kind descriptors, edge type descriptors, field descriptors, and all other metadata structures the host expects
- **Host API bindings** -- typed wrappers around the imported host functions (`query`, `emit_diagnostic`, `resolve_ref`, `read_file`)
- **Attribute macros** -- declarative macros that generate Wasm exports conforming to the Extension Protocol
- **Shared types** -- `Entity`, `EntityRef`, `Diagnostic`, `Graph`, and other types used in both host and extension code

One crate, one compile target, one import. No hand-written JSON manifests. No manual Wasm export registration.

## Extension Structure

Extensions live in the `extensions/` directory. Each extension is a standalone Rust crate with its own `Cargo.toml`:

```
extensions/
  software/
    Cargo.toml
    src/
      lib.rs          -- #[extension] module with all contributions
    templates/
      behavior.spec   -- starter template
  product/
    Cargo.toml
    src/
      lib.rs
  governance/
    Cargo.toml
    src/
      lib.rs
  formal/
    Cargo.toml
    src/
      lib.rs
  software-testing/
    Cargo.toml
    src/
      lib.rs
```

### Cargo.toml

```toml
[package]
name = "specforge-ext-software"
version = "1.0.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
specforge-extension-sdk = "1.0.0"
```

### Compile Target

Extensions compile to `wasm32-wasi`:

```bash
cargo build --target wasm32-wasi --release
```

The output `.wasm` file is what the host loads at runtime.

## Authoring Experience

The SDK uses attribute macros to generate all protocol exports from declarative Rust code. You describe what your extension contributes; the SDK generates the `__handshake`, `__describe`, and all `cmd__*` / `validate__*` / `mcp__*` / `parse__*` / `collect__*` exports.

### Complete Example

This is the canonical reference for the macro API. It shows every macro the SDK provides, applied to a realistic extension:

```rust
use specforge_extension_sdk::prelude::*;

#[extension(
    name = "@specforge/software",
    version = "1.0.0",
    short = "software",
    host_api = "1.0.0",
    incremental = true,
    query_scope = "all",
    starter = "templates/behavior.spec",
    migration = "migrate_v1_to_v2",
    reserved_keywords = ["spec", "ref"],
)]
#[sandbox(max_memory_mb = 256, max_execution_ms = 5000, network = false, filesystem = false)]
#[peer_dependency("@specforge/product", version = "^1.0")]
mod software {

    // ── Shared Fields ─────────────────────────────────────────────

    // Shared fields are applied to ALL entity kinds declared by this
    // extension. Individual entity kinds can override a shared field
    // by declaring a field with the same name.

    #[shared_field(field_type = "string_list", description = "Freeform labels")]
    struct tags;

    // ── Entity Kinds ──────────────────────────────────────────────

    // Each entity kind becomes a DSL keyword. The struct name is the
    // display name; the keyword attribute is the DSL keyword.
    // Fields on the struct become entity fields with type, edge,
    // and target_kind metadata.

    #[entity_kind(keyword = "behavior", singleton = false, open_fields = false, has_body_parser = false)]
    #[lsp(semantic_token = "function", icon = "Method")]
    #[dot(shape = "box", color = "#1565C0", fillcolor = "#E3F2FD")]
    struct Behavior {
        #[field(required, description = "The behavioral contract")]
        contract: String,

        #[field(description = "Invariants this behavior enforces")]
        #[edge("BehaviorEnforcesInvariant", target = "invariant", style = "dashed", color = "#C62828")]
        invariants: Vec<EntityRef>,

        #[field(description = "Product features this behavior implements")]
        #[edge("BehaviorImplementsFeature", target = "feature", style = "solid")]
        features: Vec<EntityRef>,
    }

    // ── Entity Enhancements ───────────────────────────────────────

    // Enhancements add fields to entity kinds owned by other extensions.
    // The target_kind and owner identify the foreign entity kind.
    // Edges declared in enhancement fields are registered as cross-
    // extension edge types.

    #[enhance(target_kind = "module", owner = "@specforge/product")]
    struct ModuleEnhancement {
        #[field(description = "Port interfaces this module consumes")]
        #[edge("ModuleConsumesPort", target = "port", style = "dashed")]
        ports: Vec<EntityRef>,
    }

    // ── Standalone Edge Types ─────────────────────────────────────

    // Edge types that exist independently of any field declaration.
    // Use these for edges that are computed by validation rules or
    // compiler passes rather than declared in entity fields.

    #[edge_type(label = "References", description = "General cross-reference")]
    const REFERENCES: EdgeType;

    // ── Declarative Validation Rules ──────────────────────────────

    // Declarative rules are evaluated by the host using pattern matching.
    // No Wasm call is needed -- the host matches the check pattern against
    // the graph and emits the diagnostic if the pattern matches.

    #[validation_rule(
        code = "W001", severity = "warning",
        check = "no_outgoing_edges",
        target_kind = "behavior",
        edge_type = "BehaviorImplementsFeature",
        message = "behavior '{id}' does not implement any feature",
    )]
    const ORPHAN_BEHAVIOR: ValidationRule;

    // ── Custom Validators ─────────────────────────────────────────

    // Custom validators are Wasm-backed. The SDK generates a
    // validate__* export that the host calls during validation.
    // The function receives the entity being validated and the
    // host API for querying the graph.

    #[validator(code = "W009", severity = "warning",
        message = "{kind} '{id}' has verify kind '{value}' not in allowed set {allowed}")]
    fn validate_verify_kind_allowlist(entity: &Entity, host: &HostApi) -> Vec<Diagnostic> {
        // Custom logic: check entity's verify kinds against
        // the allowed set for its entity kind.
        let allowed = host.resolve_ref(&entity.kind)
            .map(|k| k.allowed_verify_kinds.clone())
            .unwrap_or_default();

        entity.verify_kinds.iter()
            .filter(|vk| !allowed.contains(vk))
            .map(|vk| Diagnostic::warning(
                "W009",
                format!("{} '{}' has verify kind '{}' not in allowed set {:?}",
                    entity.kind, entity.id, vk, allowed),
            ))
            .collect()
    }

    // ── CLI Commands ──────────────────────────────────────────────

    // CLI commands are exposed as `specforge <ext-short> <command-id>`.
    // They auto-promote to MCP tools. The SDK generates a cmd__*
    // export and a surface descriptor.

    #[cli_command(id = "validate", title = "Run validation",
        description = "Run product validation rules", category = "analysis")]
    #[sandbox_override(fs_read = true)]
    fn cmd_validate(
        #[arg(required, description = "Path to spec root")] path: PathArg,
        #[arg(default = "default", description = "Lint profile")] lint: EnumArg,
    ) -> Result<()> {
        let entities = host().query("kind:behavior")?;
        for entity in entities {
            if entity.edges_out("BehaviorImplementsFeature").is_empty() {
                host().emit_diagnostic(
                    Severity::Warning,
                    "W001",
                    &format!("behavior '{}' does not implement any feature", entity.id),
                );
            }
        }
        Ok(())
    }

    // ── MCP Tools ─────────────────────────────────────────────────

    // MCP tools are exposed via the MCP server. The SDK generates
    // an mcp__* export and a surface descriptor with JSON Schema
    // input validation.

    #[mcp_tool(name = "model", description = "Generate entity model", category = "visualization")]
    fn mcp_model(#[arg(description = "Output format")] format: Option<String>) -> Result<String> {
        let fmt = format.unwrap_or_else(|| "markdown".to_string());
        let entities = host().query("kind:*")?;
        // Render model in requested format...
        Ok(render_model(&entities, &fmt))
    }

    // ── MCP Resources ─────────────────────────────────────────────

    // MCP resources are read-only endpoints exposed via the MCP server.
    // The URI template uses `{param}` placeholders that the MCP server
    // resolves from the resource request.

    #[mcp_resource(uri = "specforge://entities/{kind}", name = "entity_list",
        description = "List entities by kind", mime_type = "application/json")]
    fn resource_entity_list(kind: &str, host: &HostApi) -> Result<String> {
        let entities = host.query(&format!("kind:{kind}"))?;
        Ok(serde_json::to_string(&entities)?)
    }

    // ── Grammar Contributions ─────────────────────────────────────

    // Grammar contributions associate a Tree-sitter grammar Wasm
    // module with an entity kind. The host loads the grammar and
    // uses it to parse entity body content.

    #[grammar(entity_kind = "type", wasm_path = "type_grammar.wasm")]
    const TYPE_GRAMMAR: GrammarContribution;

    // ── Body Parsers ──────────────────────────────────────────────

    // Body parsers handle structured content inside entity blocks.
    // The SDK generates a parse__* export that the host calls when
    // parsing entities of the specified kind.

    #[body_parser(kind = "type")]
    fn parse_type_fields(content: &str, host: &HostApi) -> ParseResult {
        // Parse structured type body content...
        ParseResult::ok(fields)
    }

    // ── Collectors ────────────────────────────────────────────────

    // Collectors ingest test results in external formats and produce
    // specforge-report.json entries. The auto_detect macro configures
    // automatic discovery of result files.

    #[collector(name = "rust", formats = ["junit-xml", "json"])]
    #[auto_detect(files = ["**/target/**/junit.xml"], env = ["CARGO_TARGET_DIR"])]
    fn collect_rust(input: &[u8], host: &HostApi) -> CollectionResult {
        // Parse JUnit XML or JSON input...
        // Map test results to entity IDs...
        CollectionResult::ok(results)
    }

    // ── Compiler Passes ───────────────────────────────────────────

    // Compiler passes run after the built-in resolve phase. The `after`
    // attribute declares ordering constraints. Passes receive the full
    // graph and return diagnostics.

    #[compiler_pass(name = "condition_check", after = "resolve")]
    fn pass_condition_check(graph: &Graph, host: &HostApi) -> Vec<Diagnostic> {
        // Validate structured condition consistency...
        diagnostics
    }

    // ── Feature Flags ─────────────────────────────────────────────

    // Feature flags let users configure extension behavior via
    // specforge.json. The host reads the flag value and passes it
    // to the extension when needed.

    #[feature_flag(name = "warning_level", values = ["default", "strict"], default = "default")]
    const WARNING_LEVEL: FeatureFlag;
}
```

### Extension-Level Attributes

The `#[extension]` macro is the root declaration. It generates the `__handshake` export and wires all nested contributions into `__describe` responses.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Scoped package name (e.g., `@specforge/software`) |
| `version` | yes | Semantic version (e.g., `1.0.0`) |
| `short` | yes | Short name used in CLI subcommands (e.g., `software`) |
| `host_api` | yes | Required host API version (e.g., `1.0.0`) |
| `incremental` | no | Default incremental mode for entity kinds (default: `false`) |
| `query_scope` | no | Graph query scope (`all` or `own`, default: `own`) |
| `starter` | no | Path to starter template file |
| `migration` | no | Wasm export name for migration hook |
| `reserved_keywords` | no | Keywords reserved from entity kind names |

The `#[sandbox]` macro sets the extension-level sandbox policy:

| Attribute | Required | Description |
|-----------|----------|-------------|
| `max_memory_mb` | no | Maximum Wasm memory in megabytes |
| `max_execution_ms` | no | Maximum execution time per call |
| `network` | no | Enable network access (default: `false`) |
| `filesystem` | no | Enable filesystem access (default: `false`) |

The `#[peer_dependency]` macro declares dependencies on other extensions:

```rust
#[peer_dependency("@specforge/product", version = "^1.0")]
#[peer_dependency("@specforge/governance", version = "^1.0", optional = true)]
```

## Contribution Surface Reference

Every macro maps to a protocol category. The SDK generates the appropriate Wasm exports and metadata descriptors.

| Macro | Generates | Protocol Category |
|-------|-----------|-------------------|
| `#[extension]` | `__handshake()` export | handshake |
| `#[entity_kind]` | Entity kind descriptor | `entities` |
| `#[field]` | Field descriptor on entity kind | `entities` |
| `#[edge]` | Edge type from reference field | `edges` |
| `#[edge_type]` | Standalone edge type | `edges` |
| `#[shared_field]` | Extension-wide field | `fields` |
| `#[enhance]` | Entity enhancement | `enhancements` |
| `#[validation_rule]` | Declarative validation rule | `validation_rules` |
| `#[validator]` | Custom Wasm validator + `validate__*` export | `validation_rules` |
| `#[cli_command]` | `cmd__*` export | `surfaces` |
| `#[mcp_tool]` | `mcp__*` export | `surfaces` |
| `#[mcp_resource]` | `mcp__*` export | `surfaces` |
| `#[grammar]` | Grammar contribution | `grammars` |
| `#[body_parser]` | `parse__*` export | `body_parsers` |
| `#[collector]` | `collect__*` export | `collectors` |
| `#[compiler_pass]` | Pass descriptor | `passes` |
| `#[feature_flag]` | Flag descriptor | `feature_flags` |
| `#[sandbox]` | Sandbox policy | handshake |
| `#[sandbox_override]` | Per-surface sandbox | `surfaces` |
| `#[peer_dependency]` | Dependency declaration | handshake |
| `#[lsp]` | LSP metadata on entity kind | `entities` |
| `#[dot]` | DOT visualization metadata | `entities` |
| `#[arg]` | CLI/MCP argument descriptor | `surfaces` |
| `#[auto_detect]` | Collector auto-detection config | `collectors` |

## Macro Details

### #[entity_kind]

Declares a DSL keyword that the core grammar will parse. The struct name is the display name; the `keyword` attribute is the DSL keyword.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `keyword` | yes | DSL keyword (lowercase, used in `.spec` files) |
| `singleton` | no | Whether only one instance is allowed (default: `false`) |
| `open_fields` | no | Whether unknown fields are accepted (default: `false`) |
| `has_body_parser` | no | Whether this kind uses a custom body parser (default: `false`) |

### #[field]

Declares a field on an entity kind. Place it on a struct field inside an `#[entity_kind]` struct.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `required` | no | Whether the field must be present (default: `false`) |
| `description` | no | Human-readable description |
| `file_reference` | no | Whether string values are file paths (default: `false`) |

The field's Rust type determines the `field_type`:

| Rust Type | Protocol Field Type |
|-----------|-------------------|
| `String` | `string` |
| `bool` | `boolean` |
| `i64` | `integer` |
| `Vec<String>` | `string_list` |
| `EntityRef` | `reference` |
| `Vec<EntityRef>` | `reference_list` |
| `Block` | `block` |
| `Enum` | `enum` |

### #[edge]

Declares an edge type derived from a reference field. Place it on a struct field that has type `EntityRef` or `Vec<EntityRef>`.

| Attribute | Required | Description |
|-----------|----------|-------------|
| (positional) | yes | Edge type label |
| `target` | yes | Target entity kind keyword |
| `style` | no | DOT edge style (`solid`, `dashed`, `dotted`) |
| `color` | no | DOT edge color (hex) |

### #[lsp] and #[dot]

Attach LSP and DOT visualization metadata to an entity kind.

```rust
#[lsp(semantic_token = "function", icon = "Method")]
#[dot(shape = "box", color = "#1565C0", fillcolor = "#E3F2FD")]
```

The host requests this metadata only in contexts that need it (LSP server, DOT renderer).

### #[validation_rule]

Declares a rule the host evaluates without calling the extension.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `code` | yes | Diagnostic code (e.g., `W001`) |
| `severity` | yes | `error`, `warning`, or `info` |
| `check` | yes | Check pattern (see Extension Protocol) |
| `target_kind` | depends | Entity kind to check (required for most checks) |
| `edge_type` | depends | Edge type to check (for edge-based checks) |
| `message` | yes | Message template with `{id}`, `{kind}`, `{value}`, `{allowed}` placeholders |

### #[validator]

Declares a custom Wasm-backed validator. The SDK generates a `validate__*` export.

```rust
#[validator(code = "W009", severity = "warning",
    message = "{kind} '{id}' has verify kind '{value}' not in allowed set {allowed}")]
fn validate_verify_kind_allowlist(entity: &Entity, host: &HostApi) -> Vec<Diagnostic> {
    // Return empty Vec for no diagnostics, or Vec<Diagnostic> for findings
}
```

### #[cli_command]

Declares a CLI command. The SDK generates a `cmd__*` export and a surface descriptor.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `id` | yes | Command identifier (used in `specforge <ext-short> <id>`) |
| `title` | yes | Human-readable title |
| `description` | yes | Detailed description |
| `category` | no | Command category for grouping |

### #[arg]

Declares an argument on a CLI command or MCP tool.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `required` | no | Whether the argument must be provided (default: `false`) |
| `description` | no | Human-readable description |
| `default` | no | Default value as string |

Argument types:

| Rust Type | Protocol Arg Type |
|-----------|------------------|
| `PathArg` | `path` |
| `String` | `string` |
| `bool` | `boolean` |
| `EnumArg` | `enum` |
| `Option<T>` | optional variant of inner type |

### #[collector]

Declares a test result collector. The SDK generates a `collect__*` export.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Collector name (used in `specforge collect <name>`) |
| `formats` | yes | Accepted input formats (e.g., `["junit-xml", "json"]`) |

The `#[auto_detect]` companion macro configures automatic discovery:

| Attribute | Required | Description |
|-----------|----------|-------------|
| `files` | no | Glob patterns for result files |
| `env` | no | Environment variables indicating result locations |

### #[compiler_pass]

Declares a compiler pass that runs after the built-in resolve phase.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Pass name (must be unique across all extensions) |
| `after` | yes | Pass or phase this pass runs after (`resolve` or another pass name) |

### #[feature_flag]

Declares a feature flag configurable via `specforge.json`.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Flag name (used as key in `specforge.json`) |
| `values` | yes | Allowed values |
| `default` | yes | Default value (must be in `values`) |

## Host API Bindings

The SDK provides typed bindings for the host functions imported at the Wasm boundary.

### Accessing the Host

Inside any export function, call `host()` to get a reference to the `HostApi`:

```rust
fn cmd_validate(path: PathArg, lint: EnumArg) -> Result<()> {
    let behaviors = host().query("kind:behavior")?;
    let file = host().read_file("specforge.json")?;
    host().emit_diagnostic(Severity::Info, "I100", "validation complete");
    Ok(())
}
```

### HostApi Methods

```rust
impl HostApi {
    /// Query entities by pattern. Returns matching entities with fields and edges.
    fn query(&self, pattern: &str) -> Result<Vec<Entity>>;

    /// Emit a diagnostic to the host's collection.
    fn emit_diagnostic(&self, severity: Severity, code: &str, message: &str);

    /// Resolve an entity ID to a typed reference.
    fn resolve_ref(&self, id: &str) -> Option<EntityRef>;

    /// Read a file from the project. Subject to sandbox policy.
    fn read_file(&self, path: &str) -> Option<String>;
}
```

### Entity Type

The `Entity` type represents an entity in the graph as seen by extension code:

```rust
pub struct Entity {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub fields: HashMap<String, FieldValue>,
    pub verify_kinds: Vec<String>,
}

impl Entity {
    /// Get outgoing edges of a specific type.
    fn edges_out(&self, edge_type: &str) -> Vec<&EntityRef>;

    /// Get incoming edges of a specific type.
    fn edges_in(&self, edge_type: &str) -> Vec<&EntityRef>;

    /// Get a field value by name.
    fn field(&self, name: &str) -> Option<&FieldValue>;
}
```

## Building and Testing

### Build

```bash
cd extensions/software
cargo build --target wasm32-wasi --release
```

The output `.wasm` file is at `target/wasm32-wasi/release/specforge_ext_software.wasm`.

### Test

Extensions can be tested with standard `cargo test` (native target) for logic, and with the SDK's test harness for protocol conformance:

```rust
#[cfg(test)]
mod tests {
    use specforge_extension_sdk::test::*;

    #[test]
    fn handshake_returns_valid_metadata() {
        let ext = TestExtension::load("target/wasm32-wasi/release/specforge_ext_software.wasm");
        let metadata = ext.handshake("1.0.0");
        assert_eq!(metadata.name, "@specforge/software");
        assert!(metadata.contribution_flags.entities);
    }

    #[test]
    fn describe_entities_returns_behavior() {
        let ext = TestExtension::load("target/wasm32-wasi/release/specforge_ext_software.wasm");
        let entities = ext.describe("entities");
        assert!(entities.iter().any(|e| e.keyword == "behavior"));
    }
}
```

### Install

Copy the `.wasm` file to the extension directory and register it:

```bash
specforge add ./extensions/software/target/wasm32-wasi/release/specforge_ext_software.wasm
```

Or for published extensions:

```bash
specforge add @specforge/software
```

## Enhancement-Only Extensions

Extensions that contribute no entity kinds of their own -- only enhancements to other extensions' entity kinds -- follow the same structure but declare zero entity kinds:

```rust
#[extension(
    name = "@specforge/software-testing",
    version = "1.0.0",
    short = "testing",
    host_api = "1.0.0",
)]
#[peer_dependency("@specforge/software", version = "^1.0")]
#[peer_dependency("@specforge/product", version = "^1.0")]
mod software_testing {

    // No #[entity_kind] declarations -- enhancement-only extension

    #[enhance(target_kind = "behavior", owner = "@specforge/software")]
    struct BehaviorTestEnhancement {
        #[field(description = "BDD scenario files", file_reference = true)]
        gherkin: Vec<String>,
    }

    #[enhance(target_kind = "feature", owner = "@specforge/product")]
    struct FeatureTestEnhancement {
        #[field(description = "BDD scenario files", file_reference = true)]
        gherkin: Vec<String>,
    }

    #[edge_type(label = "TestedBy", description = "Entity is tested by these files")]
    const TESTED_BY: EdgeType;

    #[validation_rule(
        code = "W004", severity = "warning",
        check = "missing_field_when_flag_set",
        target_kind = "behavior",
        field = "gherkin",
        message = "behavior '{id}' has gherkin field but no files referenced",
    )]
    const EMPTY_GHERKIN: ValidationRule;

    #[collector(name = "cucumber", formats = ["junit-xml", "json"])]
    #[auto_detect(files = ["**/cucumber-report.json", "**/cucumber-report.xml"])]
    fn collect_cucumber(input: &[u8], host: &HostApi) -> CollectionResult {
        // Parse Cucumber/Gherkin test results...
        CollectionResult::ok(results)
    }
}
```

## Related Documentation

- [Extension Protocol](extension-protocol.md) -- the wire protocol this SDK implements
- [Extension Inventory](extension-inventory.md) -- the five official extensions
- [Extension Model](extension-model.md) -- the broader extension architecture (extensions, providers, renderers)
- [Entity Model](entity-model.md) -- entity kinds, edge types, and validation rules

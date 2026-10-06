# Extension SDK

> **Status: component model (v2).** The SDK lives in `crates/specforge-extension-sdk`
> (+ the `specforge_extension_sdk::extension` attribute macro and the
> `component_guest!` macro), with a working example at `fixtures/greet-extension/`.
> Guests are **wasip2 components** (target `wasm32-wasip2`, wit-bindgen 0.30) and
> export the `specforge:bridge` world: `call(name, export-name, input) ->
> result<list<u8>, string>` dispatches `__handshake` / `__describe` from the
> `ContributionsBuilder`, every declared surface and operation to the handler
> declared with it, and any other export name to the guest's `handler`. Host functions (the old `HostApi`) were removed with the extism
> runtime; a typed component host-import surface is future work. The design
> reference below predates the cutover; where it mentions `wasm32-unknown-unknown`,
> extism, or `plugin_fn`, read `wasm32-wasip2`, the component engine, and
> `component_guest!`.

The `specforge-extension-sdk` crate provides the wire types and attribute macros that extension authors use to build SpecForge extensions as Wasm modules.

## Overview

An extension is a standalone Rust crate that compiles to `wasm32-wasip2`. The SDK is the only dependency it needs. The SDK provides:

- **Protocol types** -- entity kind descriptors, edge type descriptors, field descriptors, and all other metadata structures the host expects
- **Host functions (planned)** -- the typed import surface is future work; guests today are pure-compute and receive all context as call input (see [Host Functions](#host-functions))
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

Extensions compile to `wasm32-unknown-unknown`:

```bash
cargo build --target wasm32-wasip2 --release
```

The output `.wasm` file is what the host loads at runtime.

## Authoring Experience

The SDK uses attribute macros to generate all protocol exports from declarative Rust code. You describe what your extension contributes; the SDK generates the `__handshake`, `__describe`, and all `cmd__*` / `mcp__*` / `__pass_*` / `collect__*` / `validate__*` / `scan__*` exports, each routed to the handler declared with it.

### Complete Example

This is the canonical reference for the macro API. It shows every macro the SDK provides, applied to a realistic extension:

```rust
use specforge_extension_sdk::prelude::*;

#[extension(
    name = "@specforge/software",
    version = "1.0.0",
    short = "software",
    description = "Software design: behaviors, invariants, events, types and ports",
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
    // The host precomputes everything the validator needs — the entity,
    // its resolved reference targets, declared type ids, and the known
    // primitive set — into a ValidatorContext snapshot. Guests are
    // pure functions of that snapshot: no host calls are needed (or
    // possible) today.

    #[validator(code = "W009", severity = "warning",
        message = "{kind} '{id}' has verify kind '{value}' not in allowed set {allowed}")]
    fn validate_verify_kind_allowlist(context: ValidatorContext) -> ValidatorVerdict {
        // Custom logic: work from the precomputed snapshot — resolved
        // reference targets live in `context.referenced` (`kind: null`
        // marks a dangling reference), declared types in
        // `context.declared_types` — instead of graph lookups.
        let dangling = context.referenced.iter().any(|r| r.kind.is_none());
        if dangling {
            ValidatorVerdict::Fail {
                field: Some("verify_kinds".to_string()),
                value: Some(context.entity.id.clone()),
            }
        } else {
            ValidatorVerdict::Pass
        }
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
        // Guests are pure functions of their input today: the host passes
        // the parsed spec path and arguments; returned output is the
        // command's result. Graph queries and diagnostics go through the
        // planned host-function surface (see "Host Functions" below).
        let report = validate_behaviors(&path, &lint)?;
        println!("{report}");
        Ok(())
    }

    // ── MCP Tools ─────────────────────────────────────────────────

    // MCP tools are exposed via the MCP server. The SDK generates
    // an mcp__* export and a surface descriptor with JSON Schema
    // input validation.

    #[mcp_tool(name = "model", description = "Generate entity model", category = "visualization")]
    fn mcp_model(#[arg(description = "Output format")] format: Option<String>) -> Result<String> {
        let fmt = format.unwrap_or_else(|| "markdown".to_string());
        // Render the model from the snapshot the host passed in...
        Ok(render_model(&model_snapshot(), &fmt))
    }

    // ── MCP Resources ─────────────────────────────────────────────

    // MCP resources are read-only endpoints exposed via the MCP server.
    // The URI template uses `{param}` placeholders that the MCP server
    // resolves from the resource request.

    #[mcp_resource(uri = "specforge://entities/{kind}", name = "entity_list",
        description = "List entities by kind", mime_type = "application/json")]
    fn resource_entity_list(kind: &str) -> Result<String> {
        // The host resolves the URI template and passes the extracted
        // parameters; the export returns the resource payload.
        Ok(serde_json::to_string(&entities_of_kind(kind))?)
    }

    // ── Grammars and body parsers ─────────────────────────────────

    // Not supported. The `grammars` and `body_parsers` contribution
    // flags are reserved: the host ignores them. A kind whose body
    // syntax the core grammar doesn't parse sets `has_body_parser` on
    // its entity descriptor, and the compiler then leaves parse errors
    // inside those entities unreported.

    // ── Collectors ────────────────────────────────────────────────

    // Test collection belongs to runner extensions (@specforge/cargo-test,
    // @specforge/vitest), which declare a collector with the
    // `c.collector(...)` builder; see [Collectors](#collectors).

    // ── Compiler Passes ───────────────────────────────────────────

    // Compiler passes run after the built-in resolve phase, in the order
    // their after/before constraints give. A pass is declared with its
    // handler, `c.pass("condition_check", |p| { p.after("resolve")
    // .run(pass_condition_check); })`: the SDK routes `__pass_<name>` to
    // it, decoding the PassInput snapshot and encoding the diagnostics
    // (see [Compiler passes](#compiler-passes)).

    fn pass_condition_check(input: &PassInput) -> Vec<PassDiagnostic> {
        PassDiagnostic::warning(
            "W096",
            "behavior 'x' declares requires but no ensures",
        )
        .with_suggestion("add an ensures clause")
        .into_iter()
        .collect()
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
| `version` | no | Semantic version (default: the crate's `CARGO_PKG_VERSION`) |
| `short` | no | The name the extension's commands are routed by: `specforge <short> <command>` on the CLI, `specforge.<short>.<id>` over MCP. Lowercase kebab case (`[a-z][a-z0-9-]*`), checked at compile time; absent, the name's last segment (`@specforge/software` is `software`). On the wire, the handshake's `ext_short`. |
| `description` | no | One line a package registry shows for the extension (the handshake's `description`) |

Everything else the handshake carries is set on the builder: `ContributionsBuilder::starter_template`, `migration_hook` and `theme_color`, and `ExtensionMeta`'s `peer_dependencies`, `sandbox_policy` and `keywords`. The declaration the builder builds (`ContributionsBuilder::declaration`) is exactly what the host loads and what `specforge publish` uploads (ADR 0012).

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
| `c.rule(...)` with `r.validate(...)` (builder) | Custom rule + `validate__*` export | `validation_rules` |
| `#[cli_command]` | `cmd__*` export | `surfaces` |
| `#[mcp_tool]` | `mcp__*` export | `surfaces` |
| `#[mcp_resource]` | `mcp__*` export | `surfaces` |
| `c.collector(...)` with `k.collect(...)` (builder) | declared command + `collect__*` export | `collectors` |
| `c.pass(...)` with `p.run(...)` (builder) | Pass descriptor + `__pass_*` export | `passes` |
| `c.analyzer(...)` with `a.scan(...)` (builder) | Analyzer descriptor + `scan__*` export | `analyzers` |
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
| `has_body_parser` | no | Whether this kind's body syntax is its own: parse errors inside its entities are not reported (default: `false`) |

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
| `bool` | `bool` |
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

### Custom rules

A `check: "custom"` rule is declared with the function that decides it
([ADR 0013](adr/0013-typed-extension-calls.md)). The SDK routes the rule's
`wasm_function` export (`validate__<code lowercased>` unless set) to it,
decoding the protocol's `ValidatorContext` and encoding its
`ValidatorVerdict`:

```rust
c.rule("W009", |r| {
    r.check(CheckKind::Custom)
        .target_kind("behavior")
        .message_template("{kind} '{id}' has verify kind '{value}' not in allowed set")
        .validate(|context: &ValidatorContext| {
            // Inspect context.entity, context.referenced, context.declared_types.
            ValidatorVerdict::Pass
        });
});
```

A custom rule without `validate` panics when the extension is built, so a rule
that would never fire cannot ship.

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

### Collectors

A test-runner extension declares a collector with `c.collector(name, ...)`
in `contribute` ([ADR 0002](adr/0002-test-runner-extensions.md)):

```rust
c.collector("cargo-test", |k| {
    k.input_format("specforge-test-json")
        .detect_files(&["Cargo.toml"])            // project-root files that select it
        .run(&["cargo", "test", "--workspace"])  // `{report}` expands to the report path
        .report("target/specforge")               // file or directory, inside the project
        .capture_stdout()                         // also pass the command's stdout
        .collect(|input: &CollectInput| Ok(collect(input))); // the handler
});
```

`specforge collect` runs the declared command in the project root, after
the user approves it once for the project, with `SPECFORGE_REPORT` set to
the absolute report path. It then reads the report (the file, or every
`*.json` file directly inside the directory) and calls the extension's
`collect__<name>` export (`-` becomes `_`), which the SDK routes to the
handler declared with `k.collect`, a pure function from [`CollectInput`] to
[`CollectOutput`] (or the reason it could not read the report): test results
grouped by entity, with `status` `passed`,
`failed` or `skipped`, plus the tests the report doesn't link
(`unlinked`), which the host links by naming convention when it can. With `capture_stdout()`, the command's output still
reaches the user's terminal, and the host also keeps it (in
`<name>.stdout.txt` inside a report directory) and passes it as
`CollectInput::stdout`. The guest never runs anything itself.

### Compiler passes

A pass is declared in `contribute` with `c.pass(name, |p| ...)`, whose
builder sets `after`, `before` and `phase`, and with the function that runs
it, `p.run(...)`: it receives the [`PassInput`] snapshot and answers its
diagnostics, bare (`Vec<PassDiagnostic>`) or with a summary
([`PassOutput`], whose `summary` keys join the pass's report). The SDK routes
the `__pass_<name>` export to it. A pass without `run` panics when the
extension is built.

```rust
c.pass("coverage", |p| {
    p.after("resolve").phase("check").run(|input: &PassInput| {
        input.entities.iter()
            .filter(|e| e.testable && !e.exempt && e.verify_texts.is_empty())
            .map(|e| PassDiagnostic::warning("A001", format!("{} declares nothing", e.id))
                .with_entity(&e.id))
            .collect::<Vec<_>>()
    });
});
```

`#[compiler_pass]` is deprecated: the function it wraps is already the
handler `p.run` takes; the attribute still generates the
`specforge_dispatch_pass_<name>` helper for a guest that routes the export
through `component_guest!`'s `handler`. A pass declared with `p.phase("check")` runs with every
compile (`specforge check`, watch, the LSP and MCP), after the graph checks:
its diagnostics are the compile's, with the codes and severities it returns,
and `specforge check` exits 1 on its errors. Check passes run in their
after/before order; a trap or malformed answer is E028. Any other phase, or
none, runs the pass only under `specforge analyze`, which skips check passes.

A pass receives `PassInput`: `entities` (each with its fields, edge counts,
span, `testable`, and `exempt`: it owes no obligations of its own, decided by
the host), `edges`, `test_results` and `proved_claims` (always absent for a
check pass), and `previous`: when
`specforge-cache.json` exists, the statuses of the build that wrote it
(`previous.statuses["<id>"].kind` / `.status`), else `None`. Give a
diagnostic the entity it is about with `PassDiagnostic::with_entity(id)`: with
no span of its own, the host attaches that entity's.

### Scanners and the migration hook

An analyzer is declared with its scanner, `a.scan(...)`, from a
[`ScanRequest`] (one source file) to a [`ScanResponse`] (its public items),
at its `scan_export` (`scan__<language>`). Its `classify__`/`map__` exports
are declared too but the host does not call them; a guest that serves them
anyway answers them from its `handler`, decoding with `answer_export`.

The hook `specforge migrate` calls after it migrates the project's files is
declared with its handler, `c.migration_hook_handler(export, |input:
&MigrationInput| ...)`: it receives the format versions and the migrated
files; its `Err` fails the migration, which is rolled back.

### #[feature_flag]

Declares a feature flag configurable via `specforge.json`.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Flag name (used as key in `specforge.json`) |
| `values` | yes | Allowed values |
| `default` | yes | Default value (must be in `values`) |

## Host Functions

**Status: not callable from guests today.** The old `HostApi` import surface
was removed with the extism runtime (see the banner at the top of this page).
Guests are pure-compute wasip2 components: the host passes everything a guest
needs as call input (e.g. the `ValidatorContext` snapshot for `validate__*`
exports, the entity snapshot for `__pass_*`), and the guest's return value is
the only channel back.

A typed component host-import surface is future work. When it lands, guests
will import the functions below, each allowed only from the call sites listed
(the contracts are specified in `spec/behaviors/wasm-host-functions.spec`):

| Function | Purpose | Allowed call sites |
|----------|---------|--------------------|
| `host_emit_diagnostic` | Emit a diagnostic to the host's collection | all |
| `host_read_file` | Read a file from the project (sandbox-checked) | Validator, Provider, Parser, Analyzer |
| `host_emit_file` | Write renderer/collector output (sandbox-checked) | Renderer, Collector |
| `host_http_get` | Fetch from an allowlisted domain | Provider |
| `host_query_graph` | Scope-limited query of the entity graph (replaces the phantom `query`/`resolve_ref` pair) | all |
| `host_add_graph_node` | Add a graph node | Parser |
| `host_add_graph_edge` | Add a graph edge | Parser |

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
cargo build --target wasm32-wasip2 --release
```

The output `.wasm` file is at `target/wasm32-wasip2/release/specforge_ext_software.wasm`.

### Test

Extensions can be tested with standard `cargo test` (native target) for logic, and with the SDK's test harness for protocol conformance:

```rust
#[cfg(test)]
mod tests {
    use specforge_extension_sdk::test::*;

    #[test]
    fn handshake_returns_valid_metadata() {
        let ext = TestExtension::load("target/wasm32-wasip2/release/specforge_ext_software.wasm");
        let metadata = ext.handshake("1.0.0");
        assert_eq!(metadata.name, "@specforge/software");
        assert!(metadata.contribution_flags.entities);
    }

    #[test]
    fn describe_entities_returns_behavior() {
        let ext = TestExtension::load("target/wasm32-wasip2/release/specforge_ext_software.wasm");
        let entities = ext.describe("entities");
        assert!(entities.iter().any(|e| e.keyword == "behavior"));
    }
}
```

A host-side test (a test of the host, or of how a host reads an extension)
serves the extension in process: `specforge_wasm::testing::InProcessRuntime`
(feature `testing`) runs an SDK `ContributionsBuilder` through the guest's
own routing (`guest_call`, what `component_guest!` calls), so a test declares
the extension with the same builders, loads it with `load_declaration` and
calls it with `ExtensionCalls` exactly as the host calls a component.
Answers no SDK guest gives (a trap, bytes that do not parse) are given with
`answer_raw`. It runs the guest unsandboxed, in the host process; sandbox
and deadline behaviour is only proven through the component runtime.

```rust
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::protocol::load_declaration;

let runtime = InProcessRuntime::new().with(my_extension_build);
let declaration = load_declaration(&runtime, "@acme/widgets").unwrap().declaration;
```

### Install

Copy the `.wasm` file to the extension directory and register it:

```bash
specforge add ./extensions/software/target/wasm32-wasip2/release/specforge_ext_software.wasm
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

    // Running Cucumber and mapping its results would be a separate runner
    // extension with a collector; see [Collectors](#collectors).
}
```

## Related Documentation

- [Extension Protocol](extension-protocol.md) -- the wire protocol this SDK implements
- [Extension Inventory](extension-inventory.md) -- the five official extensions
- [Extension Model](extension-model.md) -- the broader extension architecture (extensions, providers, renderers)
- [Entity Model](entity-model.md) -- entity kinds, edge types, and validation rules

> 📖 New to extension authoring? Follow the step-by-step tutorial:
> **[Extending SpecForge — Zero to Production](guides/extending-specforge.md)**.

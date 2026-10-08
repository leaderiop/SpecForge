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
// peers: meta.peer_dependencies (see Peer dependencies)
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

`ExtensionMeta::sandbox_policy` declares the extension's limits, `SandboxPolicy { max_execution_ms, max_memory_mb }`: at most 30000 ms and 512 MB, the ceiling when unset. A component is granted no capability whatever it declares (see Sandbox in the [protocol doc](extension-protocol.md)).

Peer dependencies go on the extension's meta:

```rust
c.meta.peer_dependencies.push(PeerDependency {
    name: "@specforge/product".to_string(),
    version: "^1.0".to_string(), // a SemVer requirement; anything else is E073
    optional: false,
});
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
| `c.command(...)` with `cmd.arg(...)` and `cmd.handler(...)` (builder) | `cmd__*` export | `surfaces` |
| `c.mcp_tool(...)` with `t.handler(...)` (builder) | `mcp__*` export | `surfaces` |
| `c.mcp_resource(...)` with `r.handler(...)` (builder) | `mcp__*` export | `surfaces` |
| `c.collector(...)` with `k.collect(...)` (builder) | declared command + `collect__*` export | `collectors` |
| `c.pass(...)` with `p.run(...)` (builder) | Pass descriptor + `__pass_*` export | `passes` |
| `c.analyzer(...)` with `a.scan(...)` (builder) | Analyzer descriptor + `scan__*` export | `analyzers` |
| `#[feature_flag]` | Flag descriptor | `feature_flags` |
| `meta.peer_dependencies` | Dependency declaration | handshake |
| `#[lsp]` | LSP metadata on entity kind | `entities` |
| `#[dot]` | DOT visualization metadata | `entities` |
| `cmd.arg(...)` (builder) | Command argument descriptor | `surfaces` |
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
that would never fire cannot ship. Each `context.entity.fields` entry's
`value` is the field's text, always a string ([Field text](#field-text)).

### Commands and their args

`c.command(id, |cmd| ...)` declares a CLI command with the function that answers it: the SDK
generates its `cmd__*` export (`cmd__<prefix>_<id>` under `command_prefix`) and its surface
descriptor, and routes the export to the handler. The CLI runs it as `specforge <short> <id with _ as
->`, MCP as the tool `specforge.<short>.<id>`.

| Builder call | Description |
|-----------|-------------|
| `cmd.title(..)` | The one-line title help lists it under |
| `cmd.description(..)` | Its long help and its MCP tool's description |
| `cmd.category(..)` | A CLI grouping |
| `cmd.arg(name, \|a\| ..)` | Declares an arg (below) |
| `cmd.handler(\|call\| ..)` | The function answering it: `CommandCall` in, `CommandOutput` out |

An arg is a string unless its builder says otherwise:

| Arg builder | On the wire | Read with |
|-----------|------------------|-----------|
| `a.string()` (default) | `"string"` | `call.str(name)` |
| `a.path()` | `"path"` | `call.str(name)` |
| `a.flag()` | `"bool"`: `--name`, `false` unless set | `call.flag(name)` |
| `a.integer()` | `"integer"` | `call.integer(name)` |
| `a.count()` | `"integer"` with `"minimum": 0` | `call.count(name)` |
| `a.one_of(&[..])` | `{"enum": {"values": [..]}}` | `call.str(name)` |
| `a.required()` | `"required": true`: positional on the command line | |
| `a.default_value(..)` | `"default_value"`: what an absent arg is, on every surface | |
| `a.description(..)` | its help and its MCP property's description | |

The host and the SDK read args by one rule (`specforge_protocol_types::command_args`, ADR 0017): both
surfaces send the export the args normalized (declared defaults applied, an unset flag `false`, each
value its declared type), and refuse a missing required arg, a value of another type or below its
minimum, or an undeclared arg before the export runs, as one `INVALID_INPUT` object; `CommandCall`
runs the same rule on what it receives. The builder panics on a declaration the host would refuse (an
arg named `path`, `format` or `help`, two args spelling one option such as `all_kinds` and `all-kinds`,
a flag or required arg with a default, a default its type refuses) and on a required flag or an empty
`one_of`, so the extension's first test finds it.

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

`#[compiler_pass]` is gone (it was deprecated): the function it wrapped is the handler `p.run`
takes, so `#[compiler_pass(name = "x")] fn f(…)` becomes `c.pass("x", |p| { p.run(f); })`. A pass declared with `p.phase("check")` runs with every
compile (`specforge check`, watch, the LSP and MCP), after the graph checks:
its diagnostics are the compile's, with the codes and severities it returns,
and `specforge check` exits 1 on its errors. Check passes run in their
after/before order; a trap or malformed answer is E028. Any other phase, or
none, runs the pass only under `specforge analyze`, which skips check passes.

A pass receives `PassInput`: `entities` (each with its field texts by name
([Field text](#field-text)), edge counts, span, `testable`, and `exempt`: it
owes no obligations of its own, decided by the host's one obligation rule), `edges`, `test_results` and `proved_claims` (always absent for a
check pass), and `previous`: when
`specforge-cache.json` exists, the statuses of the build that wrote it
(`previous.statuses["<id>"].kind` / `.status`), else `None`. Give a
diagnostic the entity it is about with `PassDiagnostic::with_entity(id)`: with
no span of its own, the host attaches that entity's.

### Field text

Every field an entity writes has exactly one text, the same for a declarative
rule (what `field_value_constraint` matches and `{value}` interpolates), a
custom rule (`ValidatorField.value`, always a JSON string) and a compiler pass
(`PassEntity.fields`) ([ADR 0019](adr/0019-one-entity-snapshot-per-compile.md)):

| Field value | Text |
|---|---|
| string, identifier, date | as written |
| integer, boolean | its literal (`42`, `true`) |
| list of strings or references | items joined by `", "` |
| mixed list (`[1, true]`) | items' texts joined by `", "` |
| variant list (`values [low, high]`), type union (`string \| string[]`) | members joined by `" \| "` |
| expression group (`expr { a < 10ms, b > 5 }`) | each expression's display form, joined by `", "` |
| verify statements | their texts joined by `"; "` |
| block (`ensures { done "…" }`) | its keys joined by `", "` |

A written empty list or block is present with the text `""`, so
`missing_required_field` does not fire on it and `non_empty` does. No written
field is left out and no value is `null`. A name written twice: a pass and a
declarative rule see its last text; a custom rule's `fields` lists every entry
in order.

A joined list cannot be split back when an item itself contains the joiner:
`["a, b", "c"]` is `"a, b, c"`. A rule that must tell items apart needs a
structured value, which would come beside the text (ADR 0019, "What would
reopen this").

This is protocol `1.1.0`; under `1.0.0` a validator received `null` for variant
lists, mixed lists, expressions and type unions, and a pass did not receive
them or empty variant lists and blocks. See the version table in
[extension-protocol.md](extension-protocol.md#protocol-versions).

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

An extension is tested natively (`cargo test`, no wasm build) at three depths:

- **Its logic**: plain unit tests of the functions its handlers call.
- **Its declaration**: `specforge_extension_sdk::testing::MockHost` pins the handshake and describe
  wire JSON (`assert_handshake`, `assert_describe`), and `testing::call_every_command` runs every
  declared command with every arg set, which catches a handler reading an arg its command does not
  declare (a panic here, E028 in the host).
- **Its commands, as the host calls them**: `specforge_wasm::testing::InProcessRuntime` (feature
  `testing`) serves the extension's `ContributionsBuilder` through the guest's own routing
  (`Served`, what `component_guest!` calls), and `ExtensionCalls::run_command` calls a command
  exactly as the CLI and MCP do: the typed `CommandInput` (the graph as the host renders it), the
  strictly decoded `CommandOutput`, a failure as E028.

```toml
[dev-dependencies]
specforge-wasm = { version = "0.1", features = ["testing"] }
specforge-protocol-types = "0.1"
specforge-test = "0.1"   # to link a test to the obligation it proves
```

```rust
use specforge_protocol_types::{CommandEvidence, CommandFormat, CommandInput, RawGraph};
use specforge_test::prelude::*;
use specforge_wasm::{ExtensionCalls, testing::InProcessRuntime};

#[specforge_test(behavior = "count_widgets", verify = "an empty graph has no widgets")]
fn an_empty_graph_has_no_widgets() {
    let runtime = InProcessRuntime::new().with(crate::specforge_extension_build);
    let input = CommandInput {
        args: serde_json::Map::new(),
        cwd: "/p".into(),
        format: CommandFormat::Json,
        today: "2026-10-08".into(),
        graph: RawGraph::new(r#"{"nodes":[],"edges":[]}"#.into()).unwrap(),
        evidence: CommandEvidence::None,
    };
    let out = ExtensionCalls::new(&runtime)
        .run_command("@acme/widgets", "cmd__widgets_count", &input)
        .unwrap();
    assert_eq!((out.exit_code, out.stdout.as_str()), (0, "{\"count\":0}"));
}
```

`@specforge/product`'s `extensions/product/src/tests/host.rs` is a complete harness of this kind.
The in-process runtime runs the guest unsandboxed, in the test's process: the sandbox, the deadline
and the component's stack are proven only through the component runtime (`specforge-component`'s
tests of the vendored blob). A host-side test (of the host, or of how it reads an extension) uses
the same runtime and loads the declaration with `specforge_wasm::protocol::load_declaration`;
answers no SDK guest gives (a trap, bytes that do not parse) are given with `answer_raw`.

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
// peers: meta.peer_dependencies (see Peer dependencies)
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

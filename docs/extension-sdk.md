# Extension SDK

> The SDK is `crates/runtime/extension-sdk`: a `ContributionsBuilder` you fill in a `Contributions` impl, the
> `#[extension]` attribute that names the extension, and `component_guest!`, which serves it as a `wasm32-wasip2`
> component exporting the `specforge:bridge` world. The working example is `fixtures/greet-extension/`, built in
> CI and pinned by `crates/runtime/component/tests/greet_sdk.rs`.
>
> `call(name, export-name, input) -> result<list<u8>, string>` dispatches `__handshake` / `__describe` from the
> builder, every declared surface and operation to the handler declared with it, and any other export name to the
> guest's `handler`. Host functions (the old `HostApi`) were removed with the extism runtime; a typed component
> host-import surface is future work.

## Overview

An extension is how SpecForge learns a domain: it declares the entity kinds a project may write, their fields
and links, the rules they must obey, and the commands and MCP tools that answer questions about them. The
compiler itself knows none of these; the builtin extensions (`@specforge/software`, `product`, `governance`,
`formal`, `testing`, the runner and language extensions) are written with this same SDK. For the method of
designing a domain before writing one, see [Model your domain](guides/model-your-domain.md).

An extension is a Rust crate compiled to `wasm32-wasip2`, depending on `specforge-extension-sdk` alone (`component_guest!` brings its own bindings).
The SDK provides:

- **Protocol types** -- re-exported from `specforge-protocol-types`, shared with the host
- **One attribute**, `#[extension]`, which names the extension
- **Builders** that declare every contribution with the handler that answers it
- **`component_guest!`**, which serves the declaration as a wasip2 component
- **`testing`** -- a runtime-free mock host
- **Host functions (planned)** -- the typed import surface is future work; guests today are pure-compute and receive all context as call input (see [Host Functions](#host-functions))

There are no hand-written JSON manifests and no export registration: the declaration the builder builds is what the
host loads and what `specforge publish` uploads (ADR 0012).

## Extension Structure

An extension is a standalone Rust crate with its own `Cargo.toml`, anywhere on disk; `specforge extension init`
scaffolds one:

```
my-ext/
  Cargo.toml          -- cdylib, depends on specforge-extension-sdk
  .cargo/config.toml  -- pins wasm32-wasip2
  src/
    lib.rs            -- the Contributions impl and component_guest!
```

The builtin extensions live in this repository's `extensions/` directory, one crate each: `software`, `product`,
`governance`, `formal`, `testing`, `rust`, `typescript`, `cargo-test` and `vitest`. Each keeps its declaration in
`src/` (the software extension's starter file is `src/starter.spec`) and its committed build in `wasm/`.

### Cargo.toml

```toml
[package]
name = "specforge-ext-software"
version = "1.0.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
specforge-extension-sdk = "0.3.0"
```

That is an outside author's crate. `specforge extension init` writes this dependency line, in a crate that
is a workspace of its own (`[workspace]`), so it builds inside another cargo workspace. A builtin in
`extensions/` names the SDK by path instead:

```toml
specforge-extension-sdk = { path = "../../crates/runtime/extension-sdk" }
```

### Building against an unreleased SDK

A specforge built after its last release names an SDK version crates.io does not have yet, and
`specforge extension build` says so (E040). Build against the SDK of the checkout that specforge was built from,
in a `.cargo/config.toml` beside the crate or in any directory above it:

```toml
[patch.crates-io]
specforge-extension-sdk = { path = "/path/to/SpecForge/crates/runtime/extension-sdk" }
```

The crate's `Cargo.toml` stays as `extension init` wrote it, so it builds against crates.io unchanged once that
version is released. The patch applies only to the version the crate names; this is how the repository's own test
builds a scaffold.

### Compile Target

Extensions compile to `wasm32-wasip2`:

```bash
cargo build --target wasm32-wasip2 --release
```

The output `.wasm` file is what the host loads at runtime. `specforge extension init` scaffolds a
`[profile.release]` (opt-level `z`, fat LTO, one codegen unit, strip) that makes it about half the default size;
stripping drops function names from trap frames, so remove the table to debug.

## Authoring Experience

You describe what the extension contributes with the builder, each contribution with the function that answers it. The SDK serves `__handshake` and `__describe` from the declaration and routes every declared export (`cmd__*`, `mcp__*`, `__pass_*`, `collect__*`, `validate__*`, `scan__*`, the migration hook) to its handler.

### Complete Example

This is the `greet` fixture (`fixtures/greet-extension/src/`), built in CI:

```rust
//! What the greet extension declares, authored entirely with the SpecForge
//! extension SDK. Kept apart from the component glue (`lib.rs`) so the
//! host's tests can serve the same declarations in process
//! (`crates/runtime/component/tests/greet_sdk.rs` includes this file) and
//! check both runtimes answer alike.

use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@sdk/greet",
    version = "0.1.0",
    short = "greet",
    description = "Friendly greetings"
)]
pub struct Greet;

/// The greeting styles.
const STYLES: Vocabulary = Vocabulary::new(&["warm", "formal"]);

impl Contributions for Greet {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("greeting", |k| {
            k.description("A friendly greeting").testable(false);
            k.field("style", |f| {
                f.vocabulary(&STYLES).required();
            });
        });
        c.rule("E901", |r| {
            r.check(CheckKind::FieldValueConstraint);
            r.target_kind("greeting");
            r.field("style");
            r.constraint(|fc| {
                fc.kind(ConstraintKind::Matches);
                fc.pattern("^(warm|formal)$");
            });
            r.severity(ValidationSeverity::Error);
            r.message_template("greeting '{id}' has unknown style");
        });
        c.code("E901", |d| {
            d.title("Unknown greeting style")
                .explanation("A greeting's style is `warm` or `formal`: set it to one of them.");
        });
        c.command("hello", |cmd| {
            cmd.title("Say hello")
                .description("Greet someone, warmly")
                .arg("name", |a| {
                    a.string().required().description("Who to greet");
                })
                .handler(|call| {
                    let name = call.str("name").unwrap_or_default();
                    let greeting = format!("Hello, {name}!");
                    call.render(&serde_json::json!({ "greeting": greeting }), |out| {
                        out.push_str(&greeting);
                        out.push('\n');
                    })
                });
        });
        c.pass("styles", |p| {
            p.run(|input: &PassInput| {
                input
                    .entities
                    .iter()
                    .filter(|e| e.kind == "greeting")
                    .map(|e| {
                        let style = e.fields.get("style").map_or("none", String::as_str);
                        PassDiagnostic::new(
                            "G900",
                            PassSeverity::Info,
                            format!("greeting '{}' is {style}", e.id),
                        )
                        .with_entity(&e.id)
                    })
                    .collect::<Vec<_>>()
            });
        });
        c.collector("greet-test", |k| {
            k.input_format("greet-lines")
                .report("greet-report.txt")
                .collect(|input: &CollectInput| {
                    // One line per test: `<greeting id> <passed|failed|skipped>`.
                    let entity_results = input
                        .reports
                        .iter()
                        .flat_map(|report| {
                            report
                                .content
                                .lines()
                                .filter_map(|line| line.split_once(' '))
                                .map(move |(id, status)| (report, id, status.trim()))
                        })
                        .map(|(report, id, status)| {
                            let status = serde_json::from_value::<TestStatus>(
                                serde_json::Value::String(status.to_string()),
                            )
                            .map_err(|_| {
                                format!(
                                    "{}: '{status}' is not passed, failed or skipped",
                                    report.path
                                )
                            })?;
                            Ok(CollectEntityResult {
                                entity_id: id.to_string(),
                                test_results: vec![TestResult {
                                    name: format!("greets_{id}"),
                                    status,
                                    verify: None,
                                    duration_ms: None,
                                }],
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(CollectOutput {
                        entity_results,
                        unlinked: Vec::new(),
                        plan: None,
                    })
                });
        });
    }
}

/// The extension's contributions, as its guest builds them per call.
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}
```

`lib.rs` serves it:

```rust
mod contributions;

specforge_extension_sdk::component_guest!(build = contributions::build);
```

`#[extension]` generates `specforge_extension_build()`; `build` hands it to `component_guest!`. An extension that also serves exports with no builder (its own scanners' helpers, say) passes `handler = <fn(&str, &[u8]) -> Option<Result<Vec<u8>, String>>>`.

### Extension-Level Attributes

`#[extension]` is the one attribute the SDK has. It names the extension; the contributions are declared on the builder in `Contributions::contribute`.

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name` | yes | Scoped package name (e.g., `@specforge/software`) |
| `version` | no | Semantic version (default: the crate's `CARGO_PKG_VERSION`) |
| `short` | no | The name the extension's commands are routed by: `specforge <short> <command>` on the CLI, `specforge.<short>.<id>` over MCP. Lowercase kebab case (`[a-z][a-z0-9-]*`), checked at compile time; absent, the name's last segment (`@specforge/software` is `software`). On the wire, the handshake's `ext_short`. |
| `description` | no | One line a package registry shows for the extension (the handshake's `description`) |

Everything else the handshake carries is set on the builder: `c.starter_template(..)`, `c.theme_color(..)`, `c.migration_hook_handler(export, |input| ..)`, `c.command_prefix(..)` and `c.providers()` (it implements configured providers), `c.peer(..)` and `c.optional_peer(..)` (a peer requirement), and `ExtensionMeta`'s `c.meta.keywords` and `c.meta.sandbox_policy`. The declaration the builder builds (`ContributionsBuilder::declaration`) is exactly what the host loads and what `specforge publish` uploads (ADR 0012).

`ExtensionMeta::sandbox_policy` declares the extension's limits, `SandboxPolicy { max_execution_ms, max_memory_mb }`: at most 30000 ms and 512 MB, the ceiling when unset. A component is granted no capability whatever it declares (see Sandbox in the [protocol doc](extension-protocol.md)).

A peer requirement is a builder call, `c.peer(name, range)`, or `c.optional_peer(name, range)` for a peer the extension works without:

```rust
c.peer("@specforge/product", "^1.0"); // a SemVer requirement; anything else is E073
```

`specforge add` and `init` enable a required peer that is a builtin along with your extension; a required peer that is not installed, or an installed one at a version your range does not accept (a builtin's included), refuses the install with E027, as `specforge check` would report it.

## Contribution Surface Reference

Every contribution is a builder call. The SDK adds the matching Wasm exports and metadata descriptors.

| Builder call | Declares | Category |
|---|---|---|
| `c.kind(name, \|k\| ..)` | an entity kind; `k.field(..)` its fields | `entities` |
| `c.edge(label, \|e\| ..)` | an edge type (`source_kind`, `target_kind`, DOT style) | `edges` |
| `c.shared_field(name, \|f\| ..)` | a field every kind of the extension has | `shared_fields` |
| `c.enhance(kind, \|e\| ..)` | fields and verify kinds on another extension's kind | `enhancements` |
| `c.rule(code, \|r\| ..)` | a declarative rule, or a custom one with `r.validate(..)` | `validation_rules` |
| `c.command(id, \|cmd\| ..)` | a CLI command (also an MCP tool) and its handler | `surfaces` |
| `c.mcp_tool(..)`, `c.mcp_resource(..)` | an MCP tool or resource and its handler | `surfaces` |
| `c.collector(name, \|k\| ..)` | a test collector and its handler | `collectors` |
| `c.pass(name, \|p\| ..)` | a compiler pass and its handler | `passes` |
| `c.analyzer(language, \|a\| ..)` | a source analyzer and its scanner | `analyzers` |
| `c.feature_flag(name, default, description)` | a bool setting of that name users set in `specforge.json` | `settings` |
| `c.setting(name, \|s\| ..)` | a typed setting: `bool()`, `integer()`, `string()`, `enum_values(..)`, `string_list()`, `block()`, `block_list()`, `default(..)`, `describe(..)`, `gates(..)`, `keys(..)` for a family | `settings` |
| `c.preset(source, \|p\| ..)` | what each value of an enum or bool setting presets and raises: `p.when(value, \|e\| e.set(..).raise(..))` | `settings` |
| `c.secret(name, env, description)` | a secret read from an environment variable | `settings` |
| `c.code(code, \|d\| ..)` | a diagnostic code the extension reports: `title(..)`, `explanation(..)`, `docs(..)` (an `http(s)` URL) | `codes` |
| `c.peer(name, range)`, `c.optional_peer(name, range)` | a peer requirement | handshake |
| `c.providers()` | it implements configured providers | handshake |

## Builder reference

### Kinds

`c.kind(name, |k| ..)` declares a DSL keyword the core grammar parses. `KindBuilder` methods:

- `description(..)`, `keyword(..)` (the DSL keyword, default the name)
- `testable(bool)`, `singleton(bool)`, `supports_verify(bool)`, `incremental(bool)`
- `open_fields(bool)` -- unknown fields are accepted
- `has_body_parser()` -- declares that the kind's body carries syntax of its own; the core grammar reads every body and reports its parse errors whatever the flag says
- `contract_target()`, `declares_types()`, `scoped_by(field)`, `verify_kinds(&[..])`, `inference_guide(..)`
- LSP and DOT metadata, requested by the host only where it needs them: `semantic_token(..)`, `lsp_icon(..)`, `dot_shape(..)`, `dot_color(..)`, `dot_fillcolor(..)`
- `field(name, |f| ..)` -- a field of the kind

### Fields

`FieldBuilder` declares a field: `field_type(FieldType::..)`, then `required()`, `description(..)`, `edge(label)`, `target_kind(..)`, `inverse_of(..)` (the name the target kind reads the relation by; never a field of the target kind, and never two kinds linking each other: both are W162), `normative()`, `exempts_obligations()`, `headline()`, `proof_role(..)`, `derived_from(..)`, `file_reference()` (string values are file paths) and `file_base(FileBase::Project)` (relative paths start at the project root, not the spec root), `target_kinds(&[..])` (a reference names any of several kinds), `minimum(n)` and `maximum(n)` (an integer's inclusive bounds), `member(name, |m| ..)` (a field of a `block` or `block_list`, declared as a field is) and `enum_list(&V)` (a list of a vocabulary's values, each once). Who writes the field is declared too (ADR 0287): `computed()` (a compute pass gives the value; one written by hand is E082), `harness()` (a command's plan sets it and the host records it in `specforge-harness`; E083 without a line), `immutable()` (a value that differs from its git baseline is E084) and `append_only()` (a list that rewrites its baseline's items is E085; a kind's `append_only()` makes removing an entity E085). A pass declares `compute()` and `computes(&["kind.field"])` to answer `PassOutput.values` (`PassValue { entity, field, value }`), and `baseline(&["kind"])` for the kinds it reads as `previous`. `vocabulary(&V)` declares the field an `enum` of a `Vocabulary` (`const V: Vocabulary = Vocabulary::new(&[..]).absent_as(..)`): its values, in order, and the value an absent field counts as; the host refuses another value (E061). It is the only way to declare an enum. A field's type is declared, not inferred:

| `FieldType` | Holds |
|---|---|
| `String` | a quoted string |
| `Integer` | a whole number |
| `Bool` | `true` or `false` |
| `Date` | one calendar day, `YYYY-MM-DD`, written bare |
| `Enum` | one of its vocabulary's values (declared with `vocabulary`) |
| `Identifier` | one word from an open set, written bare |
| `StringList` | a list of quoted strings |
| `Reference` | one entity id; creates an edge |
| `ReferenceList` | a list of entity ids; creates one edge per id |
| `EnumList` | a list of its vocabulary's values, each once (declared with `enum_list`) |
| `Block` | a block of entries; with `member(..)`, of typed members |
| `BlockList` | a list of blocks with the same `member(..)`s |

### Vocabularies

A closed set of values a field takes is declared once, as a `const`:

```rust
const STATUS: Vocabulary = Vocabulary::new(&["draft", "live", "retired"]).absent_as("draft");
const LIVE: &str = STATUS.member("live");
```

- `Vocabulary::new(values)`: the values in order; empty, an empty value or a repeat is a compile error in a `const`.
- `absent_as(value)`: what an absent field counts as; it becomes the field's `default_value`. A non-member is a compile error.
- `values()` and `absent()`: read both; `values()` is what an arg's `one_of(..)` takes.
- `member(value)` and `index(value)`: name a value (a misspelt one does not compile) and its place in the order.
- `position(value)`, `contains(value)` and `read(written)`: a sort's rank, membership, and a field read as its written value or else its absent one.

One `const` per vocabulary; the field declaration, the list filters, the passes and the rules that name a value all read it. `field_type(FieldType::Enum)` panics: an enum is declared with `vocabulary`.

### Edges

`c.edge(label, |e| ..)` with `EdgeBuilder`: `description(..)`, `source_kind(..)`, `target_kind(..)`, `edge_style(..)` (`solid`, `dashed`, `dotted`), `edge_color(..)` (hex), `edge_arrowhead(..)`. A field with `edge(label)` and `target_kind` creates the edge; `c.edge` declares one with no field, which the model draws only when both kinds are given.

### Declarative rules

`c.rule(code, |r| ..)` with `RuleBuilder`: `check(CheckKind::..)`, `target_kind(..)`, `edge_type(..)`, `field(..)`, `severity(ValidationSeverity::..)`, `message_template(..)` (placeholders `{id}`, `{kind}`, `{value}`, `{allowed}`: a `one_of` constraint's values), `target_extension(..)` and `constraint(|fc| fc.kind(ConstraintKind::..).pattern(..).values(&[..]))`. The checks:

| `CheckKind` | Reads |
|---|---|
| `NoIncomingEdges`, `NoOutgoingEdges`, `NoEdges` | the entity's edges (optionally only `edge_type` edges); the outgoing ones also read a written reference that does not resolve as a link (ADR 0232) |
| `MissingFieldWhenFlagSet`, `MissingRequiredField` | `field` (required) |
| `FieldValueConstraint` | `field` and a `constraint`: `NonEmpty`, `OneOf` with `values`, or `Matches` with a `pattern` |
| `CycleDetection` | `edge_type` (required) |
| `FileExists` | the path in `field` (required) |
| `Custom` | the rule's own function (below) |
| `ConditionalFieldRequired` | `field`, and a `WhenFieldEquals` constraint |
| `VerifyKindAllowlist` | the `constraint` values |
| `NoVerifyStatements` | the entity's `verify` statements |

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
minimum, or an undeclared arg before the export runs, as one `INVALID_INPUT` object on the command line, `invalid_input` naming the arg over MCP; `CommandCall`
runs the same rule on what it receives. The builder panics on a declaration the host would refuse (an
arg named `path`, `format` or `help`, two args spelling one option such as `all_kinds` and `all-kinds`,
a flag or required arg with a default, a default its type refuses) and on a required flag or an empty
`one_of`, so the extension's first test finds it.

An MCP tool's `input_schema` may not declare `path`, as a property or a required name: `path` names
the project, the host's call target reads it from every MCP call and takes it out before the tool's
export sees it (`specforge_protocol_types::tool_input`, ADR 0239). The builder panics on such a tool,
as on a command arg named `path`, so the extension's first test finds it.

### Writing commands and collectors

An extension never writes files. A command declares `cmd.writes()` and answers an edit plan the host checks, judges
and applies all or nothing ([the wire](extension-protocol.md#writing), ADR 0289):

```rust
c.command("record", |cmd| {
    cmd.writes()                                   // the host adds --dry-run; no arg may take that name
        .arg("phase", |a| a.required())
        .handler(|call| {
            let mut plan = call.plan();            // each op takes its file's revision from this call
            let phase = call.str("phase").unwrap_or_default();
            plan.title("Record a phase")
                .set(phase, "status", "done")
                .append(phase, "checks", json!({"status": "pass"}));
            Ok(CommandOutput::ok("").with_plan(plan.finish()))
        });
});
```

A judged rule is declared with `r.judged("judge", |j| { j.outcomes(..).passing(..); j.target("kind", |t| { t.input("name", "selector"); }) })` inside `c.rule(..)`
and a field a verdict sets with `field.verdict()` (ADR 0290, [guide](guides/judged-rules.md)); a command's `GraphNode`
lists its `judgements`, and `set_verdict` takes the `basis` the command read through `specforge query --inputs`.

`PlanBuilder` has one method per op: `create`, `set`, `unset`, `append`, `remove`, `link`, `unlink`, `retitle`,
`add_verify`, `remove_verify`, `delete`, `rename` and `set_verdict(Verdict { .. })`. A collector declares `k.writes()`
and builds with `PlanBuilder::for_collect(input)`, answering `CollectOutput.plan`. If the files changed since the
call read them, the host refuses the plan `stale_revision` and writes nothing; the caller runs the command again. The
plan is the call's whole write: a handler that also touches the disk itself is outside the host's checks, its lock
and its journal.

Who may write is declared on the vocabulary: `human()` on a kind or a field (only a person writes it; an extension's
plan and the agent surfaces get `human_only`), `harness()` and `computed()` as above, and `guard()` on a pass (it
runs when a write is judged, reads the project before the write as `previous`, and its error diagnostics refuse the
write, E080).

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
([`TestResult`]) grouped by entity, each with a
[`TestStatus`] (`passed`, `failed` or `skipped`; any other value cannot be built, and a handler that meets one
in a report answers its `Err`), plus the tests the report doesn't link
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
            .filter(|e| e.testable && !e.owes_none && e.verify_texts.is_empty())
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
([Field text](#field-text)), edge counts, span, `testable`, and `owes_none`: it
owes no obligations of its own, decided by the host's one obligation rule), `edges`, `test_results` (the recorded test report as [`TestResults`]; `TestResults::of(id)` lists an entity's tests) and `proved_claims` (always absent for a
check pass), and `previous`: when the pass names kinds in
`.baseline(&[..])`, the project at the git commit `HEAD` names, for those kinds
(`previous.revision`, `previous.entities["<id>"].kind` / `.fields["<field>"]`;
references are their targets' full paths and computed values are included),
else `None`. Give a
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
| list (`["a", b]`, `[1, true]`) | items' texts joined by `", "` |
| variant list, type union (`string \| string[]`) | members joined by `" \| "` |
| expression group (`expr { a < 10ms, b > 5 }`) | each expression's display form, joined by `", "` |
| block (`ensures { done "…" }`) | its keys joined by `", "` |

A written empty list or block is present with the text `""`, so
`missing_required_field` does not fire on it and `non_empty` does. No written
field is left out and no value is `null`. A name written twice in one block
is E081: every rule, pass and custom validator sees its first entry, once.

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

The hook `specforge migrate` calls before it writes the migrated files is
declared with its handler, `c.migration_hook_handler(export, |input:
&MigrationInput| ...)`: it receives the format versions and the names of the
files the migration will rewrite; its `Err` refuses the migration and nothing
is written.

### Settings

A setting is a typed value a user sets in `specforge.json` under the extension's name (ADR 0288):

```rust
c.setting("tdd_mode", |s| { s.bool().default(false).describe("Require a test first").gates(&["T201"]); });
c.setting("models.*", |s| { s.enum_values(&["opus", "sonnet"]).keys(&[&["planning", "execution"]]); });
c.preset("profile", |p| { p.when("strict", |e| { e.set("tdd_mode", true).raise("T202", ValidationSeverity::Error); }); });
c.secret("api_key", "MY_API_KEY", "The service key");
```

A dotted name or a family (`*` segments) has a type, an optional default and the codes it gates: while a bool setting is false, those codes are reported by no source. A preset maps each value of an enum or bool setting (the extension's own, or a peer's as `@ext:name`) to settings it sets and codes it raises, never lowers. `c.feature_flag(name, default, description)` is a bool setting. A secret is never written in `specforge.json` (E087); the extension is told only whether its variable is set.

A handler reads them with `Settings::current()`: `bool(name)`, `string(name)`, `integer(name)`, `get(name)`, `under(prefix)` for a family's keys and `secret_set(name)` (in a command, a tool or a resource; a pass or validator always reads `false`, as its result must not depend on the machine). A call without a value gets the declared default. A tool's arguments never hold them. `specforge settings` lists what a project resolves them to and `specforge schema --config` writes the project's `specforge.json` schema with them typed.

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

## Building and Testing

### Build

```bash
cd my-ext
cargo build --release          # .cargo/config.toml pins --target wasm32-wasip2
```

The output `.wasm` file is at `target/wasm32-wasip2/release/specforge_ext_software.wasm`.

`specforge extension validate` reports the protocol version the built component speaks (`protocol` under `--format json`).

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
specforge-extension-sdk = { version = "0.3.0", features = ["testing"] }   # MockHost, call_every_command
```

`specforge-wasm`'s `InProcessRuntime` and `specforge-test` are not published: the builtins in this
repository take them by path (`extensions/product/Cargo.toml`); an extension outside it tests its
declaration with `MockHost` and `call_every_command`, and its commands through
`specforge extension build` and `validate`. The in-repository harness looks like this:

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

`specforge add` copies the component into the project and registers it in `specforge.json`. Name the crate
directory (its `target/wasm32-wasip2/release` component) or the `.wasm` file:

```bash
specforge add ./my-ext
specforge add ./my-ext/target/wasm32-wasip2/release/my_ext.wasm
```

A builtin is added by name, and a published extension by name and version from a configured registry
([registry trust](registry-trust.md)):

```bash
specforge add @specforge/product
specforge add @you/my-ext@0.1.0
```

## Enhancement-Only Extensions

Extensions that contribute no entity kinds of their own -- only enhancements to other extensions' entity kinds -- follow the same structure and declare zero kinds:

```rust
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(name = "@acme/gherkin", short = "gherkin")]
struct Gherkin;

impl Contributions for Gherkin {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@specforge/software", "^1.0");
        c.enhance("behavior", |e| {
            e.field("gherkin", |f| {
                f.field_type(FieldType::StringList)
                    .description("BDD scenario files")
                    .file_reference();
            });
        });
        c.rule("W904", |r| {
            r.check(CheckKind::FileExists)
                .target_kind("behavior")
                .field("gherkin")
                .severity(ValidationSeverity::Warning)
                .message_template("behavior '{id}' names a gherkin file that does not exist");
        });
    }
}
```

An enhancement's fields are your extension's. If the kind's extension may be absent, declare it with `c.optional_peer(..)`: the host then skips the enhancement silently instead of reporting I004.

Running Cucumber and mapping its results would be a separate runner extension with a collector; see [Collectors](#collectors).

## Related Documentation

- [Extension Protocol](extension-protocol.md) -- the wire protocol this SDK implements
- [Extension Inventory](extension-inventory.md) -- the builtin extensions
- [Extension Model](extension-model.md) -- the broader extension architecture (extensions, providers, renderers)
- [Entity Model](entity-model.md) -- entity kinds, edge types, and validation rules

> 📖 New to extension authoring? Follow the step-by-step tutorial:
> **[Extending SpecForge — Zero to Production](guides/extending-specforge.md)**.

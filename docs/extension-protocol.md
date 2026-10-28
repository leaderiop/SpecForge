# Extension Protocol

The Extension Protocol is how an extension teaches SpecForge a domain. SpecForge's compiler knows no domain: the grammar has five structural keywords (`spec`, `ref`, `use`, `defaults` and the reserved `define`), and every other kind a project writes — the builtin `behavior` and `feature` as much as a third party's `procedure` or `campaign` — enters through this protocol. An extension is a Wasm component; it declares itself, and the host reads that declaration from the binary, never from a file beside it.

Most authors never see the wire: the [extension SDK](extension-sdk.md) builds the declaration and serves every call from Rust ([tutorial](guides/extending-specforge.md)). This page is the reference underneath, for the host's side and for a guest written in another language.

## Overview

Extensions are Wasm components that the host loads, interrogates, and invokes at runtime. The protocol has three phases: **handshake**, **describe** and **operate**; the host may drop and reload an extension at any time (see [Reloading extensions](#reloading-extensions)). The host drives all interactions. Extensions never call the host: they answer the call they are given, with the input it carries.

This design upholds Principle 2 (the compiler knows nothing about your domain) by ensuring that all domain vocabulary enters the compiler through a single, uniform protocol. The host does not know what an extension will contribute until it asks.

```
Host                                Extension (.wasm)
 |                                        |
 |-- load wasm binary ------------------>|
 |                                        |
 |-- __handshake(host_version) --------->|
 |<-- name, version, protocol_version, --|
 |    peer_dependencies, sandbox_policy   |
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
 |   ... every declared category, once    |
 |                                        |
 |-- (register contributions) ---------->|  (internal)
 |                                        |
 |-- cmd__validate(args) --------------->|  (on demand)
 |<-- result -----------------------------|
 |                                        |
 |-- (reload: specforge.json changed) -->|  (from the top)
```

## Discovery Protocol

### Handshake

The handshake is the first call the host makes after loading a Wasm binary. It establishes identity, compatibility and the extension's metadata.

**Export:** `__handshake`

**Input:**

```json
{ "host_version": "1.10.0" }
```

**Output** (`specforge_protocol_types::HandshakeResponse`):

```json
{
  "protocol_version": "1.10.0",
  "name": "@specforge/software",
  "version": "1.0.0",
  "peer_dependencies": [
    { "name": "@specforge/product", "version": "^1.0", "optional": true }
  ],
  "sandbox_policy": { "max_memory_mb": 256, "max_execution_ms": 5000 },
  "starter_template": "spec \"{project}\" {\n  version \"{version}\"\n}\n",
  "theme_color": "#4a90d9",
  "ext_short": "software",
  "description": "Software design: behaviors, events, types and ports, and the checks that keep them consistent",
  "keywords": ["design", "contracts"]
}
```

Peer dependencies are SemVer ranges read as Cargo reads them; one rule judges them against the loaded versions (E027; a range that is not SemVer is E073, ADR 0041). Extensions load in `specforge.json` order, except that each loads after its peers. A cycle among **required** peers is one E027 naming its extensions. **Optional** peers are only a preference: extensions may name each other as optional peers, and the host adds the optional edges after the required ones in name order, skipping any that would close a cycle. A rule on another extension's kind that this one works without uses the rule's `target_extension` (see "Category: validation_rules"), not a peer.

`protocol_version`, `name`, `version`, `peer_dependencies` and `sandbox_policy` are required on the wire (`sandbox_policy` may be `null`: the extension then runs under the host's ceiling; see Sandbox). The others are optional and omitted when absent:

- `starter_template`: the text of the starter `.spec` file `specforge init` writes for a project that enables the extension, `{project}` standing for the project's entity id and `{version}` for its version, each filled in escaped as the inside of a `.spec` string (write them between quotes). When several enabled extensions declare one, `init` uses the first listed in `specforge.json`. SDK: `ContributionsBuilder::starter_template`.
- `migration_hook`: the export `specforge migrate` calls before it writes the migrated files, with their names. SDK: `ContributionsBuilder::migration_hook`.
- `theme_color`: the hex colour (`#rgb`, `#rrggbb` or `#rrggbbaa`) the `model` and `outline` diagrams draw the extension in; grey otherwise. SDK: `ContributionsBuilder::theme_color`.
- `ext_short`: the short name that routes the extension's commands, `specforge <ext_short> <command>` on the CLI and `specforge.<ext_short>.<id>` over MCP. Lowercase kebab case (`[a-z][a-z0-9-]*`); a malformed one is E030. Absent, it is the name's last segment (`@specforge/product` is `product`). SDK: `#[extension(short = "...")]`, checked at compile time.
- `description` and `keywords`: what a package registry shows for the extension. `specforge publish` uploads them with the rest of the declaration. SDK: `#[extension(description = "...")]`, `ExtensionMeta::keywords`.
- `providers`: `true` when the extension implements configured providers (a `specforge.json` `providers` entry may name it; W118 otherwise). SDK: `ContributionsBuilder::providers`.

What an extension contributes is what it declares in the describe categories; nothing on the handshake says it (ADR 0050).

The host checks `protocol_version`: a major version other than its own fails the extension's load (E028), and its handshake's `sandbox_policy` limits every later call into it (see Sandbox).

### Describe

After the handshake, the host calls `__describe(category)` once for each category it reads, always, in this order: `entities`, `edges`, `shared_fields`, `enhancements`, `validation_rules`, `surfaces`, `collectors`, `analyzers`, `passes`, `feature_flags`, `codes`, `settings` (`specforge_protocol_types::DECLARED_CATEGORIES`). Together with the handshake they are the extension's **declaration** (`specforge_protocol_types::ExtensionDeclaration`, ADR 0012): the host loads it once per environment load (`specforge_wasm::protocol::load_declaration`) and nothing describes a category again. A guest whose handshake `protocol_version` is older than 1.7 is not asked for `codes`, and one older than 1.8 is not asked for `settings` (the host compares `(major, minor)` as numbers); each loads with none. An extension answers every supported category, `[]` when it contributes nothing to it; the SDK does.

A category whose answer fails or doesn't parse as its descriptors fails the extension's load (E028), naming the category. An item key a descriptor doesn't define is ignored and reported (W138), naming the extension, the category, the item and the key.

The host reads every string and object key of a handshake or describe answer printable (ADR 0244): each control and bidirectional character is written as its escape (`\u{1b}`), never stripped, so no declared title, description, template or key acts on a terminal. Two answers are read as sent: a command's `stdout`/`stderr` and an MCP tool's or resource's answer, which are the extension's own output. The `starter_template` keeps its newlines and tabs.

**Export:** `__describe`

**Input:**

```json
{
  "category": "entities"
}
```

**Output:**

```json
{
  "category": "entities",
  "items": [ ... ]
}
```

`items` is an array of the category's descriptors.

### Describe Categories

| Category | What it returns | Read by the host |
|----------|----------------|--------------------------|
| `entities` | Entity kind descriptors (keyword, fields, LSP and DOT metadata) | Always |
| `edges` | Edge type descriptors (label, source/target kind, visual style) | Always |
| `shared_fields` | Field descriptors every kind of the extension gets (a kind's own field of the same name wins) | Always |
| `enhancements` | Fields and verify kinds added to other extensions' entity kinds | Always |
| `validation_rules` | Declarative and custom validation rule descriptors | Always |
| `surfaces` | One descriptor of the CLI commands, MCP tools and MCP resources, or none | Always |
| `collectors` | Test result collector descriptors | Always |
| `analyzers` | Language analyzer descriptors (`specforge infer`) | Always |
| `passes` | Compiler pass descriptors with ordering constraints | Always |
| `feature_flags` | Feature flag descriptors, each read as a bool setting (below) | Always |
| `codes` | The extension's diagnostic code catalog (protocol 1.7.0) | Always, from a guest of 1.7 or later |
| `settings` | Typed settings, presets and secrets (protocol 1.8.0) | Always, from a guest of 1.8 or later |

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

A kind may add `"scoped_by": "<field>"` (protocol 1.7.0, ADR 0284): the name of one optional `reference` field with a target kind. The target kind then opens a scope, an entity's name is unique within its scope, and its id is its scope's path and its name (`v1.0/03/D-01`). A pass's `PassEntity.id` and edge ends are such paths, so a pass that keys its own tables by id keeps working unchanged.

Optional flags, `false` and omitted from the wire unless set (ADR 0007): on
a kind, `contract_target` (a reference field that targets the kind is a
contract obligation, A010) and `declares_types` (its entity ids are the type
names custom validators receive as `declared_types`); on a field, `normative`
(the review document quotes it whole), `headline` (the context export lifts it, as its field text, to the
node's top level; with `normative`, it is the statement MCP `inspect` reports
as `contract`) and `exempts_obligations` (an entity that sets it owes no
`verify` obligations).

Field types (`specforge_protocol_types::FieldType`): `string`, `integer`,
`bool`, `date` (one calendar day, `YYYY-MM-DD`, written as a string), `enum` (values in `enum_values`), `enum_list` (a list of those values, each once), `identifier` (one word from an open set, written bare), `string_list`, `reference`,
`reference_list`, `block` and `block_list` (protocol 1.7.0, ADR 0285). The host also reads the `_type`-suffixed
spellings (`string_type`, ...) and `boolean`; an unknown type drops the
field with W019. Every host output names a field type by these names — the
Graph Protocol schema and exports, the published JSON Schema, the model,
hover and E061; the older spellings are read, never written (ADR 0034).

What a field takes and what is written for it (ADR 0064): `string` takes text, a bare word, a date, a type
union or an `expr { }` group, and is written as a string (a group as its conjuncts, one per line); `enum` one of
its `enum_values`; `identifier` one word (letters, digits and `_`, starting with a letter or `_`), written as a string; `integer` and `bool` their JSON scalars; `date` one calendar day, `YYYY-MM-DD`, written as a string (a quoted day is read as the day); `reference` one id, written as a string;
`string_list` and `reference_list` a list (a single value is stored as a one-item list; a list is stored as written), written as an
array of strings; `enum_list` a list of its `enum_values` (a word twice is E061), written as an array of strings; `block` a block, written as an object of its entries, and, when the field declares `members`, an object of those members; `block_list` a list of blocks, `[ { phase 02 provides "x" } { .. } ]` (commas optional, `specforge format` writes none), written as an array of objects. Anything else is E061.

A field of 1.7.0 may also declare: `members`, the fields of a `block` or of each `block_list` item (field descriptors themselves, nested to any depth; a member may not declare `derived_from`); `target_kinds`, the kinds a `reference` or `reference_list` may name (a union, exclusive with `target_kind`; a scope field names one kind); `minimum` and `maximum`, an `integer`'s inclusive bounds (E061 outside them); and `file_base`, `spec` (the default) or `project`, where a `file_reference` field's relative paths start. A member is addressed by its declared path, `requires.phase`; the edge a reference member writes is labelled by it. A pass's `PassEntity.values` and a validator's `ValidatorEntity.values` carry a field that declares members as structured JSON, keyed by field name, while `fields` keeps its text.
A field of 1.8.0 may also declare who writes it (ADR 0287): `writer`, `"computed"` (a compute pass gives the value; a value written by hand is E082) or `"harness"` (a command's plan sets it and the host records it in `specforge-harness`; a value with no line there is E083); `immutable` (a value that differs from its git baseline is E084; unset at the baseline, it may be set once); and `append_only` (a list that rewrites its baseline's items is E085). A kind may declare `append_only` too: an entity of it that the baseline holds and the project does not is E085. A field nested under `members` declares the same. A `computed` or `harness` field is never completed or offered by an editor, and a computed value is shown marked `computed`. `lifecycle_field` on a kind is read and ignored. A field, a block member or a kind of 1.9.0 may declare `writer: "human"` (ADR 0289): only a person writes it, and an extension's plan, the MCP tools and a non-interactive CLI are refused `human_only`; a `required` field of a kind that is not human-only is W021, since an agent could then create no entity.

How a value is written in `.spec` follows the type (ADR 0147): `enum`, `identifier`, `reference` and each `reference_list`
item bare; `string` and each `string_list` item quoted. `specforge format` writes it and `check` warns W159 on the other form. A command's
`graph` input, the exports and MCP carry exactly these forms.

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

### Category: shared_fields

Returns the field descriptors every entity kind of the extension gets; a kind's own field of the same name wins.

```json
{
  "category": "shared_fields",
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

A rule's code is described in the `codes` category, not on the rule.

The host credits the fields to the answering extension and reads the kind's owner from its kind registry (ADR 0096).

Check kinds (`specforge_protocol_types::CheckKind`). The descriptor stays flat
and string-typed on the wire; the host's registry build turns each rule into a
typed rule (`specforge_registry::rules`, ADR 0020) and checks its shape against
this table. A rule missing what its check **requires** (or with an unknown
check) is **W112** and is not registered. A property its check does **not read**
is **W147**: the rule is registered without it. `field` is never W147: every
check's message reads it as the default `{field}`, and its text as the default
`{value}`. `{allowed}` is the values of a `one_of` constraint (on
`field_value_constraint` or `verify_kind_allowlist`), joined by `, `; any other
check leaves it as written.

A rule of `check: "judged"` (1.9.0, ADR 0290) holds while a fresh `verdict` of its `judge` says so, instead of computing
anything. Its `judged` descriptor is `{judge, outcomes, passing, targets: [{kind, inputs: [{name, selector}], unless_set}],
values, outcome_fields}`; selectors are `self`, `Kind`, `.field`, `<Kind.field`, `file(..)` and `test(..)`
([guide](guides/judged-rules.md)). The host evaluates it after the graph build and before the compute passes: a
`PassEntity` then carries its `judgements` (`{rule, state, passed, outcome?, at?, changed?, files?}`, gated rules
absent), a graph node and a baseline entity (`PassBaselineEntity`) the same, and a `V` field (`writer: "verdict"`)
takes the outcome or a `values` member of a fresh verdict. `verdict <rule> <outcome> { judge model at cited
cited_tests inputs findings values }` is a member of an entity's block, written through `set_verdict`; W168 is a
verdict that cannot be read, I205 test inputs not judged, E082 a `V` field written by hand and E083 a verdict with no
ledger line.

| Check | Fires when | Requires (W112 when missing) | Reads | W147 when set |
|-------|------------|------------------------------|-------|---------------|
| `no_incoming_edges` | no edge points at the entity (only edges of `edge_type`, from its source kind, when set) | — | `edge_type` | `constraint`, `wasm_function` |
| `no_outgoing_edges` | the entity points at nothing: no field writes a reference (resolved or not; one that does not resolve is E003, reported alone) and no edge leaves it (only `edge_type`'s fields and edges, to its target kind, when set) | — | `edge_type` | `constraint`, `wasm_function` |
| `no_edges` | no edge points at the entity and it points at nothing (as `no_outgoing_edges`) | — | — | `edge_type`, `constraint`, `wasm_function` |
| `missing_field_when_flag_set` | `field` is absent (when `field` is `verify`, the entity's `verify` statements; an entity owing none is exempt) | `field` | `field` | `edge_type`, `constraint`, `wasm_function` |
| `missing_required_field` | `field` is absent | `field` | `field` | `edge_type`, `constraint`, `wasm_function` |
| `file_exists` | a path `field` names (its text, or each item of a list; a block or an expression names none) does not exist, relative to the spec root; the diagnostic suggests a close sibling | `field` | `field` | `edge_type`, `constraint`, `wasm_function` |
| `field_value_constraint` | `field`'s value breaks the constraint | `field`; a constraint `non_empty`, `one_of` with values, or `matches` with a `pattern` that compiles | the constraint | `edge_type`, `wasm_function`; `pattern` on `non_empty`/`one_of`; `values` on `non_empty`/`matches` |
| `conditional_field_required` | `field` is absent or empty while the field named by `constraint.pattern` holds one of `constraint.values` | `field`; `constraint.pattern` and non-empty `constraint.values` | constraint kind `when_field_equals` | `edge_type`, `wasm_function`; any other constraint kind (read as `when_field_equals`) |
| `cycle_detection` | the entity sits on a cycle of `edge_type` edges, following every field that writes it | `edge_type` | `target_kind` (unset: every entity) | `constraint`, `wasm_function` |
| `verify_kind_allowlist` | a `verify` kind is not in `constraint.values` | a constraint with non-empty `values`; a target kind that accepts `verify` | constraint kind `one_of` | `edge_type`, `wasm_function`; `pattern`; any other constraint kind (read as `one_of`) |
| `no_verify_statements` | a testable entity declares no `verify` obligations (or does not write `field` when it names another obligation field) | a target kind that accepts `verify` (for `verify` statements) | `field` (default `verify`) | `edge_type`, `constraint`, `wasm_function` |
| `custom` | the rule's `wasm_function` export answers `fail` | `wasm_function` | — | `edge_type`, `constraint` |

Older spellings earlier SDK releases emitted (`missing_field`,
`field_constraint`, `cycle`, `conditional_required`) are still read.

Constraint kinds (`specforge_protocol_types::ConstraintKind`): `non_empty`,
`one_of` and `matches` (regex in `pattern`) for `field_value_constraint`;
`when_field_equals` (condition field in `pattern`, triggering values in
`values`) for `conditional_field_required`; `one_of` for
`verify_kind_allowlist`. Any other kind on a `field_value_constraint` rule
drops the rule with W112.

References resolve against the loaded registries. A rule whose target kind or
edge type no loaded extension declares reports nothing (it belongs to an
extension that is not installed); an edge type resolves through the edge
registry only, never as a field name. A target kind or edge type that neither
the extension, its declared peers nor the rule's `target_extension` declare is
**W021** (anything goes while a named peer is not loaded).

`target_extension` (optional, protocol `1.1.0`) names the extension whose kind
or edge type a rule is about when that is neither the declaring extension nor
one of its declared peers: a rule on another extension's kind that this one
works without. No peer dependency is needed (a peer orders extension loading
and pins a version range). While the named extension is not loaded, that rule
alone is inert and costs no W021; loaded but without the kind or edge type, it
is W021. With no `target_extension`, a kind only a non-peer extension declares
is W021 suggesting it. A `custom` rule's function is probed once at load
(W112 when it cannot answer); a function that fails on real entities during a
check is **W148**, once per rule, its data listing every entity that was not
checked.

### Category: surfaces

Returns CLI command, MCP tool, and MCP resource descriptors. CLI commands auto-promote to MCP tools. Each surface declares its arguments and export name.

```json
{
  "category": "surfaces",
  "items": [{
    "commands": [
      {
        "id": "validate",
        "title": "Run validation",
        "description": "Run product validation rules",
        "category": "analysis",
        "export": "cmd__validate",
        "args": [
          { "name": "target", "arg_type": "string", "required": true, "description": "The entity to validate" },
          { "name": "profile", "arg_type": { "enum": { "values": ["default", "strict"] } }, "default_value": "default", "description": "Lint profile" },
          { "name": "limit", "arg_type": "integer", "minimum": 0, "description": "Report at most this many" },
          { "name": "details", "arg_type": "bool", "description": "Show each finding" }
        ]
      }
    ],
    "mcp_tools": [
      {
        "name": "acme.model",
        "description": "Generate entity model",
        "category": "visualization",
        "export": "mcp__acme_model",
        "input_schema": { "type": "object", "properties": { "format": { "type": "string" } } }
      }
    ],
    "mcp_resources": [
      {
        "uri_template": "specforge://ext/acme/{kind}",
        "name": "entity_list",
        "description": "List entities by kind",
        "mime_type": "application/json",
        "export": "mcp__entity_list"
      }
    ]
  }]
}
```

A command arg declares `name`, `arg_type` (`"string"`, `"path"`, `"bool"`, `"integer"`, or
`{"enum": {"values": [..]}}`), and optionally `required`, `default_value` (a string, read as the arg's
type), `description` and `minimum` (the least value of an integer arg: `0` for a count; a host or guest
that does not know the field ignores it). The host reads the declaration by one rule, the SDK's too
(`specforge_protocol_types::command_args`, ADR 0017):

- **Refused declarations.** A command with an arg named `path`, `format` or `help` (the host's own
  options), two args whose names spell one option (`all_kinds`, `all-kinds`), a flag or a required arg
  with a default, or a default its type refuses, runs on no surface: the CLI refuses it (exit 2) and MCP
  serves no tool for it (I017). The SDK refuses to build such an extension.
- **Refused MCP tools.** An explicit MCP tool whose `input_schema` declares `path` (a property or a
  required name) is not served (I017): `path` is the call target's, taken out of every call before the
  tool's export or schema check sees it (ADR 0239). The host and the SDK read one rule
  (`specforge_protocol_types::tool_input`): the SDK refuses to build such an extension.
- **Normalized args.** Both surfaces send a `cmd__` export the same args for the same input: every
  declared arg the caller set, as its type (an integer or a flag may come as the string a command line
  gives); an absent arg its default; an unset flag `false` (a flag is never required). A missing
  required arg, a value of another type or below the minimum, or an undeclared argument is refused
  before the export runs, as one `{"code": "INVALID_INPUT", "message", "suggestion"?}` object on the
  command line and as `invalid_input` naming the arg over MCP.
- **The MCP tool** of a command is `specforge.<short>.<id>`; its `inputSchema` states each arg's type,
  values, minimum, default and description, `required` lists the required args that are not flags, and
  `additionalProperties` is `false`.

- **Gates (1.10.0, ADR 0291).** A command may declare `gates`, local codes of its extension, and `subject`, the
  name of one of its args (a `reference`'s entity scopes the gates; absent, the project). Before the export runs
  the host evaluates the gate in the subject's scope: a gated code fails while the project holds an error of that
  code on the subject, or on an entity the subject scopes. A code a bool setting gates off (`gates` on the
  setting) never fails the gate. The call is refused `gate_failed` (kind `precondition_failed`) with
  `data.failing`, the errors, and `data.gates`, each code's `{code, off, passes}`; on the command line it is
  `error[gate_failed]` and exit 1, over MCP the tool's error. A refusal writes nothing and calls no export. A guard
  pass that declares `reports: ["E903"]` (the codes it can answer) runs at a gate too, and receives
  `PassInput.trigger`, `{"kind": "gate", "command", "subject"}`; absent, the pass reads a write. A pass that only
  checks a write answers nothing at a gate.

MCP serves each tool name and resource URI once: the core ones, then explicit tools and resources in
extension load order, then the commands. A contribution not served under its name (an explicit tool
named as a core tool, a resource whose URIs a core resource serves, a command whose tool name is taken)
is reported with I017. A resource template may use any scheme; its read failing (a trap, an answer that
is not `{content, mime_type}`) is a JSON-RPC internal error whose `data` is an McpError carrying E028.

### Body syntax

Extensions cannot contribute grammars or body parsers (ADR 0004, D5-a, ADR 0050). A kind whose
body carries syntax of its own may set `has_body_parser` on its entity descriptor; the flag changes
nothing in the compiler: the core grammar reads every body and every parse error in it is reported.

**Prompts (1.10.0, ADR 0291).** A `surfaces` item may declare `prompts`: workflows an agent follows, served as MCP
prompts `specforge.<short>.<id>`, as `specforge prompt <short>.<id>` and as `specforge <short> <id>` (an id's `_` is
spelled `-`) when the extension has no command of that name. A prompt is routed under the same `(short name, name)` as
a command, commands first; a prompt whose name is taken is E039 and runs on no surface.

```json
{
  "prompts": [{
    "id": "plan_phase",
    "title": "Plan a phase",
    "description": "Plan the phase, then check the plans",
    "export": "prompt__plan_phase",
    "args": [
      { "name": "phase", "field_type": "reference", "target_kind": "phase", "required": true },
      { "name": "depth", "field_type": "integer", "minimum": 1, "maximum": 3 },
      { "name": "mode", "field_type": "enum", "enum_values": ["fast", "slow"] }
    ],
    "gates": ["E901"], "subject": "phase",
    "steps": [
      { "id": "plan", "title": "Write the plans", "command": "record_plan" },
      { "id": "check", "title": "Check the plans" }
    ],
    "when": "workflow.research",
    "reads_graph": false,
    "attribution": { "upstream": "gsd-core", "path": "workflows/plan-phase.md", "version": "1.18.0",
                     "license": "MIT", "copyright": "Copyright (c) 2026 Open GSD" }
  }]
}
```

- **Arguments** are typed like fields: `field_type` is `string`, `integer` (with `minimum` and `maximum`), `bool`,
  `date`, `enum` (with `enum_values`), `string_list`, `enum_list`, `reference` or `reference_list` (with
  `target_kind`, or kinds as `a|b`); never `path`. A caller's value is read by its type (a word `2` is an integer,
  `a,b` a list) and admitted by the form every field of that type is admitted by; a reference is resolved by
  `Graph::find` (a short form is accepted) and the export is sent its full path. An argument named `path`,
  `format`, `help`, `settings`, `secrets`, `host` or `argv` is the host's, and a name is snake_case. Defaults fill an absent argument; an absent
  optional argument (or one given as `null` or `""`) is absent; on a command line a double-quoted run is one word and a
  quoted `--x` is a value, `--name=value` and `--` (the rest are positional) are read, and an unclosed quote is refused; a bool is not defaulted to `false`.
- **Entry gate.** `gates` and `subject` are a command's: the prompt is refused `gate_failed` before its export runs
  while a gated code fails in the subject's scope.
- **Steps** are the order the agent works in. A step runs a `command` of this extension (or `<peer>:<id>` of a
  declared peer), an `agent`, a sub-`prompt`, or none (the agent does it). The host renders a command step as its MCP
  tool and CLI line with the state of its gate now. A prompt is not a contract: an agent may skip a step, and only a
  command's own gate refuses.
- **`when`** names a bool setting of the extension; while it is false the prompt is neither listed nor served.
- **`reads_graph`** sends the export the graph, as a command's. Absent, a workflow reads through tools.
- **Rendering.** The `prompt__` export receives `PromptInput` and answers at least one message. The host appends one
  message of its own: the prompt as `@ext@version:id`, a `sha256:` digest of the export's message texts, the
  attribution, the subject's full path and each step with its tool, command line and gate. `specforge.prompt` returns
  the export's messages and that message as `payload`.
- **Attribution** says where ported text came from (`upstream`, `path`, `version`, an SPDX `license`, `copyright`); it
  is rendered with the prompt.
- **Refused declarations.** A prompt or a gated command the registry cannot serve as declared is dropped whole, with
  one W021 naming it and the first reason (an unknown gated code, a `subject` that is no reference argument of the
  command or prompt, an argument of a type a prompt does not take, a reserved or repeated name, a `when` that is no
  bool setting, a step that names nothing, a default its type refuses). A step's `agent` and an empty attribution are
  dropped alone. An older host ignores `prompts`, `gates` and `subject` (W138 for the keys), so a gated command runs
  ungated there.

### Category: collectors

Returns the test-result collectors a runner extension contributes
([ADR 0002](adr/0002-test-runner-extensions.md)): the project-root files that
select it (`auto_detect.file_patterns`, last segment may use `*`), the command
`specforge collect` runs with the user's consent (`run`; `{report}` expands to
the absolute report path, also exported as `SPECFORGE_REPORT`), where the
report lands (`report`, a file or directory inside the project; default
`.specforge/reports/<name>.json`), whether the host also keeps the command's
standard output (`capture: "stdout"`, for runners whose results only appear
there), and the pure export that maps the report to entities.

```json
{
  "category": "collectors",
  "items": [
    {
      "name": "cargo-test",
      "input_formats": ["specforge-test-json"],
      "export": "collect__cargo_test",
      "auto_detect": { "file_patterns": ["Cargo.toml"], "env_vars": [] },
      "run": ["cargo", "test", "--workspace", "--no-fail-fast"],
      "report": "target/specforge",
      "capture": "stdout"
    }
  ]
}
```

The host calls the export with a `CollectInput`, `{"reports": [{"path",
"content"}], "stdout"?}` (`stdout` only when the collector captures it), and
reads a `CollectOutput`, `{"entity_results": [{"entity_id", "test_results":
[{"name", "status", "verify"?, "duration_ms"?}]}], "unlinked"?: [{"name",
"path", "status"}]}` (each test a `TestResult`), with `status` a `TestStatus`:
`passed`, `failed` or `skipped`, the one vocabulary of the collect wire, the
recorded test report and a pass's test results. `entity_results`, each test's
`name` and its `status` are required, and a status outside the three is not a
`CollectOutput`: an answer without them is E028 naming the collector, never
empty results or a skipped test.
`unlinked` lists tests the report doesn't link to an entity (`path` is the
test's name split into segments, its own name last); the host links them by
naming convention when it can (`entity_id__obligation_slug`, or a module
named after an entity).

### Category: passes

Returns compiler pass descriptors. Each pass declares ordering constraints relative to the built-in resolve phase and other passes.

`phase: "check"` makes the pass part of every compile: it runs after the
graph checks, and its diagnostics join the compile's (`specforge check`,
watch, the LSP, MCP). Passes with any other phase, or none, run only under
`specforge analyze`. The `__pass_<name>` export receives a `PassInput`
(`{"entities", "edges", "test_results"?, "proved_claims"?, "previous"?}`;
`test_results` is the recorded test report as `TestResults`, `{"runner"?,
"results": {"<id>": {"tests": [TestResult]}}}`;
each entity is `{"id", "kind", "fields", "incoming_edge_count",
"outgoing_edge_count", "span"?, "testable", "owes_none", "verify_kinds",
"verify_texts"}`, `owes_none` meaning it owes no obligations of its own, decided
by the host from the registries; `incoming_edge_count` and `outgoing_edge_count`
count the edges from and to other entities, an edge from the entity to itself in
neither). An entity's `id`, and an edge's `source` and `target`, are entity paths: a scope's path and a name
(`v1.0/03/D-01`), or the name alone in the project scope; a name never holds `/`. The host re-keys
`test_results` by path: a key written as a path, a rooted path or a unique short form reaches its
entity (ADR 0284). A pass answers host diagnostics, bare or as
`{"diagnostics", "summary"}` (a `PassAnswer`). A diagnostic may carry
`"entity": "<id>"`; with no `span`, the host attaches that entity's.
A pass of 1.8.0 declares `baseline`, the kinds whose git baseline it reads as `previous`, and `computes`, the `kind.field` names it gives values for. `phase: "compute"` makes the pass run before the checks: its `PassOutput.values`, `[{"entity", "field", "value"}]`, are held beside the graph (never written to a file) and read by the rules, the check passes and every surface; a value for a field that is not declared `computed` is W165. A pass of 1.9.0 may run in `phase: "guard"`: it runs only when a write is judged, receives the project before the write as `previous`, and its error diagnostics refuse the write (E080); it answers no values.
`previous` (compute and check passes that name kinds in `baseline`) is the
git baseline of those kinds, `{"revision", "entities": {"<id>": {"kind",
"fields": {...}}}}`; it is absent with no baseline (not in a git repository, no
commit yet). See
[Operate](#operate) for every operation's input and answer.

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

Returns feature flag descriptors. Each flag declares allowed values and a default. The host reads a flag as the bool setting of its name and default ([Category: settings](#category-settings)), so `"settings": {"<extension>": {"<flag>": true}}` in `specforge.json` sets it.

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

### Category: settings

Returns typed settings, presets and secrets (protocol 1.8.0, ADR 0288). `specforge.json` sets a setting under `"settings": {"<extension>": {"<name>": value}}`; a feature flag is a bool setting. Each item is told apart by `kind`:

```json
{
  "category": "settings",
  "items": [
    {"kind": "setting", "name": "tdd_mode", "field_type": "bool", "default": false, "gates": ["T201"]},
    {"kind": "setting", "name": "models.*", "field_type": "enum", "enum_values": ["opus", "sonnet"], "keys": [["planning", "execution"]]},
    {"kind": "preset", "source": "profile", "values": {"strict": {"settings": {"tdd_mode": true}, "severities": {"T202": "error"}}}},
    {"kind": "secret", "name": "api_key", "env": "MY_API_KEY"}
  ]
}
```

A name is dotted, or a family with `*` segments. `field_type` is a field type of this document; `default` absent means unset. `gates` (a bool setting only) lists local codes reported by no source while it is false. A preset follows an enum or bool setting of the extension, or a peer's (`@ext:name`); each value sets `settings` and raises `severities`, never lowers one. A secret is read from the environment variable `env`, never from `specforge.json`: a guest is told only whether it is set. An item of a kind the host does not know costs that item (W021), not the category.

### Category: codes

Returns the extension's diagnostic code catalog (protocol 1.7.0, ADR 0286): one entry per code the extension's rules and passes report. `code` is the extension's own code, `[EWI]` and three digits, never qualified (the host adds the extension's namespace). `title` and `explanation` are required; `docs` (optional) is an absolute `http(s)` URL.

```json
{
  "category": "codes",
  "items": [
    {
      "code": "E901",
      "title": "Requirement not mapped",
      "explanation": "Every v1 requirement maps to exactly one phase.",
      "docs": "https://gsd.dev/codes#e101"
    }
  ]
}
```

SDK: `c.code("E901", |d| { d.title(..).explanation(..).docs(..); })`.

## Operate

Once the declaration is loaded, the host calls an extension's exports on
demand, each through one call of the bridge `call(name, export, input)` with a
JSON input, reading a JSON answer ([ADR 0013](adr/0013-typed-extension-calls.md)).
Every input and answer is a type of `specforge_protocol_types` (module
`calls`), the same definitions the SDK re-exports, so a Rust guest built with
the SDK cannot answer the wrong shape; a guest in another language reads the
table below and the goldens in `crates/runtime/wasm/tests/wire/`.

Every call but the handshake and describe also carries the extension's own `settings` (a JSON object of its effective values by name, a family's keys written out; omitted when it has none) and, to a command, an MCP tool, a resource or a prompt of an extension that declares a secret, `secrets` (each declared secret's name, `true` when its environment variable is set, never the value), added at the end of the input object. A prompt export that puts presence into its messages makes them (and the `digest` in its payload) differ between a laptop and CI, so a prompt keeps it out of its text. A pass, validator, collector, scanner or migration is not told (nor is a gate's guard pass): whether a variable is set is the machine's, so a result that depended on it would differ between CI and a laptop with nothing in the project to explain it. Another extension's settings are never sent, a client's `settings` or `secrets` argument of an MCP tool is dropped, and an explicit tool's input schema may not declare `settings` or `secrets` (I017). The SDK reads them with `Settings::current()`, which falls back to each declared default.

| Operation | Export | Input | Answer |
|---|---|---|---|
| Handshake | `__handshake` | `HandshakeRequest` `{"host_version"}` | `HandshakeResponse` |
| Describe | `__describe` | `DescribeRequest` `{"category"}` | `DescribeResponse` `{"category", "items"}` |
| Command | the command's `export` (`cmd__<id>`) | `CommandInput` `{"args", "cwd", "format", "today", "graph", "revisions"?}`; `cwd` is the project root, absolute and canonical | `CommandOutput` `{"exit_code", "stdout"?, "stderr"?, "plan"?}` |
| Prompt | the prompt's `export` (`prompt__<id>`) | `PromptInput` `{"args", "cwd", "today", "graph"?, "host"?}`; `args` are typed, a reference its full path; `host` is `claude-code`, `codex` or `cursor` when known | `PromptOutput` `{"messages": [{"role": "user" or "assistant", "text"}]}`, at least one |
| MCP tool | the tool's `export` (`mcp__<name>`) | the JSON its `input_schema` describes | the JSON its `output_schema` describes |
| MCP resource | the resource's `export` (`mcp__<name>`) | `McpResourceRequest` `{"uri"}` | `McpResourceContent` `{"content", "mime_type"}` |
| Compiler pass | `__pass_<name>` | `PassInput` | `PassAnswer`: `[PassDiagnostic]` or `{"diagnostics", "summary"?}` |
| Collector | the collector's `export` (`collect__<name>`) | `CollectInput` (a writing collector: `{"reports", "stdout"?, "cwd", "today", "graph", "revisions"}`) | `CollectOutput` (`{"entity_results", "unlinked"?, "plan"?}`) |
| Custom validator | the rule's `wasm_function` (`validate__<code>`) | `ValidatorContext` `{"entity", "referenced", "declared_types", "primitives"}` | `ValidatorVerdict` `{"verdict": "pass"}` or `{"verdict": "fail", "field"?, "value"?}` |
| Scanner | the analyzer's `scan_export` (`scan__<language>`) | `ScanRequest` `{"file_path", "content"}` | `ScanResponse` `{"items": [{"name", "item_kind", "line", "visibility"?, "signature"?}], "language"?}` |
| Migration hook | the handshake's `migration_hook` | `MigrationInput` `{"from", "to", "files"}` | not read |

An analyzer also declares `classify_export` and `map_export`; the host does
not call them.

The wire rules, on both sides:

- **Absent, never null.** An optional field that is unset is left out (the
  host leaves out a collect input's `stdout`, a pass input's `test_results`,
  `proved_claims` and `previous`, an entity's `span`), and an absent optional
  field reads as its default.
- **Strict on shape, lenient on unknown fields.** A required field is required:
  a command answer without `exit_code`, a resource answer without `mime_type`,
  a collected test without `name` or with a status outside `passed`,
  `failed` and `skipped` is not that type. A field the reader does
  not know is ignored, so a newer peer may add one.

Every failure of a call is E028, in one shape: `<operation> <export>() of
'<extension>' trapped: <kind>: <message>` (the export trapped, ran out of time
or fuel, or the guest does not route it: `guest_error: unknown export
'<export>'`), `... answered output that is not a <Type>: <reason>`, or `... is
not loaded`, with the suggestion to report it to the extension's author. What
the failure costs is the operation's: a check pass's is a compile error, an
analyze pass's an E028 finding of that pass (the analysis fails), a command's
its error (exit 1; under `--format json` one `{code, message, suggestion}`
object on stderr), an MCP tool's or resource's a structured MCP error, a
scanner's an entry of the gap report's `scan_failures` (the report is then
approximate), a collector's the `collect` error, a custom validator's the
probe's W112 at load, a migration hook's a failure line that refuses the
migration (nothing is written).

With the SDK, every one of these exports is declared together with its
handler (`ContributionsBuilder::command`, `mcp_tool`, `mcp_resource`, `pass`
with `run`, `collector` with `collect`, `rule` with `validate`, `analyzer`
with `scan`, `migration_hook_handler`); the SDK decodes the input and encodes
the answer. See [the SDK guide](extension-sdk.md).

## Writing

A command or collector of 1.9.0 may declare `writes: true` (ADR 0289). It does not write files: it answers an
**edit plan**, and the host checks it, plans it with the typed edit engine, judges it as one edit and applies it all
or nothing, under the project's write lock and a write journal (a write interrupted by a crash is put back by the
next one; W167 reports one that is waiting). Only a command or collector that declares `writes` may answer a plan
(`undeclared_write` otherwise).

A writing command's input also carries `revisions`, `{"<file>": "sha256:<hex>" | "absent"}` for the project's
sources, and every `graph` node carries its `file`. A writing collector is sent `cwd`, `today`, `graph` and
`revisions` beside the report. The command answers `CommandOutput.plan`, a collector `CollectOutput.plan`:

```json
{
  "title": "Record 03-02",
  "ops": [
    {"op": "set", "entity": "03-02", "field": "status", "value": "done", "revision": "sha256:9f2c..."},
    {"op": "append", "entity": "03-02", "field": "checks", "value": {"status": "pass"}, "revision": "sha256:9f2c..."},
    {"op": "create", "kind": "decision", "id": "D-9", "title": "Use X", "file": "spec/d.spec", "revision": "absent"}
  ]
}
```

| `op` | Fields besides `revision` |
|---|---|
| `create` | `kind`, `id`, `title`, `file`, `fields`?, `verify`? (`[{"kind"?, "text"}]`) |
| `set`, `append` | `entity`, `field` (a path: `status`, `completion.at`, `checks[0].status`), `value` |
| `unset` | `entity`, `field` |
| `remove` | `entity`, `field`, `index` |
| `link`, `unlink` | `entity`, `field`, `target` |
| `retitle` | `entity`, `title` |
| `add_verify`, `remove_verify` | `entity`, `kind`?, `text` |
| `delete` | `entity`, `unlink_references`? |
| `rename` | `entity`, `to` (the move record is written with the ledger) |
| `set_verdict` | `entity`, `rule`, `outcome`, `judge`, `model`, `findings`?, `cited`? (`{"entity"}`, `{"file"}` or `{"test"}`), `values`?, `basis`? |

Ops apply in order, each over the texts the ones before it left. An op's `revision` is the revision of the file it
writes, as the call received it (`absent` for a file that is not there yet); an op with none is `invalid_plan`. If the
file has changed since the compile the call read, the whole plan is refused `stale_revision` and nothing is written:
the command reads again and answers a new plan. A `set_verdict` has no file revision; its `basis` names the inputs
the judgement read, and any that changed is `stale_revision` with `inputs`.

A plan is refused, with nothing written, by one of these codes (the CLI prints `error[<code>]` and exits 1; MCP puts
the code in `data.code`):

| Code | The plan |
|---|---|
| `invalid_plan` | has an op that is not well formed (the message names its number, from 1) |
| `undeclared_write` | comes from a command or collector that does not declare `writes` |
| `human_only` | writes a kind, field or member declared `writer: "human"` |
| `computed_field` | writes a `computed` value |
| `harness_field` | sets a `harness` value its origin may not set |
| `stale_revision` | names a file revision that is not the file's now |
| `not_judged`, `verdict_refused` | writes a verdict for a rule the host does not judge, or one it refuses (`reason`: `kind`, `outcome`, `origin`, `judge`, `value`, `cite`) |
| `E080` | would leave the project with an error it does not have now (a guard pass counts) |

The host adds `--dry-run` to a writing command (it prints the diff and writes nothing), so a writing command declares
no arg of that name. A writing command's MCP tool is a mutation and reports the files it wrote.

## Host Functions

The protocol was designed to be bidirectional: the host drives the
conversation (calling exports on the Wasm module), and extensions would call
back into the host through imported functions to read the graph, emit
diagnostics, and access files. None of that is built. In particular an
extension cannot read a business system, a file or the network: what it
knows is what the call hands it.

> **Status: not callable from guests today.** The old extism-era import
> surface was removed with the component-model cutover. Guests are
> pure-compute wasip2 components; the host passes all needed context as call
> input (e.g. the `ValidatorContext` snapshot for `validate__*` exports). A
> typed component host-import surface is future work — the names below are
> the planned surface, specified in `spec/behaviors/wasm-host-functions.spec`.
> The permissions those functions would check come with them; today's sandbox grants none
> (Sandbox, ADR 0037).

### Host Function Table

| Function | Signature | Purpose | Allowed call sites |
|----------|-----------|---------|--------------------|
| `host_query_graph` | `(pattern: &str) -> Vec<Entity>` | Scope-limited query of the entity graph by kind, field values, or graph pattern | all |
| `host_emit_diagnostic` | `(severity: Severity, code: &str, msg: &str)` | Emit a diagnostic to the host's diagnostic collection | all |
| `host_read_file` | `(path: &str) -> Option<String>` | Read a file from the project (subject to sandbox policy) | Validator, Provider, Parser, Analyzer |
| `host_emit_file` | `(path: &str, content: &[u8])` | Write renderer/collector output (subject to sandbox policy) | Renderer, Collector |
| `host_http_get` | `(url: &str) -> Vec<u8>` | Fetch from an allowlisted domain | Provider |
| `host_add_graph_node` | `(kind: &str, id: &str, fields: ...)` | Add a graph node | Parser |
| `host_add_graph_edge` | `(label: &str, source: &str, target: &str)` | Add a graph edge | Parser |

Reference resolution — resolving an entity ID to its typed node, with a
`None`/null kind for dangling IDs — is part of `host_query_graph` semantics
(the scope-filtered graph carries nodes and edges) and of the precomputed
`ValidatorContext.referenced` snapshot handed to `validate__*` exports.

### Host API Versioning

The host-function surface is versioned alongside the wire protocol (`protocol_version` in the handshake response). A compatible major version guarantees the same function names and signatures.

| Protocol Version | Functions Available |
|-----------------|-------------------|
| `1.0.0` | `host_query_graph`, `host_emit_diagnostic`, `host_read_file`, `host_emit_file`, `host_http_get`, `host_add_graph_node`, `host_add_graph_edge` |
| `1.1.0` | the same as `1.0.0` |
| `1.2.0` | the same as `1.0.0` |
| `1.3.0` | the same as `1.0.0` |
| `1.4.0` | the same as `1.0.0` |
| `1.5.0` | the same as `1.0.0` |
| `1.6.0` | the same as `1.0.0` |
| `1.7.0` | the same as `1.0.0` |
| `1.8.0` | the same as `1.0.0` |
| `1.9.0` | the same as `1.0.0` |
| `1.10.0` | the same as `1.0.0` |

### Protocol Versions

The protocol version is semver (`specforge_protocol_types::PROTOCOL_VERSION`). The host loads every
guest of its major version, whatever minor it was built with, and sends its own version as
`host_version` in every handshake request. A guest built with the SDK declares the SDK's version in
its handshake and its registry manifest.

The minor moves when a payload changes in a way every older guest survives: a field added, removed or
given a new meaning. The major moves only when an older guest could no longer be decoded
or answered.

| Version | What it guarantees |
|---------|--------------------|
| `1.0.0` | The baseline: the handshake, describe and operate payloads of this document. |
| `1.1.0` | A validation rule's optional `target_extension` (ADR 0020). Field text (ADR 0019): every field an entity writes has one text, the same in a pass's `PassEntity.fields`, a validator's `ValidatorField.value` and what declarative rules match (see "Field text" in `extension-sdk.md`). A written field is present even when empty (`""`). A validator's field `value` is always a string (a variant list, mixed list, expression or type union was `null`). `PassEntity.exempt` follows the host's one obligation rule: a `no_verify_statements` rule without a target kind obliges every kind that accepts `verify` statements (an entity of a kind that accepts none is exempt). |
| `1.2.0` | The declared categories are the only describe categories (`fields`, `grammars`, `body_parsers` are gone); the handshake request carries only `host_version`; the handshake response carries `providers` and no `contribution_flags`; a pass's `PassEntity` says `owes_none` where it said `exempt` (ADR 0050). A 1.2 host loads every 1.x guest and ignores an older guest's flags; a 1.1 host cannot read a 1.2 guest's handshake. |
| `1.3.0` | An enhancement declares no `edge_types`: an edge type is declared once, in `edges` (an enhancement's fields name it as their `edge`). A 1.3 host reports a 1.2 guest's enhancement `edge_types` key as W138 and ignores it (ADR 0060). |
| `1.4.0` | A recorded test is one `TestResult` with a `TestStatus`: `passed`, `failed` or `skipped`, decoded strictly on the collect wire (a collector's other status is E028), and a pass's `test_results` carry the same, `duration_ms` included (they said `pass`/`fail`). A 1.3 pass that reads `test_results` misreads 1.4 values (ADR 0069). |
| `1.5.0` | An entity's `verify` statements are no field text (ADR 0094): a pass's `PassEntity.fields` and a validator's `ValidatorField` carry no `verify` entry, so a field named `verify` is never overwritten; a pass reads the statements in `verify_kinds` and `verify_texts`, and the graph a command gets may carry a node-level `verify`, which a guest ignores. A 1.4 guest that read `fields["verify"]` loses it. An enhancement names no `source_extension`: the host credits its fields to the answering extension and reads the kind's owner from its kind registry (ADR 0096); a 1.5 host reports a 1.4 guest's key as W138 and ignores it. |
| `1.6.0` | A field's `field_type` may be `date` (ADR 0234), and a validator's `primitives` include `date`; a 1.5 host drops a `date` field with W019. (`identifier`, ADR 0147, came under 1.5.0 without a bump.) |
| `1.7.0` | An entity kind may declare `scoped_by`, the single `reference` field naming the entity its entities are scoped by (ADR 0284): a name is unique within its scope, and an entity's id is its scope's path and its name (`v1.0/03/D-01`), so a pass's `PassEntity.id` and edge ends may hold `/`; a 1.6 host ignores the key, and ids stay flat. An extension catalogs its codes (`codes`, ADR 0286); a guest older than 1.7 is not asked for the category and loads with none. A validation rule's `description` (added under 1.6.0 without a bump, ADR 0278) is gone, superseded by `codes`; a guest that still sends it loads, and the key is W138. A field's `field_type` may be `enum_list` or `block_list`, and a field may declare `members` (the typed fields of a `block` or of each `block_list` item, themselves field descriptors), `target_kinds` (a union of entity kinds a reference may name), integer `minimum` and `maximum`, and a file field's `file_base` (`spec` or `project`; ADR 0285). A pass's `PassEntity.values` and a validator's `ValidatorEntity.values` carry the structured value (`{"a": ..}`, `[{..}, ..]`) of every field that declares members, keyed by field name; `fields` keeps the text. A 1.6 host drops the new types with W019 and ignores the new keys. |
| `1.8.0` | A field and a block member may declare `writer` (`computed` or `harness`), `immutable` and `append_only`, and a kind `append_only` (ADR 0287): E082 a hand-written computed value, E083 a harness value with no line in `specforge-harness`, E084 an immutable value that differs from its git baseline, E085 an append-only list or kind that rewrites it. A pass may declare `baseline` and `computes`, phase `compute` runs it before the checks, and `PassOutput.values` gives it computed values (W165 for a field that is not computed). `previous` is the git baseline of the kinds `baseline` names, not the build cache (`specforge-cache.json` and `check --cache` are gone). `lifecycle_field` is ignored. An extension may declare the `settings` category (ADR 0288): typed settings, presets and secrets, and an older guest is not asked for it. A 1.7 host ignores the new keys. |
| `1.9.0` | A rule may be `check: "judged"` with a `judged` descriptor, an entity may hold `verdict` statements, a field may declare `writer: "verdict"`, and `PassContext.judgements`, `GraphNode.judgements` and `PassBaselineEntity.judgements` carry each judged rule's state (ADR 0290); `set_verdict` takes `values` and `basis`. A command or collector may declare `writes` and answer an edit `plan` the host checks and applies (ADR 0289); a writing command's input carries `revisions`, a graph node its `file`, and a writing collector the graph, `cwd`, `today` and `revisions`. A kind and a field may declare `writer: "human"`, and a pass `phase: "guard"`. A 1.8 host ignores the new keys. |
| `1.10.0` | A `surfaces` item may declare `prompts` (a `PromptDescriptor`: typed `args`, `gates`, `subject`, `steps`, `when`, `reads_graph`, `attribution`) served as MCP prompts, `specforge prompt` and `specforge <short> <id>`, rendered by a `prompt__<id>` export (`PromptInput`, `PromptOutput`) with a host message appended (ADR 0291). A command may declare `gates` and `subject`: the host refuses the call `gate_failed` while a gated code fails in the subject's scope. A pass may declare `reports`, the codes it answers at a gate, and `PassInput.trigger` says whether it is asked to judge a write or a gate. A 1.9 host ignores the new keys, so a gated command runs ungated there. |

A host of `1.0.x` still loads a `1.1.0` guest and hands it the `1.0.0` values; a guest that needs the
`1.1.0` values can read `host_version` in its handshake request.

### Sandbox

An extension runs with no capability: its component's WASI context preopens no directory, passes
no environment, arguments or stdin, discards stdout and stderr, and refuses sockets and name
lookup. What it needs it is handed in each call's input (a scanner gets each file's content, a
command the graph). No declaration grants more (ADR 0037). No text an extension supplies acts on
the user's terminal either: the host reads a handshake, describe, pass or validator answer, and a
call's failure text, printable where it reads them (ADR 0244); a command's output and an MCP tool's
or resource's answer are the extension's own and are written as answered.

The handshake's `sandbox_policy` declares limits, which the host enforces on every call:

```json
{ "max_execution_ms": 5000, "max_memory_mb": 256 }
```

| Limit | Ceiling (and value when undeclared) | Enforced by | A call past it |
|-------|-------------------------------------|-------------|----------------|
| `max_execution_ms` | 30000 | epoch interruption, checked every 10 ms; never before the budget | traps `deadline_exceeded` |
| `max_memory_mb` | 512 | a limit on the instance's linear memory | traps `memory_limit_exceeded` |
| (fuel, not declared) | a fixed instruction budget, whole for every call | fuel metering | traps `fuel_exhausted` |

A declared limit above its ceiling is held to it. A call that traps on a limit fails with E028
naming the limit, and the extension's next call gets a fresh instance under the same limits. A
`sandbox_policy` key other than the two limits that asks for something (`network_access: true`, a
non-empty `allowed_paths`, which older SDKs wrote), a surface's `sandbox` override, and a limit
above its ceiling are W153 at load.

## Reloading extensions

The LSP and the MCP server keep a project loaded between requests. When `specforge.json`,
`specforge.lock` or an extension's module changes on disk, they load the environment again from the
first step (ADR 0159): every extension is loaded, handshaken and described afresh, the registries are
rebuilt, and the extensions' MCP tools and resources are registered again. Subscribed MCP clients
hear what changed, as a diagnostics delta. The CLI loads the
environment once per command.

There is no partial, typeless mode. While an extension is not enabled or does not load:

- An entity whose keyword no loaded extension declares is **E024** (unknown entity kind); a failed
  load is **E028**.
- A reference field that targets a kind no enabled extension declares, an enhancement of such a kind,
  or an `@scope/name` import of a known extension that isn't installed is **I004** (extension not
  installed).
- Its rules, passes, commands, MCP tools and resources are simply absent.

Install it again (`specforge add`) and the next load restores all of it.

## Lifecycle Summary

```
  .wasm component
        │
        v
  LOAD ──> __handshake(host_version)      name, version, protocol_version,
        │                                 peers, sandbox_policy, providers
        v
  __describe(category), each once        entities, edges, shared_fields, enhancements,
        │                                 validation_rules, surfaces, collectors,
        │                                 analyzers, passes, feature_flags,
        │                                 codes, settings
        v
  REGISTER                               KindRegistry, FieldRegistry, EdgeRegistry
        │
        v
  OPERATE (on demand)                    cmd__*, prompt__*, mcp__*, __pass_*, collect__*,
        │                                 validate__*, scan__*, migration hook
        v
  RELOAD when specforge.json, specforge.lock or a module changes: from LOAD
```

## Wasm Export Naming Conventions

All extension exports follow a strict naming convention that the host uses to dispatch calls:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `__handshake` | Protocol handshake | `__handshake` |
| `__describe` | Category description | `__describe` |
| `cmd__` | CLI command execution | `cmd__product_features` |
| `prompt__` | MCP prompt rendering | `prompt__plan_phase` |
| `mcp__` | MCP tool or resource execution | `mcp__model` |
| `__pass_` | Compiler pass | `__pass_coverage` |
| `collect__` | Collector execution | `collect__cargo_test` |
| `validate__` | Custom validation logic (a rule's `wasm_function`) | `validate__port_methods` |
| `scan__` | Source scanner (an analyzer's `scan_export`) | `scan__rust` |
| `classify__`, `map__` | Declared by analyzers; the host does not call them | `classify__rust` |
| (any name) | Migration hook (the handshake's `migration_hook`) | `migrate_acme` |

## Design Principles

The Extension Protocol embodies two SpecForge principles:

**Principle 2 (zero domain knowledge in core):** The host never hardcodes knowledge of what extensions will contribute. It discovers everything through `__handshake` and `__describe`.

**Principle 7 (extensions over built-ins):** The protocol is the sole mechanism for adding domain vocabulary. There is no alternative path that bypasses it.

**Principle 8 (seconds to value):** The declaration is read once per environment load, the handshake and one `__describe` call per declared category, and nothing describes a category again; the operations that follow call only the exports they need.

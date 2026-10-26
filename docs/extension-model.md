# Extension Model

What you get from this page: how SpecForge learns a domain. SpecForge's core knows no domain at all: not
software, not product management, not pharma. **Extensions** teach it one. An extension says which kinds of
things exist (an SOP, a campaign, a behavior), what fields they have, how they link, which rules they must obey,
and which commands can query them. Load the extension and SpecForge speaks that domain; every check, query and
editor feature works on it with no change to SpecForge itself.

Think of extensions as the apps, and SpecForge as the system they run on.

## What an extension can declare

Everything below is a builder call in the extension SDK (`ContributionsBuilder`,
[`crates/runtime/extension-sdk/src/lib.rs`](../crates/runtime/extension-sdk/src/lib.rs)):

| Contribution | What it adds | SDK call |
|--------------|--------------|----------|
| **Kinds** | A new block keyword (`sop`, `campaign`), its fields with their types, required fields, enum values | `kind` |
| **Edges** | A typed relationship between kinds (`SopGovernsStep`), written by a reference field | `edge` |
| **Shared fields** | A field every kind of the extension gets (product's `tags`) | `shared_field` |
| **Enhancements** | Fields, or `verify` kinds, added to a kind *another* extension declares | `enhance` |
| **Rules** | Checks run on every compile and every edit: a missing link, a cycle, a missing field when a flag is set, a value constraint, a file that must exist, or a custom check written in code | `rule` |
| **Passes** | Analyses over the whole graph (formal's `condition_check`, testing's `coverage`) | `pass` |
| **Commands** | `specforge <extension> <command>` on the CLI, promoted to an MCP tool of the same name | `command` |
| **MCP tools and resources** | Explicit tools and resources for AI agents | `mcp_tool`, `mcp_resource` |
| **Collectors** | A test runner `specforge collect` can run (after you approve the command) and the mapping from its report to entities | `collector` |
| **Analyzers** | A source-code scanner `specforge infer` uses to suggest entities from existing code | `analyzer` |
| **Providers** | A `ref` scheme the project can configure (see [below](#providers-registered-not-yet-connected)) | `providers` |
| **Peers** | Other extensions it needs, or works with when present, with a version range | `peer`, `optional_peer` |
| Starter file, diagram colour, feature flags, a migration hook | Conveniences: what `specforge init` writes, how `model` and `outline` draw it | `starter_template`, `theme_color`, `feature_flag`, `migration_hook_handler` |

Here is the start of a real declaration, `@specforge/governance` declaring the `decision` kind
([`extensions/governance/src/declaration.rs`](../extensions/governance/src/declaration.rs)):

```rust
const DECISION_STATUS: Vocabulary =
    Vocabulary::new(&["proposed", "accepted", "deprecated", "superseded"]);

c.kind("Decision", |k| {
    k.keyword("decision")
        .description("An architectural or design decision with rationale and status");
    k.field("status", |f| {
        f.vocabulary(&DECISION_STATUS)
            .required()
            .description("Current lifecycle status of the decision")
            .headline();
    });
    k.field("decision", |f| {
        f.field_type(FieldType::String)
            .required()
            .description("The decision statement itself")
            .normative();
    });
    // ...
});
```

From that, a project that loads the extension can write `decision use_postgresql "..." { status accepted ... }`;
a missing `status` is E006, `status maybe` is E061, the editor offers the `status` values as completions, and `specforge model`
draws the kind.

## How extensions run

An extension is a **WebAssembly component**. SpecForge loads it in a sandbox and calls it through one entry
point ([`bridge.wit`](../crates/runtime/component/wit/bridge.wit) exports a single `call` function and imports
nothing). The component is granted no capability: no files, no environment, no network; each call runs within a
memory and time budget ([`crates/runtime/wasm/src/sandbox.rs`](../crates/runtime/wasm/src/sandbox.rs)). An extension
can declare tighter memory and time limits, never looser ones than the host allows.

That makes extensions **pure**: they read the graph SpecForge hands them and answer. They cannot read a database,
call an API or write a file. When something needs to touch the outside world (running a test command, for
example), the extension declares it and SpecForge does it, asking you first.

## How a project loads them

`specforge.json` lists the extensions a project loads, and that list decides which kinds exist:

```json
{
  "name": "clinic-operations",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/product", "@acme/gxp", "./extensions/sites.wasm"]
}
```

An entry is a builtin (`@specforge/product`), a package installed from a registry (`@acme/gxp`), or a `.wasm`
file. Use the commands rather than editing by hand: `specforge add`, `specforge remove`, `specforge extensions`.
Extensions load in peer order: an extension loads after the peers it declares.

## The builtins are examples

SpecForge ships nine builtin extensions. They are a starter set, aimed at software and product work, and they
are written with the same SDK any extension uses:

| Builtin | Shows how to |
|---------|--------------|
| `@specforge/product` | Declare a domain vocabulary (9 kinds) and 40 query commands |
| `@specforge/software` | Declare kinds, enhance another extension's kinds, write custom validators |
| `@specforge/governance` | Declare kinds that point into other extensions, with optional peers |
| `@specforge/formal` | Add analysis passes and fields to kinds it does not own |
| `@specforge/testing` | Declare no kind at all, only `verify` obligations and a coverage pass |
| `@specforge/cargo-test`, `@specforge/vitest` | Declare a collector for one test runner |
| `@specforge/rust`, `@specforge/typescript` | Declare a source analyzer for `specforge infer` |

The [extension inventory](extension-inventory.md) lists exactly what each declares; the
[entity pages](entities/) document each builtin kind.

## Any domain

Because the core has no domain, the same mechanism fits very different work. These are examples of what an
extension would declare, not extensions that ship:

| Domain | Kinds an extension might declare | Rules worth checking |
|--------|----------------------------------|----------------------|
| Pharma / GxP | `sop`, `procedure_step`, `evidence`, `deviation`, `capa` | Every step names its approver; every deviation has a CAPA; every SOP has a review date |
| Marketing | `campaign`, `audience`, `channel_plan`, `asset`, `claim` | Every claim cites its evidence; every campaign has an owner and an audience |
| Operations | `site`, `process`, `kpi`, `owner` | Every KPI has a target and an owner; no process without a site |
| Software spec frameworks | the artefacts of GSD, BMAD, OpenSpec or SpecKit | Each framework's own rules, checked on every edit |

A GSD extension family (`@gsd/*`) is **in progress**: it is at the design stage and has no code yet. BMAD,
OpenSpec and SpecKit extensions are not started.

## Writing your own

```bash
specforge extension init      # scaffold an extension crate written with the SDK
specforge extension build     # build its WebAssembly component
specforge extension validate  # load the component and report its declaration's problems
specforge add ./target/wasm32-wasip2/release/my_kinds.wasm
```

The walkthrough is [Extending SpecForge](guides/extending-specforge.md). The SDK reference is
[extension-sdk.md](extension-sdk.md); the wire protocol behind it is [extension-protocol.md](extension-protocol.md).
Extensions you publish go to a package registry (`specforge publish`); see [registry-trust.md](registry-trust.md)
for how packages are signed and checked.

## Providers: registered, not yet connected

A provider is an extension that serves a `ref` scheme (`gh` for `ref gh.issue:42 "..."`). A project configures
providers in `specforge.json`:

```json
"providers": [
  { "scheme": "gh", "alias": "work", "extension": "@acme/gh", "settings": { "repo": "acme/service" } }
]
```

Declaration order decides which provider wins a shared scheme (E057), and two instances of one provider use two
schemes. `specforge providers` lists each entry and whether its scheme is registered.

**That is all a provider does today.** The settings are passed through but nothing reads them: there is no host
reader yet ([`crates/session/project/src/providers.rs`](../crates/session/project/src/providers.rs)), and since
extensions are pure they cannot reach GitHub, Jira, a CRM or a document store themselves. A ref is not checked
against the system it names. Live connectors to outside systems are roadmap.

## What extensions do not do

- **Generate code.** There is no generator contribution; outputs are the core's exports (`specforge export`,
  `model`, `outline`, `trace`).
- **Reach outside systems.** See providers above.
- **Collect evidence beyond tests.** Proof today is test-centred: `verify` kinds such as `unit`, `property` or
  `integration`, and collectors that read test runner reports. A general notion of evidence for non-software
  domains (a signed batch record, an audit finding) is future work.

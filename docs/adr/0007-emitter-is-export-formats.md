# The emitter is export formats; domain names come from manifests

**Status:** accepted (2026-10-02)

`specforge-emitter` held the compile's glue (graph checks, extension loading, Wasm custom rules),
the analysis passes, the host's coverage view, traces, plan validation, stats, source scanning,
diagnostic presentation and the schema cache's file I/O beside the export formats, and linked the
Wasm runtime for them. ADR 0004 D1-d wanted it free of the runtime. It also named domain concepts
the compiler must not know (principle 2): union `variants`, `abstract`, the `invariant` and
`property` kinds, the `type` kind, `contract` and `status`, and the builtin extensions by name.

## Where things live

- **specforge-emitter**: the export formats only (graph JSON, Graph Protocol V2 schema and
  exports, context, brief, scoped and budgeted exports, DOT, the model and outline diagrams).
  It depends on the graph, the registry, serde and sha2; no Wasm runtime, no file I/O. The
  three DOT renderers draw different things and keep their layouts; escaping and identifiers
  are shared (`diagram.rs`). The two Mermaid renderers (an ER diagram, a flowchart) share nothing
  worth extracting. The legacy flat re-exports are gone: callers use `emit` or a format module.
- **specforge-project**: `compile` (graph checks, `load_extensions`, Wasm custom rules),
  `field_types`, `passes` (extension compiler passes), `coverage` (the host's coverage view and
  the test report format).
- **specforge-ops**: the built-in `contracts` pass, `trace`, `plan`, `stats`, `scan`,
  `schema_cache`.
- **specforge-common**: diagnostics JSON, the one-line form, the output cap and the exit code,
  beside `Diagnostic`. The catalog crate stays dependency-free (D6-d); common links it for the
  titles. The validator keeps the source-annotated rendering and the one summary line
  (`aggregate_diagnostic_summary`; the emitter's other summary had no caller).

## What extensions now declare

| Declaration | On | Replaces | Builtins that set it |
|---|---|---|---|
| `exempts_obligations` | field | `abstract` by name (D2-b) | formal's `abstract` enhancement |
| `headline` | field | `contract`, `status` in the context export | software `contract`, `status`; every product and governance `status` |
| `contract_target` | entity kind | `["invariant", "property"]` (A010) | software `invariant`, formal `property` |
| `declares_types` | entity kind | `kind == "type"` (custom validators' `declared_types`) | software `type` |
| `theme_color` | handshake / manifest | the model and outline palettes keyed by extension name | software, product, governance, formal |

A union body (`kind X = a | b`) is structural syntax: it owes no obligations because it has no
body to hold them. It is recognised by the key the parser itself gives it
(`specforge_parser::UNION_VARIANTS_FIELD`), not by a value's shape alone: the parser also turns
a user's `values [a, b]` into a variant list, and that exempts nothing. `title` (the entity
title) and `verify` (ADR 0002) stay structural. The flags are optional on the wire and in
`ManifestV2`; the Graph Protocol schema does not carry them, so exports are unchanged.

MCP `inspect`'s `contract` and the context prompt's `contract_text` are the entity's field
declared both `headline` and `normative` (`specforge_emitter::context::headline_statement`): a
behavior's or an event's `contract`; a `status` is headline only.

One visible consequence: the context export lifts only declared headline fields, so a struct
member that happens to be named `status` on an open-fields kind (a `type`) is no longer shown
as the entity's status, as `abstract` already exempts nothing unless declared; MCP's `contract`
likewise.

## Known gaps

- ~~**The prove pass** (`specforge_ops::prove`) is host-side because z3 is native. It reads
  `constraint` entities' `metric` as bounds and any entity's `expression` as a claim, and the
  unknown-field check accepts `expression` on every kind for it. Moving that needs a manifest
  declaration of bound and claim fields; it waits for the shared prove code (D3-f).~~ Closed by
  [ADR 0009](0009-host-passes-read-declarations.md): fields declare a proof role, and the pass
  stays host-side.
- ~~**The coverage rule** (`specforge-coverage`, owned by `@specforge/testing`, D2-f) names
  `invariant` and its `risk` (A002, the risk tallies) and the `property` verify kind; the host
  passes `risk` by name into it. `contract_target` is not a substitute: it also marks formal's
  `property` kind, which would gain risk tallies and A002. It needs a declaration of the
  risk-graded kind and its risk field, sent to the testing pass.~~ Closed by
  [ADR 0009](0009-host-passes-read-declarations.md): `@specforge/testing` passes its risk
  grading to the rule. The `property` verify kind stays: it is testing's own vocabulary.
- ~~**The build cache** (`specforge_project::BuildCache`) records every entity's `status` for the
  product's transition checks (W087–W091). Recording declared fields instead changes the cache
  file and `PassBuildCache` (the SDK, so every blob), and wants a field flag of its own.~~ Closed
  by [ADR 0009](0009-host-passes-read-declarations.md): a kind declares its lifecycle field, and
  the cache file and `PassBuildCache` are unchanged.
- ~~**`specforge-cli/src/product`** implements product queries natively.~~ Closed by
  [ADR 0008](0008-extension-commands-run-over-the-graph.md): they are `@specforge/product`'s commands.
- The model's DOT cluster ids strip the `@specforge/` scope (snapshot-locked output).

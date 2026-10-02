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
body to hold them, decided from the value's shape, not the `variants` name. `title` (the entity
title) and `verify` (ADR 0002) stay structural. The flags are optional on the wire and in
`ManifestV2`; the Graph Protocol schema does not carry them, so exports are unchanged.

One visible consequence: the context export lifts only declared headline fields, so a struct
member that happens to be named `status` on an open-fields kind (a `type`) is no longer shown
as the entity's status, as `abstract` already exempts nothing unless declared.

## Known gaps

- **The prove pass** (`specforge_ops::prove`) is host-side because z3 is native. It reads
  `constraint` entities' `metric` as bounds and any entity's `expression` as a claim, and the
  unknown-field check accepts `expression` on every kind for it. Moving that needs a manifest
  declaration of bound and claim fields; it waits for the shared prove code (D3-f).
- **The coverage rule** (`specforge-coverage`, owned by `@specforge/testing`, D2-f) names
  `invariant`, `property` and their `risk`; the host passes `risk` by name into it.
- **The build cache** (`specforge_project::BuildCache`) records every entity's `status` for the
  product's transition checks (W087–W091).
- **MCP** `inspect` and the context prompt print an entity's `contract` under that key.
- **`specforge-cli/src/product`** implements product queries natively.
- The model's DOT cluster ids strip the `@specforge/` scope (snapshot-locked output).

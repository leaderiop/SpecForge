# Model command architecture

What you get: where the `specforge model` code lives and how it turns the loaded extensions' declarations into a
drawing. For how to use the command, see [README.md](README.md).

## Data flow

```
ProjectView (the compiled project)
   |  versioned_schema(): the Graph Protocol schema, the same one `specforge schema` prints
   |  registries().declarations(): each loaded extension's declaration (for its theme colour)
   v
specforge_ops::read::model::model(view, options)          -- the one operation, CLI and MCP alike
   |  refuses an --extension the project does not load (extension_not_found)
   |  refuses a --root no loaded extension declares (unknown_kind)
   |  reports a --kinds name the project does not know (I020)
   v
specforge_emitter::model::export(schema, declarations, options) -> String
   |  ModelIntermediate::of(schema)        build the IR: one entity per kind, a synthetic `id` first,
   |                                       one relationship per (edge type, source kind, target kind)
   |  .with_theme_colors(declarations)     each extension's declared theme_color
   |  .selected(options)                   --extension, --kinds, --root/--depth; recount the extensions
   |  .with_fields(options.fields)         none | keys | all
   v
markdown | mermaid | dot | json | dbml renderer
```

`export` is total: a name the schema does not have selects nothing. Refusing or reporting such a name is the
operation's job.

## Where the code is

| Path | Holds |
|------|-------|
| [`crates/pipeline/emitter/src/model/mod.rs`](../../crates/pipeline/emitter/src/model/mod.rs) | `ModelOptions`, `ModelRoot`, `ModelFormat`, `GroupBy`, `FieldLevel`; the IR types; `export` |
| [`build.rs`](../../crates/pipeline/emitter/src/model/build.rs) | Builds the IR from the schema |
| [`cardinality.rs`](../../crates/pipeline/emitter/src/model/cardinality.rs) | Cardinality from the fields that write an edge |
| [`filter.rs`](../../crates/pipeline/emitter/src/model/filter.rs) | The selection and the field level |
| `markdown.rs`, `mermaid.rs`, `dot.rs`, `json.rs`, `dbml.rs` (same directory) | One renderer per format |
| [`crates/app/ops/src/read/model.rs`](../../crates/app/ops/src/read/model.rs) | The operation both surfaces call |
| [`crates/surfaces/cli/src/model.rs`](../../crates/surfaces/cli/src/model.rs) | The CLI command: arguments to `ModelOptions`, output on stdout, notices on stderr |
| [`crates/surfaces/mcp/src/tools/model.rs`](../../crates/surfaces/mcp/src/tools/model.rs) | The MCP tool `specforge.model`: the same options, notices in `_meta.diagnostics` |

## The intermediate representation

The IR types are private to the emitter (`pub(crate)`); the JSON format serializes them as they are.

- `ModelIntermediate`: the schema version, the extensions (name, version, entity and edge counts, colour), the
  entities and the relationships.
- `ModelEntity`: a kind's name, its extension, its description, its declared `dot_color`, its fields, and the
  extensions that enhance it (`enhanced_by`).
- `ModelField`: name, `FieldType` (the protocol's ten types), required, description, default, enum values,
  whether it is the synthetic key, the kind it references, the extension that contributed it when that is not
  the kind's own (`contributed_by`), and its edge (`contribution`, as `EdgeLabel -> target`).
- `ModelRelationship`: the edge label, source and target kinds, cardinality (`1:1`, `1:N`, `N:1`, `N:M`) and the
  field that writes it.

## Cardinality

`infer_cardinality` looks for the field carrying the edge's label: on the source, a `reference_list` gives N:M and
a single `reference` gives N:1 (1:1 when the target has a single `reference` back with the same label); with no
field on the source, a single `reference` on the target gives 1:N. With no field on either side the builder uses
N:M.

## Related commands

- `specforge schema` prints the Graph Protocol schema the model is built from, as JSON, unchanged.
- `specforge export --format dot` draws the **instances** (your entities and their edges); `specforge model
  --format dot` draws the **schema** (kinds and edge types).
- `specforge outline` draws the extensions themselves: their peers, enhancements and contributions.

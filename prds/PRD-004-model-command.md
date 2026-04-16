# PRD-004: specforge model Command

**Status:** Draft
**Author:** Mohammad AL Mechkor
**Date:** 2026-04-12

---

## Problem Statement

Users and AI agents need to visualize the **schema-level structure** of a SpecForge project: which entity kinds exist, what fields they have, and how they relate via edge types. This is the "Modele Logique des Donnees" (MLD) -- the data model that extensions collectively define.

Today, `specforge schema` emits the raw Graph Protocol JSON schema. While complete, it is not human-readable and provides no visual representation. An agent trying to understand a project's structure must parse the full schema JSON and mentally reconstruct the entity-relationship model. A human must do the same, or build external tooling to visualize it.

There is no built-in way to:

1. **See the entity model at a glance** -- which kinds exist, their key fields, and how they connect
2. **Filter by extension** -- view only the software entities, or only the product entities
3. **Control field detail level** -- show just primary keys and relationships, or all fields
4. **Render in diagram formats** -- Mermaid ER diagrams for documentation, DOT for custom rendering, DBML for database tooling
5. **Query via MCP** -- AI agents cannot request a model visualization through the MCP server

## Solution

Add a `specforge model` command that renders the entity-relationship model in 5 formats: Markdown (default, LLM-friendly), Mermaid erDiagram, DOT (HTML labels with color-coded extensions), JSON (ERD-oriented), and DBML. The command supports filtering by extension, entity kind, and field detail level. It is also exposed as an MCP tool (`specforge.model`) for agent consumption.

The implementation builds on the existing `specforge schema` pipeline. It compiles the project, generates the Graph Protocol schema, transforms it into a model intermediate representation (IR), applies filters, and renders to the requested format.

## User Stories

1. As a developer new to a SpecForge project, I want to run `specforge model` to see a readable summary of all entity kinds and their relationships, so that I understand the project's domain model without reading raw JSON.

2. As an AI agent, I want to call the `specforge.model` MCP tool with `format: "markdown"`, so that I receive a token-efficient overview of the project structure in my context window.

3. As a documentation author, I want to run `specforge model --format=mermaid` to generate a Mermaid ER diagram, so that I can embed it in project documentation that renders on GitHub.

4. As a developer, I want to run `specforge model --format=dot` to generate a Graphviz DOT diagram with color-coded extension headers, so that I can render a publication-quality entity model diagram.

5. As a database designer, I want to run `specforge model --format=dbml` to generate a DBML schema, so that I can visualize the entity model in dbdiagram.io or similar tools.

6. As a developer, I want to run `specforge model --format=json` to get a machine-readable ERD representation, so that I can build custom visualizations or analysis tools.

7. As a developer working on one extension, I want to run `specforge model --extension=@specforge/software`, so that I see only the 5 software entity kinds and their relationships without product/governance/formal noise.

8. As a developer, I want to run `specforge model --kinds=behavior,event,type`, so that I see only the entity kinds I care about and their inter-relationships.

9. As a developer, I want to run `specforge model --fields=none`, so that I see only entity names and relationships without any field details -- a pure graph topology view.

10. As a developer, I want to run `specforge model --fields=keys`, so that I see only primary key, required, and reference fields -- the essential structure without noise.

11. As a developer, I want to run `specforge model --fields=all` (the default), so that I see every field on every entity kind for complete documentation.

12. As a developer, I want to run `specforge model --root=behavior --depth=2`, so that I see behavior and all entity kinds within 2 relationship hops -- a focused neighborhood view.

13. As a developer, I want the Markdown output to include a preamble explaining how to read the model, so that the document is self-contained and useful without external context.

14. As a developer, I want the Markdown output to group entities by extension, so that I can see which extension contributes which entity kinds.

15. As a developer, I want cardinality labels on relationships (1:1, 1:N, N:1, N:M), so that I understand the multiplicity of each relationship from the diagram.

16. As a developer, I want cardinality to be inferred from field types (`reference` = N:1, `reference_list` = N:M), so that the model is accurate without manual annotation.

17. As a developer, I want the DOT output to use HTML table labels with extension-specific header colors, so that entity kinds are visually grouped by their owning extension.

18. As a developer, I want the DBML output to use `TableGroup` for extension grouping, so that dbdiagram.io renders extensions as visual clusters.

19. As a developer, I want the Mermaid output to include `%% @ext-name` comments for extension grouping, so that the ER diagram is navigable in large multi-extension projects.

20. As an AI agent, I want the MCP tool to accept `extension`, `kinds`, `fields`, `root`, and `depth` filter parameters, so that I can request exactly the slice of the model I need.

21. As a developer, I want `specforge model --group-by=extension` (the default) to organize entities under extension headers, so that the output reflects the extension architecture.

22. As a developer, I want `specforge model --group-by=none` to list all entities flat without extension grouping, so that I get a unified view when extension boundaries don't matter.

23. As a developer, I want each entity in the model to include a synthetic `id` primary key field, so that the ERD representation follows standard data modeling conventions.

24. As a developer, I want the JSON format to serialize cardinality as human-readable strings ("1:1", "1:N", "N:1", "N:M"), so that the JSON output is self-documenting.

25. As a developer, I want the model to skip edge types that have no source or target kind (like the generic `References` edge), so that the ERD only shows concrete, typed relationships.

## Implementation Decisions

### Architecture: IR + Renderers

The model command uses a three-stage pipeline:

1. **Build**: Transform `GraphProtocolSchema` into `ModelIntermediate` -- a normalized IR with entities, fields, relationships, and extension metadata. Cardinality is inferred during this step.

2. **Filter**: Apply extension filter, kind filter, root+depth BFS, and field level to produce a filtered `ModelIntermediate`. Orphaned relationships are pruned; extension counts are recomputed.

3. **Render**: Dispatch to one of 5 format-specific renderers that produce output strings from the filtered IR.

This design makes each renderer trivial (60-120 lines) because the IR handles all normalization, cardinality inference, and filtering. Adding a 6th format requires only a new renderer function.

### ModelIntermediate IR

```
ModelIntermediate {
  model_version: String,
  extensions: Vec<ModelExtension>,
  entities: Vec<ModelEntity>,
  relationships: Vec<ModelRelationship>,
}

ModelEntity {
  name: String,
  extension: String,
  description: Option<String>,
  fields: Vec<ModelField>,
}

ModelField {
  name: String,
  field_type: ModelFieldType,
  required: bool,
  description: Option<String>,
  is_primary_key: bool,
  references: Option<String>,  // target entity name
}

ModelRelationship {
  name: String,
  source: String,
  target: String,
  cardinality: Cardinality,  // OneToOne, OneToMany, ManyToOne, ManyToMany
  source_field: Option<String>,
  description: Option<String>,
}
```

### Cardinality Inference

For each edge type, find the field on the source entity that declares `edge == edge_label`:
- `reference` field type -> ManyToOne (each entity references one target)
- `reference_list` field type -> ManyToMany (each entity references many targets)
- No matching field -> default ManyToMany (computed edge)

### SchemaField Enrichment (Prerequisite)

`SchemaField` must be enriched with `edge`, `target_kind`, `description`, and `default_value` from `FieldRegistryEntry`. Without these, cardinality inference cannot find the field-to-edge mapping.

### Filter Dimensions

| Filter | CLI Flag | MCP Param | Behavior |
|--------|----------|-----------|----------|
| Extension | `--extension=@specforge/software` | `extension` | Show only entities from this extension |
| Kinds | `--kinds=behavior,event` | `kinds` | Show only these entity kinds |
| Root+Depth | `--root=behavior --depth=2` | `root`, `depth` | BFS from root kind to N hops |
| Field level | `--fields=none\|keys\|all` | `fields` | Control field detail in output |
| Group by | `--group-by=extension\|none` | `group_by` | Extension grouping vs flat |

Filters compose: `--extension=@specforge/software --kinds=behavior,event` shows behaviors and events from software only.

### Format-Specific Rendering

| Format | Key Feature |
|--------|-------------|
| Markdown | Preamble + extension table + entity sections with field tables + relationship lists |
| Mermaid | `erDiagram` with cardinality notation (`\|\|--\|\|`, `\|\|--o{`, `}o--\|\|`, `}o--o{`) |
| DOT | HTML `<table>` labels, color-coded headers, `subgraph cluster_*` grouping |
| JSON | Pretty-printed `ModelIntermediate` with human-readable cardinality strings |
| DBML | `TableGroup`, `Enum`, `Table` with columns, named `Ref` declarations |

### DOT Color Palette

| Extension | Color |
|-----------|-------|
| @specforge/software | #4a90d9 |
| @specforge/product | #2ecc71 |
| @specforge/governance | #e74c3c |
| @specforge/formal | #9b59b6 |
| fallback | #95a5a6 |

### CLI Interface

```
specforge model [OPTIONS] [PATH]

Options:
  --format <FORMAT>        Output format [default: markdown]
                           [possible values: markdown, mermaid, dot, json, dbml]
  --group-by <GROUP_BY>    Grouping strategy [default: extension]
                           [possible values: extension, none]
  --fields <FIELDS>        Field detail level [default: all]
                           [possible values: none, keys, all]
  --extension <EXT>        Filter to entities from this extension
  --kinds <KINDS>          Filter to these entity kinds (comma-separated)
  --root <ROOT>            Start BFS from this entity kind
  --depth <DEPTH>          BFS depth from root (requires --root)
```

### MCP Tool

```json
{
  "name": "specforge.model",
  "description": "Generate entity-relationship model visualization",
  "inputSchema": {
    "type": "object",
    "properties": {
      "format": { "type": "string", "enum": ["markdown", "mermaid", "dot", "json", "dbml"] },
      "group_by": { "type": "string", "enum": ["extension", "none"] },
      "fields": { "type": "string", "enum": ["none", "keys", "all"] },
      "extension": { "type": "string" },
      "kinds": { "type": "array", "items": { "type": "string" } },
      "root": { "type": "string" },
      "depth": { "type": "integer" }
    }
  }
}
```

## Testing Decisions

### What Makes a Good Test

Tests verify that the model pipeline produces correct output for known inputs. Builder tests assert on IR structure (entity count, field count, relationship cardinality). Renderer tests use insta snapshots to catch visual regressions. Filter tests verify that entities are correctly included/excluded. Tests should not depend on specific line numbers or whitespace in rendered output (use snapshots for that).

### Modules to Test

**Builder** (`ModelIntermediate::from_schema`) -- Empty schema produces empty model. Single entity with fields maps correctly. Edge types produce relationships with correct cardinality. Synthetic `id` PK field is prepended. Extension metadata (entity/edge counts) is computed. Edge types with no source/target are skipped. Prior art: `specforge-emitter/tests/schema.rs`.

**Cardinality inference** -- `reference` field produces ManyToOne. `reference_list` field produces ManyToMany. Missing field produces ManyToMany fallback. Prior art: field type mapping in `specforge-registry/src/registries/field.rs`.

**Filters** -- Extension filter. Kind filter. Root+depth BFS. Filter intersection. Orphaned relationship pruning. Field level (none/keys/all). Prior art: `specforge-emitter/tests/outline.rs` filter tests.

**Renderers (5 formats)** -- Insta snapshots for each format at keys/none/all field levels, with and without extension grouping, and with an empty model. This produces ~30 snapshot files that catch any rendering regression. Prior art: `specforge-emitter/tests/` uses insta extensively.

**CLI E2E** -- `specforge model` produces Markdown. `specforge model --format=mermaid` produces valid ER diagram. `specforge model --format=dot` produces valid DOT. Filter flags work. Prior art: `specforge-cli/tests/` uses assert_cmd.

**MCP tool** -- Tool appears in tool list. Default call returns Markdown. `format: "json"` returns valid JSON. Invalid format returns error. Filter parameters work. Prior art: `specforge-mcp/tests/tools_core.rs`.

## Out of Scope

- **Interactive diagram rendering.** The command produces text output (Markdown, Mermaid, DOT, JSON, DBML). Rendering to PNG/SVG requires external tools (mermaid-cli, graphviz, dbdiagram.io).

- **Instance-level visualization.** This command shows the schema (entity kinds and their fields), not actual entity instances. Instance visualization is a separate concern (graph rendering).

- **Custom color themes.** The DOT color palette is hardcoded. Theming support is deferred.

- **Entity enhancement visualization.** The model shows the final merged schema (after enhancements are applied). It does not separately visualize which fields were added by which extension via enhancements.

- **Diff or changelog.** The command shows the current model state. Comparing two versions of the model is deferred.

## Further Notes

### Agent-First Design

The Markdown format is designed for LLM consumption. It includes:
- A preamble explaining how to read the model
- Entity sections with field tables (name, type, required, description)
- Explicit cardinality labels on relationships
- Extension grouping for navigation

An agent can request `specforge.model` via MCP with `format: "markdown"` and `fields: "keys"` to get a compact overview that fits in a context window. For deeper exploration, it can filter to specific extensions or entity kinds.

### Relationship to specforge schema

`specforge model` and `specforge schema` serve different audiences:

| Command | Audience | Content | Format |
|---------|----------|---------|--------|
| `specforge schema` | Tooling, parsers | Full Graph Protocol schema | JSON |
| `specforge model` | Humans, agents | ERD visualization | Markdown, Mermaid, DOT, JSON, DBML |

The model command consumes the schema as input. It is a **presentation layer** over the schema, not a replacement for it.

### Phased Delivery

The existing implementation plan breaks this into 5 phases:
1. SchemaField enrichment (prerequisite)
2. Core IR + builder + filters
3. Five renderers
4. CLI wiring
5. MCP wiring + tests

Each phase has a clear verification step. The total scope is approximately 14 new files and 9 modified files.

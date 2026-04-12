# `specforge model` — Logical Data Model

## Overview

`specforge model` renders the **schema-level meta-model** (entity kinds, fields, edge types) declared by extensions as a logical data model. It is the "Modele Logique des Donnees" of a SpecForge project.

This is a **core command** with zero domain knowledge. It formats whatever extensions have registered — entity kinds become "tables", fields become "columns", edge types become "relationships". Adding new extensions or entity kinds requires zero changes to the model command.

### What it shows

- Entity kinds declared by extensions (e.g., `behavior`, `feature`, `event`)
- Fields on each entity kind (name, type, required, description, enum values, defaults)
- Edge types connecting entity kinds (e.g., `Implements: behavior -> feature`)
- Cardinality of relationships (inferred from field types)
- Grouping by extension

### What it does NOT show

- Instance data (actual entities in the project). Use `specforge export` for that.
- Runtime behavior or compiler internals.
- Validation rules or diagnostic codes.

## CLI Interface

```
specforge model [OPTIONS]

OPTIONS:
  --format <FORMAT>        Output format [default: markdown]
                           [possible: markdown, mermaid, dot, json, dbml]
  --group-by <STRATEGY>    Grouping strategy [default: extension]
                           [possible: extension, none]
  --fields <LEVEL>         Field detail level [default: keys]
                           [possible: none, keys, all]
  --extension <NAME>       Filter to a single extension
  --kinds <LIST>           Comma-separated entity kinds to include
  --root <KIND>            Root entity kind for depth traversal
  --depth <N>              Max depth from root kind (requires --root)
```

### MCP Tool

Exposed as `specforge.model` with identical parameters. The Markdown format is specifically designed for LLM consumption via MCP.

## Design Decisions

| # | Decision | Answer | Rationale |
|---|----------|--------|-----------|
| 1 | Scope | Schema-only (meta-model) | A true MLD shows structure, not instances. Instance view already served by `specforge export --format=dot` |
| 2 | Command | New `specforge model` | `specforge schema` has an established JSON Schema contract. "model" is the natural name for data model visualization |
| 3 | Ownership | Core command | Zero domain knowledge required — renders extension-declared metadata. Analogous to `schema` and `export` |
| 4 | Formats | DOT, Mermaid, JSON, DBML, Markdown | Each serves a distinct audience and toolchain |
| 5 | Default format | Markdown | Only format readable directly in terminal. Serves both humans and LLMs |
| 6 | Cardinality | Inferred from field types | `reference` -> N:1, `reference_list` -> N:M. No manifest changes needed |
| 7 | Grouping | `--group-by=extension` (default) or `none` | Extension boundaries are natural schema boundaries |
| 8 | Filtering | `--extension`, `--kinds`, `--root`+`--depth` | Three dimensions: by extension, by entity kind, by kind-level adjacency |
| 9 | Field display | `--fields=none\|keys\|all` (default: `keys`) | Required + reference fields by default. Full metadata (type, description, enum values, default) at every level |
| 10 | Mermaid dialect | `erDiagram` | Semantically correct for entity-relationship modeling with native cardinality syntax |
| 11 | Markdown structure | Full detail + framing preamble | Extension summary, entity kind tables with field metadata, relationship lists, LLM-oriented preamble |
| 12 | DBML mapping | Table/Column/Ref/TableGroup + synthetic `id` PK | Natural mapping. Enum definitions extracted. Extension grouping via TableGroup |
| 13 | DOT rendering | HTML-like labels | Full control: color-coded headers by extension, bold required fields, reference markers |
| 14 | JSON structure | ERD-oriented (distinct from `specforge schema`) | Simplified schema focused on entities-as-tables, fields-as-columns, edges-as-relationships with cardinality |
| 15 | Output destination | stdout only | Consistent with all SpecForge commands. Unix composability via `>` |
| 16 | MCP surface | Exposed as `specforge.model` | Markdown format designed for LLM consumption — must be accessible via MCP |
| 17 | Kind-level depth | `--root`+`--depth` on kind adjacency graph | Not instances — shows a kind plus all kinds connected within N hops by edge types |

## Architecture

See [architecture.md](architecture.md) for the data flow, crate locations, and key types.

See [formats.md](formats.md) for detailed format specifications with examples.

See [field-tiers.md](field-tiers.md) for the three field display tiers.

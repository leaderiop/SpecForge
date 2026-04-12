# Phase 7: Renderer Improvements

**Goal**: Improve all 4 renderers to surface the enriched IR data, fix visual issues (Mermaid overlap), and add structured output for AI agents.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 7.1 | Markdown: show validation rule codes at keys level | PENDING |
| 7.2 | Markdown: show shared fields section per extension | PENDING |
| 7.3 | Markdown: show required/optional indicators in field tables | PENDING |
| 7.4 | Markdown: show edge descriptions at all level | PENDING |
| 7.5 | Markdown: add summary statistics footer | PENDING |
| 7.6 | Mermaid: switch to `graph LR` for better label readability | PENDING |
| 7.7 | Mermaid: add cross-extension edges (dashed red) | PENDING |
| 7.8 | Mermaid: add `classDef` color styling per extension | PENDING |
| 7.9 | DOT: add cross-extension edges | PENDING |
| 7.10 | DOT: show validation rule count in node labels | PENDING |
| 7.11 | JSON keys: include validation rule codes array | PENDING |
| 7.12 | JSON keys: include edge descriptions | PENDING |
| 7.13 | JSON keys: include shared_fields, collector_count, grammar_count | PENDING |
| 7.14 | JSON summary (none): include cross_edge_count per extension | PENDING |
| 7.15 | Test: markdown keys shows rule codes | PENDING |
| 7.16 | Test: mermaid has graph LR and classDef | PENDING |
| 7.17 | Test: mermaid renders cross-extension edges | PENDING |
| 7.18 | Test: json keys includes validation_rules array | PENDING |
| 7.19 | Test: markdown all shows required indicators | PENDING |
| 7.20 | Test: markdown summary footer contains totals | PENDING |

## Details

### 7.1: Markdown validation rules at keys level

Currently keys level shows rule count in header but no codes. Add a section per extension:

```markdown
**Validation rules**: E007, E008, W041, W042 (19 total)
```

Group by severity prefix (E=error, W=warning, I=info) with count.

### 7.2: Markdown shared fields

```markdown
**Shared fields** (applied to all entities): name, description, tags, status
```

### 7.3: Markdown required indicators

In field tables (detail=All), add a Required column:

```markdown
| Field | Type | Required | Source | Edge | Target |
```

### 7.5: Summary footer

After all extension detail sections, add:

```markdown
## Summary

- **4** extensions, **26** entity kinds, **57** edge types
- **X** validation rules (Y errors, Z warnings)
- **N** cross-extension edges, **M** enhancements
```

### 7.6: Mermaid graph LR

Change `graph TD` to `graph LR` — left-to-right layout reduces vertical label overlap with 4+ nodes.

### 7.7: Mermaid cross-extension edges

Currently only dependency and enhancement edges rendered. Add cross-extension edges:

```mermaid
specforge_software -. "Implements (behavior→feature)" .-> specforge_product
```

### 7.8: Mermaid classDef

Add color-coded node classes matching DOT colors:

```mermaid
classDef product fill:#2ecc71,color:white
classDef software fill:#4a90d9,color:white
classDef governance fill:#e74c3c,color:white
classDef formal fill:#9b59b6,color:white
```

### 7.9: DOT cross-extension edges

Add cross-extension edges as dotted red lines (currently missing from DOT renderer).

### 7.14: JSON summary cross-edge count

Add `cross_edge_count` per extension at none level so agents can quickly see which extensions have cross-boundary relationships.

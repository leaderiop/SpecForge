# Phase 8: MCP & AI Agent Optimization

**Goal**: Optimize the MCP tool for AI agent consumption — change default format to JSON, add structured metadata, and ensure the output is maximally useful for programmatic consumers.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 8.1 | MCP: change default format from markdown to json | PENDING |
| 8.2 | MCP: change default detail from keys to keys (confirm) | PENDING |
| 8.3 | MCP: add `isError: false` to success responses | PENDING |
| 8.4 | MCP registry: update tool description for AI consumption | PENDING |
| 8.5 | JSON: add `metadata` envelope with timestamp, total counts | PENDING |
| 8.6 | Test: MCP default produces valid JSON | PENDING |
| 8.7 | Test: JSON metadata envelope present | PENDING |

## Details

### 8.1: MCP default format

AI agents parse JSON natively. Markdown requires regex/heuristic parsing. Change the default:

```rust
let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("json");
```

CLI default stays markdown (human-facing).

### 8.4: Tool description

Update the registry description to be more specific about what the tool returns and when to use it:

```
"Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes."
```

### 8.5: JSON metadata envelope

Wrap JSON output in a metadata envelope at keys and all levels:

```json
{
  "metadata": {
    "total_extensions": 4,
    "total_entity_kinds": 26,
    "total_edge_types": 57,
    "total_validation_rules": 88,
    "total_cross_edges": 5,
    "total_enhancements": 4
  },
  "extensions": [...],
  "dependencies": [...],
  "enhancements": [...],
  "cross_edges": [...]
}
```

# Phase 1: Mermaid Renderer Rewrite

**Goal**: Replace the flat `graph LR` renderer with the subgraph card design validated through visual iteration.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 1.1 | Switch from `graph LR` to `flowchart TB` | PENDING |
| 1.2 | Render each extension as a subgraph card | PENDING |
| 1.3 | Render single inner node with `<br>` + `───` dividers | PENDING |
| 1.4 | Implement detail tiers (None / Keys / All) | PENDING |
| 1.5 | Add color palette with dynamic assignment | PENDING |
| 1.6 | Add `classDef` per extension for inner card borders | PENDING |
| 1.7 | Add subgraph `style` directives for backgrounds | PENDING |
| 1.8 | Render required dependency edges (solid arrows) | PENDING |
| 1.9 | Render optional dependency edges (dashed arrows) | PENDING |
| 1.10 | Render enhancement edges (dotted arrows) | PENDING |
| 1.11 | Position orphan nodes with `~~~` invisible links | PENDING |
| 1.12 | Add `linkStyle` for colored/typed edges | PENDING |

## Details

### 1.1: Switch to `flowchart TB`

Replace `graph LR` with `flowchart TB`. `flowchart` supports subgraphs with `direction` and `style` directives. TB (top-to-bottom) creates a tree structure where root extensions appear at top and dependents below.

### 1.2: Subgraph cards

Each extension becomes a named subgraph:

```mermaid
subgraph ext_id["  @specforge/name v1.0  "]
    inner_node["...content..."]
end
```

The subgraph title doubles as the card header. Extra spaces in the title string add visual padding.

### 1.3: Single inner node with dividers

The critical design decision — ONE node per subgraph eliminates Mermaid's uncontrollable vertical gaps between multiple inner nodes.

Node content structure:
```
<b>N entities · M edges · K rules</b>     ← stats (bold)
─────────────────────────                ← divider (U+2500)
keyword1 · keyword2 · keyword3          ← entity keywords (regular)
keyword4 · keyword5 · keyword6
─────────────────────────                ← divider
<i>extras line 1</i>                     ← metadata (italic)
<i>extras line 2</i>
```

Extras vary per extension:
- Surface counts: "N CLI cmds · M MCP resources"
- Shared fields: "shared: field1, field2"
- Enhancement summary: "enhances: kind1 +N · kind2 +M"

### 1.4: Detail tiers

**None**: Stats line only (no keywords, no extras).
```
<b>12 entities · 16 edges · 69 rules</b>
```

**Keys** (default): Stats + keywords + extras.
```
<b>12 entities · 16 edges · 69 rules</b><br>───<br>keywords<br>───<br><i>extras</i>
```

**All**: Stats + keywords + full field attribution per entity kind. This may be too dense for Mermaid — consider falling back to Keys with a note "use --format=markdown --fields=all for field detail".

### 1.5: Color palette with dynamic assignment

Replace hardcoded 4-extension `STYLE_CLASSES` with a palette that cycles:

```rust
const PALETTE: &[ExtensionColors] = &[
    // (subgraph_fill, subgraph_stroke, inner_stroke, text_color)
    ("#e8f5e9", "#2e7d32", "#66bb6a", "#1b5e20"),  // green (product)
    ("#e3f2fd", "#1565c0", "#42a5f5", "#0d47a1"),  // blue (software)
    ("#fffde7", "#f9a825", "#ffca28", "#e65100"),  // amber (governance)
    ("#f3e5f5", "#7b1fa2", "#ab47bc", "#4a148c"),  // purple (formal)
    ("#fce4ec", "#c62828", "#ef5350", "#b71c1c"),  // red
    ("#e0f2f1", "#00695c", "#26a69a", "#004d40"),  // teal
];
```

Assign by index: `palette[ext_index % palette.len()]`. No more `name.contains("product")` matching.

### 1.6-1.7: classDef + style directives

For each extension, emit two styling blocks:

```mermaid
%% classDef for inner card node
classDef ext0_card fill:#fff,stroke:#66bb6a,stroke-width:1.5px,color:#333

%% style for subgraph background
style ext0 fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px,color:#1b5e20
```

### 1.8-1.10: Three edge styles

```mermaid
%% Required dependency (solid, colored by source extension)
ext1 -->|"depends on"| ext0

%% Optional dependency (dashed)
ext2 -.->|"optional dep"| ext1

%% Enhancement (thin dotted)
ext3 -.->|"enhances"| ext1
```

Differentiate optional deps from enhancements via `linkStyle` — optional deps use thicker dashed stroke, enhancements use thinner stroke with tighter dash array.

### 1.11: Orphan positioning

Extensions with no dependency or enhancement edges to other extensions need invisible links to prevent floating:

```rust
// Find extensions with no edges
// Connect them to the nearest neighbor via ~~~
```

### 1.12: linkStyle

Track edge index during rendering. Emit `linkStyle N stroke:#color,stroke-width:Xpx` for each edge, matching the source extension's color.

For invisible links: `linkStyle N stroke:none`.

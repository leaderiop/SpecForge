---
id: RES-22
kind: research
title: "Tree-Sitter + Wasm Highlighting — 3-Tier Solution for Plugin Entity Syntax Support"
status: complete
date: 2026-03-03
depends_on: [RES-11a, RES-20, RES-21]
---

# RES-22: Tree-Sitter + Wasm Highlighting

## Executive Summary

Plugin and custom entities (e.g., `threat`, `audit_log`, `capability`) declared via Wasm plugins or `define` blocks are parsed as **ERROR nodes** by the tree-sitter grammar because it only recognizes the 16 built-in entity keywords. This means plugin entities get zero syntax highlighting, no code folding, and broken document symbols in every editor.

The solution is a **3-tier architecture**:

1. **Tier 1 — Generic Grammar Fallback**: A `generic_entity_block` rule in the tree-sitter grammar that matches `identifier id [title] { fields }` patterns not matching built-in keywords. Zero runtime cost, clean AST for all entities.
2. **Tier 2 — Plugin Query Extensions**: Plugins ship `.scm` query patterns in their manifest. The LSP concatenates them with base query files (following the Neovim `;extends` / Elixir `#match?` pattern).
3. **Tier 3 — LSP Semantic Tokens**: Runtime overlay via the LSP 3.16+ semantic tokens protocol. Custom entity keywords, enhanced fields, and cross-plugin references get semantic classification.

Tier 1 is the prerequisite. Tier 3 has the best ROI (already partially implemented). Tier 2 is optional polish for editors that support tree-sitter query composition.

---

## Problem Statement

When a Wasm plugin registers a new entity type (e.g., `threat` via `@specforge/governance`), the tree-sitter grammar has no rule for the `threat` keyword. The parser produces an ERROR node for the entire block:

```
(source_file
  (entity_block ...)       ; ← built-in: parsed correctly
  (ERROR                   ; ← plugin entity: ERROR node
    (UNEXPECTED 't')
    ...))
```

**Consequences:**
- **Zero syntax highlighting**: ERROR nodes receive no captures in `highlights.scm`
- **No code folding**: ERROR nodes are not matched by `folds.scm`
- **Broken document symbols**: LSP outline/symbol search cannot extract entity info from ERROR nodes
- **Degraded error recovery**: Subsequent valid blocks may also fail to parse due to cascading errors
- **Poor DX for plugin authors**: The most common extension scenario (adding entity types) produces the worst editing experience

This affects every editor: Neovim, Zed, Helix, Emacs (tree-sitter mode), and VS Code (via any tree-sitter extension).

---

## Tier 1: Generic Grammar Fallback

### Design

Add a `generic_entity_block` rule to `grammar.js` that matches any identifier followed by the standard entity block pattern:

```javascript
generic_entity_block: $ => seq(
  field('kind', $.identifier),        // unknown keyword
  field('name', $.entity_name),       // entity ID
  optional(field('title', $.string)), // optional title
  '{',
  repeat($._block_item),             // fields, verify, scenario, etc.
  '}'
),
```

This rule has **lower priority** than all built-in entity rules (tree-sitter's `prec()` ensures built-in keywords win). When no built-in keyword matches, the generic rule catches the block and produces a clean AST node:

```
(source_file
  (generic_entity_block
    kind: (identifier)
    name: (entity_name)
    title: (string)
    (field_entry ...)
    ...))
```

### Impact

- **Immediate syntax highlighting**: `highlights.scm` captures `kind` as `@keyword`, `name` as `@constant`
- **Code folding**: `folds.scm` captures `generic_entity_block` as `@fold`
- **Document symbols**: LSP can extract entity info from clean AST nodes
- **Error recovery**: No cascading parse failures from unknown keywords

### Effort: Low (grammar change + query file updates)

### Risk: Minimal

The generic rule only matches when no built-in rule matches. It cannot cause false positives for built-in entities. The resolver validates whether the `kind` field is registered by an installed plugin — unregistered kinds produce a diagnostic.

---

## Tier 2: Plugin Query Extensions

### Design

Plugins declare `.scm` query patterns in their manifest:

```json
{
  "package": "@specforge/governance",
  "queryExtensions": [
    {
      "kind": "highlights",
      "patterns": "(generic_entity_block kind: (identifier) @keyword (#match? @keyword \"^(constraint|decision|failure_mode)$\"))"
    },
    {
      "kind": "folds",
      "patterns": "(generic_entity_block kind: (identifier) @keyword (#match? @keyword \"^(constraint|decision|failure_mode)$\")) @fold"
    }
  ]
}
```

The LSP composes final query files by **string concatenation**: base queries first, plugin extensions appended in plugin load order. This follows the established pattern:

- **Neovim**: `;extends` directive in query files
- **Elixir/HCL**: `#match?` predicates for keyword variants
- **Zed**: Wasm grammars ship their own query files

### Composition Algorithm

```
final_highlights = base_highlights.scm
for plugin in plugins (topological order):
    if plugin.queryExtensions contains "highlights":
        final_highlights += "\n" + plugin.queryExtensions["highlights"].patterns
```

### Validation

Plugin query patterns are validated at load time:
1. Parse with `tree_sitter::Query::new()` — invalid patterns produce a warning diagnostic
2. Invalid patterns are **skipped**, not fatal — plugin loading continues
3. Capture names are validated against the tree-sitter standard set

### Effort: Medium

Requires manifest schema extension, query composition in LSP, and editor-specific delivery (Neovim needs `;extends` files, VS Code ignores tree-sitter queries).

---

## Tier 3: LSP Semantic Tokens

### Design

The LSP already provides semantic tokens (rated 9/10 in RES-20). Extend it to classify:

1. **Custom entity keywords**: Plugin entity keywords (`constraint`, `decision`, `failure_mode`, etc.) → `keyword` token type
2. **Custom entity IDs**: Entity IDs → classified by entity kind (same as built-in entities)
3. **Enhanced fields**: Fields added by entity enhancements → `property` token type
4. **Cross-plugin references**: References to entities from other plugins → classified by target kind

Implementation in `semantic_tokens.rs`:

```rust
// During token classification:
if let Some(kind) = plugin_registry.entity_kind(&keyword) {
    // Plugin entity keyword → keyword token
    tokens.push(SemanticToken {
        token_type: KEYWORD,
        ..
    });
}
```

### Editor Support

- **VS Code**: Full semantic token support (primary path)
- **Neovim**: Supported via `vim.lsp.semantic_tokens`
- **Zed**: Semantic token support (in progress)
- **Helix**: Semantic token support since 23.10

### Effort: Medium (extend existing implementation)

### Impact: High (works in every LSP-capable editor)

---

## Editor Landscape

| Editor | Tree-sitter | Query Extensions | Semantic Tokens | Best Tier |
|--------|-------------|-----------------|-----------------|-----------|
| Neovim | Native | `;extends` mechanism | Yes | 1 + 2 + 3 |
| Zed | Native (Wasm grammars) | Query files in extensions | In progress | 1 + 2 |
| Helix | Native | No extension mechanism | Yes (23.10+) | 1 + 3 |
| Emacs (ts) | Native (29+) | Manual query file config | Yes | 1 + 3 |
| VS Code | TextMate only | N/A | Yes | 3 only |

**Key insight**: VS Code does not use tree-sitter at all. Tier 3 (semantic tokens) is the **only** way to highlight plugin entities in VS Code. For tree-sitter editors, Tier 1 provides the baseline and Tier 2/3 refine it.

---

## Technical Details

### Tree-sitter WasmStore API

Tree-sitter 0.25 includes a production-ready `WasmStore` API for loading Wasm-compiled parsers. However, **this is NOT needed** for our use case. We are not dynamically loading grammar parsers — we are extending query files, which are plain text strings. The `WasmStore` API is relevant only for the hypothetical future scenario of loading entire grammar parsers at runtime.

### Query Composition via String Concatenation

Tree-sitter queries compose naturally via string concatenation. A query file is simply a sequence of S-expression patterns. Appending more patterns to the end adds more matches without affecting existing ones. This is the same mechanism Neovim's `;extends` uses internally.

```
; Base highlights.scm
(entity_block keyword: (keyword) @keyword)
(entity_name) @constant

; Plugin extension (appended)
(generic_entity_block kind: (identifier) @keyword
  (#match? @keyword "^(constraint|decision|failure_mode)$"))
```

### Wasmtime Engine Sharing with Extism

The Extism runtime already uses wasmtime under the hood. If we ever needed to load tree-sitter Wasm parsers (Tier 2 future), the wasmtime engine instance could be shared between Extism plugin execution and tree-sitter parser loading. This avoids duplicating the ~2MB wasmtime compilation overhead.

Currently this is **not needed** — noted for future reference only.

---

## Effort vs Impact Matrix

| Tier | Effort | Impact | Dependencies | Priority |
|------|--------|--------|-------------|----------|
| **1: Generic Grammar Fallback** | Low | High | None | **P0 — Prerequisite** |
| **2: Plugin Query Extensions** | Medium | Medium | Tier 1, manifest schema | P2 — Optional polish |
| **3: LSP Semantic Tokens** | Medium | High | Plugin registry | **P1 — Best ROI** |

---

## Recommendations

1. **Implement Tier 1 first** — it is a prerequisite for all other tiers and has the highest impact-to-effort ratio. A grammar change + query file updates gives every editor baseline support for plugin entities.

2. **Implement Tier 3 next** — the semantic token infrastructure already exists (RES-20 rated it 9/10). Extending it to cover custom entity keywords is incremental work with high payoff, and it's the **only** way to highlight plugin entities in VS Code.

3. **Defer Tier 2** — query extensions are polish for tree-sitter-native editors. They provide finer-grained highlighting than Tier 1 alone, but Tier 1 + Tier 3 already covers the critical path. Implement when the plugin ecosystem matures and authors want editor-specific customization.

4. **Ship a `.tmLanguage.json`** — for VS Code users without the LSP running, a TextMate grammar provides basic keyword highlighting. This is orthogonal to the 3-tier system and is standard practice for VS Code extensions.

---

## What This Does NOT Need

- **No dynamic grammar compilation**: We are NOT compiling new grammars at runtime. The generic grammar fallback handles all unknown keywords statically.
- **No tree-sitter Wasm parser runtime**: We are NOT loading tree-sitter parsers from Wasm. The `WasmStore` API is irrelevant for our use case.
- **No external scanners for plugin keywords**: External scanners add complexity and platform-specific build requirements. The generic rule approach is simpler and portable.
- **No editor-specific plugins per entity type**: One generic rule covers all plugin entities. Per-entity-type rules would be unmaintainable.

---

## Cross-References

- **RES-11a** — Core compiler architecture, tree-sitter grammar design
- **RES-20** — LSP implementation review, semantic tokens rated 9/10
- **RES-21** — Wasm/Extism plugin runtime, plugin manifest schema
- `spec/behaviors/parsing.spec` — `parse_generic_entity_blocks`, `provide_syntax_highlighting_queries`, `provide_code_folding_queries`
- `spec/behaviors/wasm.spec` — `provide_plugin_query_extensions`, `compose_query_files_from_plugins`
- `spec/behaviors/lsp.spec` — `provide_semantic_tokens`
- `spec/types/wasm.spec` — `QueryExtension`, `QueryFileKind`, `WasmPluginManifest`

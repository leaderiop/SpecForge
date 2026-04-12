# Phase 2: Other Renderer Updates

**Goal**: Propagate the `optional` dependency field to Markdown, DOT, and JSON renderers.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 2.1 | Markdown: show optional indicator on dependencies | PENDING |
| 2.2 | DOT: render optional deps as dashed edges | PENDING |
| 2.3 | JSON: include `optional` field in dependency objects | PENDING |
| 2.4 | Markdown: show dependency direction (DAG arrow) | PENDING |
| 2.5 | DOT: use dynamic color palette (match Mermaid) | PENDING |

## Details

### 2.1: Markdown optional indicator

**File**: `crates/specforge-emitter/src/outline/markdown.rs` (~line 51)

Change dependency rendering from:
```
- @specforge/software → @specforge/product (^1.0)
```
To:
```
- @specforge/software → @specforge/product (^1.0)
- @specforge/governance → @specforge/product (^1.0, optional)
```

Add `(optional)` suffix when `dep.optional == true`.

### 2.2: DOT optional deps as dashed

**File**: `crates/specforge-emitter/src/outline/dot.rs` (~line 55-63)

Required deps: `style=solid` (current default).
Optional deps: `style=dashed, color="#999999"`.

```rust
let style = if dep.optional { "dashed" } else { "solid" };
writeln!(out, "    {} -> {} [label=\"depends {}\", style={}];",
    from_id, to_id, dep.version, style)
```

### 2.3: JSON optional field

**File**: `crates/specforge-emitter/src/outline/json.rs`

The `OutlineDependency` is already serialized via `serde_json::json!`. Since `optional: bool` is on the struct and `Serialize` is derived, it will automatically appear in JSON output. No code change needed — just verify in tests.

### 2.4: Markdown dependency direction

Currently the Dependencies section uses `→` which is correct. Verify that after removing product→software, the section shows the clean DAG:

```markdown
## Dependencies

- @specforge/software → @specforge/product (^1.0)
- @specforge/governance → @specforge/software (^1.0)
- @specforge/governance → @specforge/product (^1.0, optional)
- @specforge/formal → @specforge/software (^1.0)
```

### 2.5: DOT dynamic color palette

**File**: `crates/specforge-emitter/src/outline/dot.rs`

Replace the hardcoded `COLORS` array (matched by `name.contains()`) with index-based palette assignment matching the Mermaid renderer. Both renderers should use the same color palette for consistency.

Extract the palette into `mod.rs` or a shared constant so both renderers reference it.

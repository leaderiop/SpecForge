# Phase 2: Dependency Visibility Modes

**Goal**: Add `--deps=direct|effective|full` flag to the outline command. Each renderer filters dependencies based on the selected mode. Sensible defaults per format.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 2.1 | Add `DependencyDepth` enum to outline IR | PENDING |
| 2.2 | Add `deps` field to `OutlineOptions` | PENDING |
| 2.3 | Add `--deps` CLI flag to Outline command | PENDING |
| 2.4 | Implement dependency filtering function | PENDING |
| 2.5 | Update renderers to filter deps before rendering | PENDING |
| 2.6 | Update MCP tool to accept `deps` parameter | PENDING |
| 2.7 | Set format-specific defaults | PENDING |

## Details

### 2.1: DependencyDepth enum

**File**: `crates/specforge-emitter/src/outline/mod.rs`

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyDepth {
    /// Only explicitly declared peer_dependencies
    #[default]
    Direct,
    /// Direct + transitive deps that reference kinds from the target extension
    Effective,
    /// All transitive deps, including unused
    Full,
}
```

### 2.2: Update OutlineOptions

**File**: `crates/specforge-emitter/src/outline/mod.rs`

```rust
pub struct OutlineOptions {
    pub format: OutlineFormat,
    pub detail: OutlineDetail,
    pub deps: DependencyDepth,
}
```

### 2.3: CLI flag

**File**: `crates/specforge-cli/src/main.rs`

Add to Outline command variant:
```rust
Outline {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long, default_value = "markdown")]
    format: String,
    #[arg(long, default_value = "keys")]
    fields: String,
    /// Dependency visibility: direct, effective, full
    #[arg(long, default_value = "direct")]
    deps: String,
}
```

**File**: `crates/specforge-cli/src/outline.rs`

Parse `deps` string into `DependencyDepth` enum.

### 2.4: Dependency filtering function

**File**: `crates/specforge-emitter/src/outline/mod.rs` or a new `filter.rs`

```rust
pub fn filter_dependencies(deps: &[OutlineDependency], depth: DependencyDepth) -> Vec<&OutlineDependency> {
    match depth {
        DependencyDepth::Direct => deps.iter().filter(|d| d.kind == DependencyKind::Direct).collect(),
        DependencyDepth::Effective => deps.iter().filter(|d| d.kind != DependencyKind::Transitive).collect(),
        DependencyDepth::Full => deps.iter().collect(),
    }
}
```

| Mode | Shows DependencyKind::Direct | Shows Effective | Shows Transitive |
|------|------------------------------|-----------------|------------------|
| direct | yes | no | no |
| effective | yes | yes | no |
| full | yes | yes | yes |

### 2.5: Renderer updates

Each renderer calls `filter_dependencies()` before rendering the dependency section. The IR (`OutlineIntermediate`) always contains ALL dependencies (direct + transitive). Filtering is a presentation concern.

For **markdown**: effective deps get a `(transitive, via X)` label. Full mode adds `(transitive, unused)` for pure transitive deps.

For **mermaid/dot**: effective deps rendered with a distinct edge style (e.g., thinner dashed, different color). Full mode adds very light/gray edges for unused transitive.

For **json**: all deps included with their `kind` field regardless of mode (JSON consumers do their own filtering). The `deps` mode is noted in a top-level `"dependency_depth"` field.

### 2.6: MCP tool

**File**: `crates/specforge-mcp/src/tools/outline.rs`

Add `deps` parameter to the MCP tool's input schema:
```json
{
  "name": "deps",
  "type": "string",
  "enum": ["direct", "effective", "full"],
  "default": "direct",
  "description": "Dependency visibility: direct (declared only), effective (direct + used transitive), full (all transitive)"
}
```

### 2.7: Format-specific defaults

| Format | Default `deps` mode | Rationale |
|--------|---------------------|-----------|
| markdown | `direct` | Clean readable output |
| mermaid | `direct` | Clean graph, no clutter |
| dot | `direct` | Clean graph |
| json | `full` | Machine consumers want everything, filter themselves |

If the user explicitly passes `--deps=X`, it overrides the default for any format.

Implementation: In `outline.rs` CLI handler, if `deps` flag was not explicitly provided, apply the format-specific default.

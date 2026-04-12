# Phase 1: Transitive Dependency Computation

**Goal**: Enrich the outline builder to compute transitive dependencies with optional propagation and effective-use detection.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 1.1 | Add `DependencyKind` enum to outline IR | PENDING |
| 1.2 | Add `kind` field to `OutlineDependency` | PENDING |
| 1.3 | Compute transitive closure in builder | PENDING |
| 1.4 | Implement optional propagation (weakest link) | PENDING |
| 1.5 | Detect "effective" transitive deps (kind references) | PENDING |
| 1.6 | Skip self-loops in transitive computation | PENDING |
| 1.7 | Update JSON serialization for DependencyKind | PENDING |
| 1.8 | Verify builder produces correct transitive deps for all 4 extensions | PENDING |

## Details

### 1.1: DependencyKind enum

**File**: `crates/specforge-emitter/src/outline/mod.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    /// Explicitly declared in peerDependencies
    Direct,
    /// Transitive AND the extension references kinds from the target
    Effective,
    /// Pure transitive — no direct kind references
    Transitive,
}
```

### 1.2: Update OutlineDependency

**File**: `crates/specforge-emitter/src/outline/mod.rs`

Add `kind: DependencyKind` to `OutlineDependency`:
```rust
pub struct OutlineDependency {
    pub from: String,
    pub to: String,
    pub version: String,
    pub optional: bool,
    pub kind: DependencyKind,
}
```

All existing direct dependencies get `kind: DependencyKind::Direct`.

### 1.3: Transitive closure

**File**: `crates/specforge-emitter/src/outline/build.rs`

After collecting all direct dependencies, compute transitive closure:

```
Algorithm:
1. Start with direct deps: {(A→B, required, direct)}
2. For each pair (A→B) and (B→C) where A != C:
   - Add (A→C, weakest(A→B.optional, B→C.optional), transitive)
3. Repeat until no new deps added (fixed point)
4. For transitive deps, detect if "effective" (step 1.5)
```

Version for transitive deps: use the target's version from the direct dep that reaches it (the last hop's version, since that's what the target actually declares).

### 1.4: Optional propagation

If A →(required) B →(optional) C, then A →(optional) C transitively.
If A →(optional) B →(required) C, then A →(optional) C transitively.
If A →(required) B →(required) C, then A →(required) C transitively.

Rule: `transitive_optional = any link in chain is optional`.

For multi-hop chains, propagate through shortest path. If multiple paths exist with different optionality, use the "strongest" (required wins — if ANY path is fully required, the transitive dep is required).

### 1.5: Effective detection

A transitive dep A → C is "effective" (not just theoretical) if extension A references any entity kind owned by extension C. Detection sources:

1. **Edge types**: A's `edgeTypes` where `sourceKind` or `targetKind` is owned by C
2. **Entity enhancement targets**: A's `entityEnhancements` where `targetKind` is owned by C
3. **Field target_kind**: A's entity kind fields where `targetKind` is owned by C

Build a `kinds_owned_by: HashMap<String, String>` (kind → extension name) index. For each transitive dep A → C, check if any of A's edges/enhancements/fields reference a kind in C's namespace.

If references exist → `DependencyKind::Effective`
If no references → `DependencyKind::Transitive`

### 1.6: Self-loop prevention

During transitive closure, skip any computed dep where `from == to`. This prevents degenerate cycles in edge cases.

### 1.7: JSON serialization

`DependencyKind` serializes as lowercase string: `"direct"`, `"effective"`, `"transitive"`. No custom serializer needed — `#[serde(rename_all = "lowercase")]` handles it.

### 1.8: Expected transitive deps

After Phase 0 manifest cleanup and Phase 1 transitive computation:

| From | To | Optional | Kind | Why |
|------|----|----------|------|-----|
| software → product | required | direct | declared peer_dependency |
| governance → software | required | direct | declared peer_dependency |
| formal → software | required | direct | declared peer_dependency |
| governance → product | required | **effective** | transitive via software; governance has edges targeting product's `feature` kind |
| formal → product | required | **effective** | transitive via software; formal enhances software entities which reference product kinds (TBD — verify) |

If formal doesn't reference any product kinds directly, its transitive dep on product would be `Transitive` not `Effective`.

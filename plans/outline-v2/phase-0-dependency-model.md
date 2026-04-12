# Phase 0: Dependency Model Fixes

**Goal**: Fix the two dependency model bugs — circular product↔software dep and missing `optional` field on `PeerDependency`.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 0.1 | Add `optional: bool` to `PeerDependency` struct | PENDING |
| 0.2 | Add `optional: bool` to `OutlineDependency` IR type | PENDING |
| 0.3 | Update outline builder to populate `optional` | PENDING |
| 0.4 | Remove product's peer_dependency on software | PENDING |
| 0.5 | Verify governance manifest parses with optional=true | PENDING |
| 0.6 | Test: outline dependency has optional flag | PENDING |
| 0.7 | Test: product has zero peer dependencies | PENDING |
| 0.8 | Test: no circular dependencies in outline | PENDING |

## Details

### 0.1: Add `optional: bool` to `PeerDependency`

**File**: `crates/specforge-registry/src/manifest/types.rs:179-182`

```rust
pub struct PeerDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub optional: bool,
}
```

`#[serde(default)]` ensures backward compatibility — existing manifests without `optional` deserialize as `false`.

### 0.2: Add `optional: bool` to `OutlineDependency`

**File**: `crates/specforge-emitter/src/outline/mod.rs:146-150`

```rust
pub struct OutlineDependency {
    pub from: String,
    pub to: String,
    pub version: String,
    pub optional: bool,
}
```

### 0.3: Update builder to populate `optional`

**File**: `crates/specforge-emitter/src/outline/build.rs:141-148`

```rust
for dep in &m.peer_dependencies {
    dependencies.push(OutlineDependency {
        from: m.name.clone(),
        to: dep.name.clone(),
        version: dep.version.clone(),
        optional: dep.optional,
    });
}
```

### 0.4: Remove product's peer_dependency on software

**File**: `extensions/product/manifest.json:547-549`

Change from:
```json
"peerDependencies": [
  { "name": "@specforge/software", "version": "^1.0" }
]
```

To:
```json
"peerDependencies": []
```

**Justification**: Product's 16 edge types all target product-owned kinds. No product edge type targets a software entity kind. The `Implements` edge (behavior→feature) is owned by software, not product. Product is the standalone root of the dependency DAG.

### 0.5: Verify governance `optional` parses

Governance manifest declares `{ "name": "@specforge/product", "version": "^1.0", "optional": true }`. After 0.1, this should deserialize as `PeerDependency { optional: true }`. Verify via unit test or integration test.

### 0.6-0.8: Tests

**0.6** — Load all manifests, build outline, assert governance→product dependency has `optional == true`.

**0.7** — Load product manifest alone, assert `outline.dependencies.is_empty()`.

**0.8** — Load all manifests, assert no `(from, to)` pair has a reverse `(to, from)` — i.e., the dependency graph is a DAG with no cycles.

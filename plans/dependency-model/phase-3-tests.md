# Phase 3: Tests

**Goal**: Verify the full dependency model — manifest cleanup, transitive computation, visibility modes — with targeted tests following TDD vertical slices.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 3.1 | Test: product has zero peer_dependencies (standalone root) | PENDING |
| 3.2 | Test: governance has exactly one direct dep (software only) | PENDING |
| 3.3 | Test: no reverse dependencies in the DAG | PENDING |
| 3.4 | Test: software enhances product's library with ports_defined, ports_consumed | PENDING |
| 3.5 | Test: software enhances product's roadmap with behaviors | PENDING |
| 3.6 | Test: transitive closure computes governance → product | PENDING |
| 3.7 | Test: transitive governance → product is effective (references feature kind) | PENDING |
| 3.8 | Test: optional propagation — if any link optional, transitive is optional | PENDING |
| 3.9 | Test: `--deps=direct` filters out transitive deps | PENDING |
| 3.10 | Test: `--deps=effective` shows direct + effective, hides pure transitive | PENDING |

## Details

### 3.1: Product standalone root

```rust
#[test]
fn product_has_zero_peer_dependencies() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let product_deps: Vec<_> = outline.dependencies.iter()
        .filter(|d| d.from == "@specforge/product")
        .collect();
    assert!(product_deps.is_empty(), "product is the root — zero deps");
}
```

### 3.2: Governance single direct dep

```rust
#[test]
fn governance_has_only_software_as_direct_dep() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let gov_direct: Vec<_> = outline.dependencies.iter()
        .filter(|d| d.from == "@specforge/governance" && d.kind == DependencyKind::Direct)
        .collect();
    assert_eq!(gov_direct.len(), 1);
    assert_eq!(gov_direct[0].to, "@specforge/software");
}
```

### 3.3: No reverse deps

```rust
#[test]
fn no_reverse_dependencies_in_dag() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let direct: Vec<_> = outline.dependencies.iter()
        .filter(|d| d.kind == DependencyKind::Direct)
        .collect();
    for dep in &direct {
        let reverse = direct.iter().any(|d| d.from == dep.to && d.to == dep.from);
        assert!(!reverse, "reverse dep: {} <-> {}", dep.from, dep.to);
    }
}
```

### 3.4–3.5: Software enhances product's library and roadmap

Verify that after moving the fields, the outline builder detects:
- software enhances product's `library` (+2 fields: ports_defined, ports_consumed)
- software enhances product's `roadmap` (+1 field: behaviors)

Check via `outline.enhancements` entries.

### 3.6: Transitive closure

```rust
#[test]
fn transitive_governance_to_product_computed() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let gov_to_prod = outline.dependencies.iter()
        .find(|d| d.from == "@specforge/governance" && d.to == "@specforge/product");
    assert!(gov_to_prod.is_some(), "governance → product should exist as transitive dep");
    assert_ne!(gov_to_prod.unwrap().kind, DependencyKind::Direct, "should be transitive, not direct");
}
```

### 3.7: Effective detection

```rust
#[test]
fn governance_product_transitive_is_effective() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let gov_to_prod = outline.dependencies.iter()
        .find(|d| d.from == "@specforge/governance" && d.to == "@specforge/product")
        .expect("governance → product transitive dep should exist");
    assert_eq!(gov_to_prod.kind, DependencyKind::Effective,
        "governance references product's 'feature' kind via cross-extension edges");
}
```

### 3.8: Optional propagation

Test with synthetic manifests:
- A → B (required), B → C (optional) → A → C should be optional
- A → B (required), B → C (required) → A → C should be required
- A → B (optional), B → C (required) → A → C should be optional

### 3.9–3.10: Visibility mode filtering

```rust
#[test]
fn deps_direct_filters_transitive() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let filtered = filter_dependencies(&outline.dependencies, DependencyDepth::Direct);
    assert!(filtered.iter().all(|d| d.kind == DependencyKind::Direct));
    // governance → product should NOT appear
    assert!(!filtered.iter().any(|d| d.from == "@specforge/governance" && d.to == "@specforge/product"));
}

#[test]
fn deps_effective_shows_used_transitive() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let filtered = filter_dependencies(&outline.dependencies, DependencyDepth::Effective);
    // governance → product SHOULD appear (effective)
    assert!(filtered.iter().any(|d| d.from == "@specforge/governance" && d.to == "@specforge/product"));
    // No pure transitive deps should appear
    assert!(!filtered.iter().any(|d| d.kind == DependencyKind::Transitive));
}
```

## Existing tests to update

| Test | Change needed |
|------|--------------|
| `peer_dependencies_mapped_to_outline_dependencies` | Update count, add kind assertions |
| `governance_product_dependency_is_optional` | Remove (gov no longer has direct dep on product) |
| `product_has_no_required_dependencies` | Simplify to `product_has_zero_peer_dependencies` |
| `no_required_circular_dependencies` | Simplify to `no_reverse_dependencies_in_dag` |
| `json_dependencies_include_optional_field` | Update comment (product→software optional no longer exists) |
| `markdown_shows_optional_dependency_indicator` | May need updating depending on whether any optional deps remain |
| `mermaid_renders_optional_dep_as_dashed_arrow` | May need updating |

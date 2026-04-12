# Phase 3: Tests

**Goal**: Update existing tests and add new tests for the dependency model fixes and Mermaid rewrite. TDD vertical slices — each test written immediately before the code that makes it pass.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 3.1 | Test: governance→product dependency has optional=true | PENDING |
| 3.2 | Test: product has zero peer dependencies (after manifest fix) | PENDING |
| 3.3 | Test: no circular dependencies in outline DAG | PENDING |
| 3.4 | Test: mermaid produces flowchart TB with subgraph | PENDING |
| 3.5 | Test: mermaid card contains stats + divider + keywords | PENDING |
| 3.6 | Test: mermaid renders required dep as solid arrow | PENDING |
| 3.7 | Test: mermaid renders optional dep as dashed arrow | PENDING |
| 3.8 | Test: mermaid renders enhancement as dotted arrow | PENDING |
| 3.9 | Test: JSON dependency objects include optional field | PENDING |
| 3.10 | Test: markdown dependency shows optional indicator | PENDING |

## Details

### 3.1: Optional dependency flag

```rust
#[test]
fn governance_product_dependency_is_optional() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let gov_to_prod = outline.dependencies.iter()
        .find(|d| d.from == "@specforge/governance" && d.to == "@specforge/product");
    assert!(gov_to_prod.is_some());
    assert!(gov_to_prod.unwrap().optional, "governance→product should be optional");
}
```

### 3.2: Product standalone

```rust
#[test]
fn product_has_no_peer_dependencies() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let product_deps: Vec<_> = outline.dependencies.iter()
        .filter(|d| d.from == "@specforge/product")
        .collect();
    assert!(product_deps.is_empty(), "product should be standalone root with no deps");
}
```

### 3.3: DAG validation

```rust
#[test]
fn dependency_graph_is_acyclic() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    for dep in &outline.dependencies {
        let reverse = outline.dependencies.iter()
            .any(|d| d.from == dep.to && d.to == dep.from);
        assert!(!reverse, "circular dependency: {} ↔ {}", dep.from, dep.to);
    }
}
```

### 3.4: Mermaid flowchart TB + subgraph

Update existing `mermaid_produces_graph_lr_with_styling` test:

```rust
#[test]
fn mermaid_produces_flowchart_tb_with_subgraphs() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let opts = OutlineOptions { format: OutlineFormat::Mermaid, detail: OutlineDetail::Keys };
    let output = render(&outline, &opts);
    assert!(output.starts_with("flowchart TB"));
    assert!(output.contains("subgraph"));
    assert!(output.contains("end"));
}
```

### 3.5: Card content structure

```rust
#[test]
fn mermaid_card_has_stats_divider_keywords() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let opts = OutlineOptions { format: OutlineFormat::Mermaid, detail: OutlineDetail::Keys };
    let output = render(&outline, &opts);
    // Stats line with bold
    assert!(output.contains("<b>"));
    // Unicode divider
    assert!(output.contains("─────"));
    // Entity keywords (product has "journey")
    assert!(output.contains("journey"));
}
```

### 3.6-3.8: Edge styles

Verify edge rendering by checking for Mermaid syntax patterns:
- Required: `-->|"depends on"|` (solid arrow with label)
- Optional: `-.->|"optional dep"|` (dashed arrow with label)
- Enhancement: `-.->|"enhances` (dotted arrow with label)

### 3.9: JSON optional field

```rust
#[test]
fn json_dependencies_include_optional_field() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let opts = OutlineOptions { format: OutlineFormat::Json, detail: OutlineDetail::Keys };
    let output = render(&outline, &opts);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let deps = parsed["dependencies"].as_array().unwrap();
    // At least one dep should have optional=true (governance→product)
    assert!(deps.iter().any(|d| d["optional"] == true));
}
```

### 3.10: Markdown optional indicator

```rust
#[test]
fn markdown_shows_optional_dependency_indicator() {
    let manifests = load_all_manifests();
    let outline = OutlineIntermediate_from_manifests(&manifests);
    let opts = OutlineOptions { format: OutlineFormat::Markdown, detail: OutlineDetail::Keys };
    let output = render(&outline, &opts);
    assert!(output.contains("optional"), "optional deps should be marked in markdown");
}
```

## Existing tests to update

These tests currently pass but will need updating:

| Test | Current assertion | New assertion |
|------|-------------------|---------------|
| `mermaid_produces_graph_lr_with_styling` | `starts_with("graph LR")` | `starts_with("flowchart TB")` + `contains("subgraph")` |
| `mermaid_renders_cross_extension_edges` | `contains("cross:")` | May change if cross-edges are rendered differently in subgraph layout |
| `peer_dependencies_mapped_to_outline_dependencies` | `>= 4` deps | May decrease after removing product→software |
| `mermaid_all_includes_entity_names` | `contains("feature")` | Same check but in subgraph card context |

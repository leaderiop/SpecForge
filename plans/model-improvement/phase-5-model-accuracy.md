# Phase 5: Model Builder Accuracy

**Status**: COMPLETE (2026-04-12)

**Goal**: Fix edge ownership accounting, total edge count mismatches, and entity count
inaccuracies in the `specforge model` output so the rendered LDM header statistics
match the actual manifest declarations.

**Depends on**: Phases 1-4 (manifest data must be correct before the builder can report it correctly)

**Targets axes**: 1 (Domain Completeness), 2 (Relationship Correctness), 5 (Cross-Extension Coherence), 6 (AI Agent Usability)

---

## Issue 5.1: Edge Ownership Accounting in Model Header

### Problem

The extension summary table in model output attributes edge types to the wrong extension:

```
| Extension           | Version | Entity Kinds | Edge Types |
|---------------------|---------|-------------|------------|
| @specforge/formal   | 1.0.0   | 6           | 5          |  <-- should be 11
| @specforge/software | 1.0.0   | 6           | 16         |  <-- inflated by 6 misattributed formal edges
```

Formal declares 11 edge types in its manifest, but 6 cross-extension edges injected via
`entityEnhancements` (RequiresCondition, EnsuresCondition, MaintainsCondition, Satisfies,
FollowsProtocol, ParticipatesIn) get their `source_extension` set to the **target entity's
owner** rather than the **enhancement's declaring extension**.

### Root Cause Analysis

The data flow is:

1. **Manifest loading** (`populate.rs:register_edge_types`) -- Edges declared in
   `manifest.edge_types[]` get `source_extension: manifest.name` (correct).

2. **Entity enhancements** (`populate.rs:apply_entity_enhancements`) -- Enhancement
   fields are registered into `FieldRegistry` with the correct `source_extension` from
   `enhancement.source_extension`. However, entity enhancements can introduce reference
   fields with `edge` + `target_kind` that imply new edge relationships. These fields
   are stored correctly in `FieldRegistry`.

3. **Implicit edge registration** (`populate.rs:register_implicit_edges`) -- This runs
   **per manifest** and only checks `kind.fields` from `manifest.entity_kinds[].fields`.
   It does NOT scan enhancement-contributed fields. So if `@specforge/formal` contributes
   a field with `edge: "RequiresCondition"` to behavior (owned by software), and that
   edge is not in formal's `edge_types[]`, the implicit edge is never registered.

4. **Schema generation** (`schema.rs:generate_schema`) -- `SchemaEdgeType` entries get
   `source_extension` from `EdgeRegistryEntry.source_extension`. If the edge was
   registered by the implicit edge pass of the wrong manifest, or was inferred from
   field scanning (lines 303-333 in schema.rs), it may get wrong attribution.

5. **Model builder** (`build.rs:build_extensions`, lines 150-154) -- Counts edges per
   extension using `edge.source_extension`. If step 4 produced wrong attribution, the
   counts are wrong.

The bug could manifest in two ways:

- **Way A**: The edge IS in formal's `edge_types[]` with correct `source_extension`, but
  the field-level inference loop (schema.rs lines 303-333) overwrites or conflicts.
- **Way B**: The edge is NOT in formal's `edge_types[]` (declared only implicitly via
  enhancement fields), and the implicit edge pass in `register_implicit_edges` never
  picks it up because it only iterates `manifest.entity_kinds[].fields`, not
  enhancement-contributed fields.

### Investigation Steps

- [ ] **5.1.1**: Enumerate all 11 formal edge types from the formal manifest JSON
      (`spec/extensions/formal/`) and check which are in `edge_types[]` vs which are
      only implied by enhancement field `edge` attributes.

- [ ] **5.1.2**: Run `specforge schema` and pipe to `jq '.edge_types[] | select(.source_extension == "@specforge/formal")'` to see how many edges the schema attributes to formal today.

- [ ] **5.1.3**: Run `specforge schema` and pipe to `jq '.edge_types | length'` to get the total edge count and compare with the model header total.

- [ ] **5.1.4**: For each misattributed edge, trace whether it exists in `EdgeRegistry` (from `register_edge_types`) or was created by `register_implicit_edges` or by the field-inference loop in `generate_schema`.

### Fix Plan

The fix depends on which root cause applies. Both are addressed:

#### Fix A: Ensure `register_implicit_edges` covers enhancement-contributed fields

Currently `register_implicit_edges` in `populate.rs` only iterates `manifest.entity_kinds[].fields`. It misses fields contributed by entity enhancements from OTHER extensions.

**Change**: After `apply_entity_enhancements` runs, do a second pass to register implicit edges from enhancement-contributed fields.

**File**: `crates/specforge-registry/src/compilation/populate.rs`

```rust
// In populate_registries(), AFTER apply_entity_enhancements:

// Register implicit edges from enhancement-contributed fields.
// Enhancement fields may introduce edge+target_kind references that
// need to be in the EdgeRegistry, attributed to the enhancing extension.
for (ext_name, enhancement) in &all_enhancements {
    for field in &enhancement.fields {
        if let Some(ref edge_label) = field.edge {
            if !edge_reg.contains(edge_label) {
                edge_reg.register(EdgeRegistryEntry {
                    label: edge_label.clone(),
                    source_kind: Some(enhancement.target_kind.clone()),
                    target_kind: field.target_kind.clone(),
                    source_extension: ext_name.clone(),
                    edge_style: None,
                    edge_color: None,
                    edge_arrowhead: None,
                });
            }
        }
    }
}
```

#### Fix B: Prevent field-inference loop from overwriting source_extension

The field-inference loop in `generate_schema()` (schema.rs lines 303-333) infers
source/target kinds for edges that have `None` values. It does NOT touch
`source_extension`. Verify this is indeed the case -- it should be safe as-is since
it only mutates `source_kinds` and `target_kinds`.

**Verification**: Read `generate_schema()` lines 303-333 and confirm no mutation of
`source_extension`. (Already confirmed from reading: the loop only touches
`edge_type.source_kinds` and `edge_type.target_kinds`.)

#### Fix C: If edges ARE in formal's `edge_types[]` but still misattributed

If investigation 5.1.1 shows the edges are already declared in formal's manifest
`edge_types[]`, then the issue is in `register_edge_types`. Check that formal's manifest
is being loaded and that the 11 edges are present in the JSON. If duplicate labels exist
across extensions, the first-wins policy (W018) might be eating formal's edges.

**Verification**: Check for W018 diagnostics when all extensions are loaded together.

### Verification

```bash
# After fix, run the model command and check formal's edge count
specforge model --format json | jq '.extensions[] | select(.name == "@specforge/formal")'
# Expected: { "name": "@specforge/formal", "version": "1.0.0", "entity_count": 5, "edge_count": 8 }

# Check total edge types matches
specforge schema | jq '.edge_types | length'
specforge model --format json | jq '[.extensions[].edge_count] | add'
# These two numbers must match

# Run existing tests
cargo test -p specforge-emitter -- model
cargo test -p specforge-registry -- populate
```

---

## Issue 5.2: Total Edge Count Mismatch

### Problem

The model header claims 54 total edge types (sum of per-extension edge_counts), but only
52 unique edge labels appear in the rendered relationship section. A delta of 2 suggests
either duplicate edge labels counted twice across extensions, or edges with missing
source_kinds/target_kinds that produce zero relationships but are still counted in the
extension summary.

### Root Cause Analysis

Two independent paths compute "edge count":

1. **Extension summary table** (`build.rs:build_extensions`, line 152): Iterates
   `schema.edge_types` and counts by `source_extension`. This counts ALL edge types
   in the schema, including those with `source_kinds: None` or `target_kinds: None`.

2. **Relationship list** (`build.rs`, line 23-27): Iterates `schema.edge_types` and
   calls `build_relationships()`, which returns an empty Vec for edges with
   `source_kinds: None` or `target_kinds: None` (lines 108-115).

3. **Summary footer** (`markdown.rs`, line 42-46): Uses `model.relationships.len()`,
   which counts expanded (source x target) Cartesian products, NOT unique edge labels.
   An edge with `source_kinds: [behavior, invariant]` and `target_kinds: [condition]`
   produces 2 relationships, inflating the count.

So there are TWO distinct mismatch sources:
- Edges with no source/target kinds contribute to extension edge_count but produce zero
  relationships (count goes DOWN in relationship list).
- Edges with multiple source/target kinds produce Cartesian product relationships, so a
  single schema edge may produce 2+ relationships (count goes UP in relationship list).

### Investigation Steps

- [ ] **5.2.1**: Count edges with `source_kinds: null` or `target_kinds: null` in the
      schema output. These are phantom edges that inflate the extension count.

- [ ] **5.2.2**: Count edges whose source_kinds or target_kinds have >1 entry. These
      create Cartesian product relationships that inflate the relationship count.

- [ ] **5.2.3**: Determine which number the user actually expects: unique edge TYPE
      labels, or expanded relationship count.

### Fix Plan

The model header should report **unique edge type labels** per extension (matching what
the schema declares), and the summary footer should ALSO report unique edge type labels
(not expanded relationship count).

#### Fix 5.2a: Header edge counts stay as-is (they already count schema edge_types)

The `build_extensions` function in `build.rs` counts `schema.edge_types` by
`source_extension`. This is correct -- it counts declared edge types. No change needed
here (assuming 5.1 fixes attribution).

#### Fix 5.2b: Summary footer should count unique edge labels, not relationships

**File**: `crates/specforge-emitter/src/model/markdown.rs`, line 42-46

Current:
```rust
writeln!(
    out, "{} entity kinds, {} edge types across {} extensions.",
    model.entities.len(),
    model.relationships.len(),  // <-- BUG: counts expanded relationships
    model.extensions.len()
).unwrap();
```

Fix:
```rust
// Count unique edge type labels (not expanded Cartesian relationships)
let unique_edge_labels: std::collections::HashSet<&str> = model
    .relationships
    .iter()
    .map(|r| r.name.as_str())
    .collect();

writeln!(
    out, "{} entity kinds, {} edge types across {} extensions.",
    model.entities.len(),
    unique_edge_labels.len(),
    model.extensions.len()
).unwrap();
```

**Alternative**: Use the sum of extension edge_counts directly, which already comes from
schema edge_types:

```rust
let total_edges: usize = model.extensions.iter().map(|e| e.edge_count).sum();

writeln!(
    out, "{} entity kinds, {} edge types across {} extensions.",
    model.entities.len(),
    total_edges,
    model.extensions.len()
).unwrap();
```

The second approach is preferred because it always matches the header table and does not
depend on whether edges have resolved source/target kinds.

#### Fix 5.2c: Filter's `recompute_extensions` should also count edge types, not relationships

**File**: `crates/specforge-emitter/src/model/filter.rs`, lines 99-112

Current code counts relationships by source entity extension, which is
(a) inaccurate for edge ownership (same problem as 5.1) and (b) counts expanded
relationships rather than unique edge labels.

This is harder to fix because after filtering, we do not have access to the original
schema edge_types. Two options:

1. **Carry `declared_edge_count` on `ModelRelationship`** -- Add a
   `source_extension: String` field to `ModelRelationship` so `recompute_extensions`
   can count by the edge's declaring extension.

2. **Store edge type metadata on `ModelIntermediate`** -- Add a lightweight
   `edge_type_extensions: Vec<(String, String)>` field that maps edge label to
   source_extension, copied from the schema during build. Filtering preserves this
   and uses it for counts.

Option 2 is cleaner. Add to `ModelIntermediate`:

```rust
/// Maps edge label -> declaring extension. Used for accurate extension edge counts
/// after filtering. Not serialized to output.
#[serde(skip)]
pub edge_type_owners: Vec<(String, String)>,
```

Populate in `build.rs`:
```rust
let edge_type_owners: Vec<(String, String)> = schema
    .edge_types
    .iter()
    .map(|e| (e.label.clone(), e.source_extension.clone()))
    .collect();
```

Use in `filter.rs:recompute_extensions`:
```rust
// Count unique edge types per extension that survive the filter
let surviving_labels: HashSet<&str> = relationships
    .iter()
    .map(|r| r.name.as_str())
    .collect();

let mut edge_counts: HashMap<&str, usize> = HashMap::new();
for (label, ext) in &model.edge_type_owners {
    if surviving_labels.contains(label.as_str()) {
        *edge_counts.entry(ext.as_str()).or_insert(0) += 1;
    }
}
```

### Verification

```bash
# Verify header total matches footer total
specforge model | grep -E "edge types|Edge Types"
# The row totals in the extension table should sum to the footer number

# Verify with JSON
specforge model --format json | jq '{
  header_total: ([.extensions[].edge_count] | add),
  unique_relationship_labels: ([.relationships[].name] | unique | length)
}'
# header_total should match unique_relationship_labels

# Run tests
cargo test -p specforge-emitter -- model
```

---

## Issue 5.3: Verify Entity Counts

### Problem

Some expert reviewers flagged entity count mismatches. Need to verify that after
Phases 1-4, the model header reports accurate entity counts per extension.

### Expected Counts (Post Phase 1-4)

| Extension | Expected Entities | Expected Edges | Notes |
|-----------|------------------|----------------|-------|
| @specforge/software | 6 | 10 | behavior, invariant, event, type, port + MilestoneBehavior via enhancement. 8 original + Implements + MilestoneBehavior. |
| @specforge/product | 12 | 16 | journey, deliverable, milestone, module, term, feature, persona, channel, release + 3 more from Phase 4. 16 edge types declared. |
| @specforge/governance | 3 | 6 | decision, constraint, failure_mode. 6 edge types. |
| @specforge/formal | 5 | 8 | property, axiom, protocol, refinement, process. 8 edge types. |

### Investigation Steps

- [ ] **5.3.1**: Run `specforge model --format json` and extract per-extension counts:
      ```bash
      specforge model --format json | jq '.extensions[] | {name, entity_count, edge_count}'
      ```

- [ ] **5.3.2**: Cross-reference against manifest files:
      ```bash
      # Count entity kinds in each manifest
      for ext in software product governance formal; do
        echo "=== $ext ==="
        cat spec/extensions/$ext/manifest.json | jq '.entityKinds | length'
        cat spec/extensions/$ext/manifest.json | jq '.edgeTypes | length'
      done
      ```

- [ ] **5.3.3**: If software shows 5 instead of 6, check whether `feature` is being
      miscounted. Feature is owned by `@specforge/product`, not software. The Implements
      edge goes behavior->feature but the feature entity itself belongs to product.

- [ ] **5.3.4**: If governance shows fewer than 6 edges, check whether some edges are
      only implied via fields (not declared in `edge_types[]`). The implicit edge
      registration may have missed them, or they may be cross-extension enhancement
      edges (same as issue 5.1).

### Fix Plan

Entity counts should be straightforward -- `build_extensions` counts entities by
`entity.extension`, which comes from `SchemaEntityKind.source_extension`, which comes
from `KindRegistryEntry.source_extension`, which is set to `manifest.name` in
`register_entity_kinds`. This chain is clean.

If counts are wrong, the issue is in the manifest data (Phases 1-4 territory), not the
builder. Document any discrepancies found and file them back to the relevant phase.

Edge counts depend on fix 5.1 being applied correctly. After 5.1, re-verify.

### Verification

```bash
# Full verification script
specforge model --format json | jq '
  .extensions[] | {
    name,
    entity_count,
    edge_count,
    entities: [.name] # not available at extension level, check separately
  }
'

# Cross-check entities
specforge model --format json | jq '
  [.entities[] | {name, extension}] | group_by(.extension) |
  map({extension: .[0].extension, count: length, names: [.[].name]})
'

# Cross-check edge labels per extension
specforge schema | jq '
  [.edge_types[] | {label, source_extension}] | group_by(.source_extension) |
  map({extension: .[0].source_extension, count: length, labels: [.[].label]})
'
```

---

## Implementation Order

Fixes should be applied in this order due to dependencies:

1. **5.1** (Edge ownership) -- Must be fixed first. All other counts depend on correct
   `source_extension` attribution.
2. **5.2b** (Footer count) -- Quick fix, no dependencies.
3. **5.2c** (Filter recompute) -- Requires the `edge_type_owners` field from 5.1.
4. **5.3** (Entity count verification) -- Verification pass after 5.1 + 5.2 are done.

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/specforge-registry/src/compilation/populate.rs` | Add enhancement-contributed implicit edge registration after `apply_entity_enhancements` |
| `crates/specforge-emitter/src/model/build.rs` | Add `edge_type_owners` population to `ModelIntermediate_from_schema` |
| `crates/specforge-emitter/src/model/mod.rs` | Add `edge_type_owners: Vec<(String, String)>` field to `ModelIntermediate` |
| `crates/specforge-emitter/src/model/markdown.rs` | Fix summary footer to use extension edge_count sum |
| `crates/specforge-emitter/src/model/filter.rs` | Fix `recompute_extensions` to count edge types by label owner |
| `crates/specforge-emitter/tests/model.rs` | Add tests for cross-extension edge attribution, update snapshots |
| `crates/specforge-registry/src/compilation/populate.rs` (tests) | Add test for enhancement-contributed implicit edge registration |

---

## New Tests

### Test: Enhancement-contributed edges attributed to enhancing extension

```rust
// In crates/specforge-registry/src/compilation/populate.rs tests
#[test]
fn enhancement_contributed_edge_attributed_to_enhancing_extension() {
    let software = software_manifest();
    // Create a manifest that enhances behavior with a reference field that has an edge
    let enhancer: ManifestV2 = serde_json::from_str(r#"{
        "name": "@test/enhancer",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "enhancer.wasm",
        "entityEnhancements": [{
            "targetKind": "behavior",
            "sourceExtension": "@test/enhancer",
            "fields": [{
                "name": "conditions",
                "fieldType": "reference_list",
                "edge": "RequiresCondition",
                "targetKind": "condition"
            }]
        }]
    }"#).unwrap();

    let (_, _, edge_reg, _) = populate_registries(&[software, enhancer]);

    // The edge should exist and be attributed to the enhancer
    assert!(edge_reg.contains("RequiresCondition"));
    let edge = edge_reg.get("RequiresCondition").unwrap();
    assert_eq!(edge.source_extension, "@test/enhancer");
    assert_eq!(edge.source_kind.as_deref(), Some("behavior"));
    assert_eq!(edge.target_kind.as_deref(), Some("condition"));
}
```

### Test: Model extension edge count matches schema edge types

```rust
// In crates/specforge-emitter/tests/model.rs
#[test]
fn extension_edge_count_matches_schema_edge_types() {
    // Schema with cross-extension edges: formal enhances software entities
    let schema = GraphProtocolSchema {
        schema_version: SchemaVersion::new(1, 0, 0),
        extensions: vec![
            SchemaExtensionInfo { name: "@ext/a".into(), version: "1.0.0".into() },
            SchemaExtensionInfo { name: "@ext/b".into(), version: "1.0.0".into() },
        ],
        entity_kinds: vec![
            SchemaEntityKind {
                name: "alpha".into(),
                source_extension: "@ext/a".into(),
                testable: false,
                fields: vec![],
            },
            SchemaEntityKind {
                name: "beta".into(),
                source_extension: "@ext/b".into(),
                testable: false,
                fields: vec![],
            },
        ],
        edge_types: vec![
            SchemaEdgeType {
                label: "OwnedByA".into(),
                source_extension: "@ext/a".into(),
                source_kinds: Some(vec!["alpha".into()]),
                target_kinds: Some(vec!["beta".into()]),
            },
            // Cross-extension edge: declared by B, connects A's entity to B's entity
            SchemaEdgeType {
                label: "CrossFromB".into(),
                source_extension: "@ext/b".into(),
                source_kinds: Some(vec!["alpha".into()]),
                target_kinds: Some(vec!["beta".into()]),
            },
        ],
    };

    let model = ModelIntermediate_from_schema(&schema);

    let ext_a = model.extensions.iter().find(|e| e.name == "@ext/a").unwrap();
    let ext_b = model.extensions.iter().find(|e| e.name == "@ext/b").unwrap();

    // Edge counts should follow source_extension, not source entity's extension
    assert_eq!(ext_a.edge_count, 1, "OwnedByA belongs to @ext/a");
    assert_eq!(ext_b.edge_count, 1, "CrossFromB belongs to @ext/b");
}
```

### Test: Footer total matches header total

```rust
// In crates/specforge-emitter/tests/model.rs
#[test]
fn markdown_footer_total_matches_header_total() {
    let schema = multi_extension_schema();
    let model = ModelIntermediate_from_schema(&schema);
    let output = render(&model, &default_options(ModelFormat::Markdown));

    // Extract the footer line
    let footer_line = output.lines().last().unwrap();
    // Should contain "3 edge types" (3 unique edge labels in multi_extension_schema)
    assert!(
        footer_line.contains("3 edge types"),
        "footer should report 3 unique edge types, got: {}",
        footer_line
    );

    // Header sum should also be 3
    let header_sum: usize = model.extensions.iter().map(|e| e.edge_count).sum();
    assert_eq!(header_sum, 3);
}
```

---

## Progress Tracking

- [ ] 5.1.1 -- Enumerate formal edge types in manifest vs edge_types[]
- [ ] 5.1.2 -- Run `specforge schema` to check current attribution
- [ ] 5.1.3 -- Check total edge count from schema
- [ ] 5.1.4 -- Trace each misattributed edge through the registration pipeline
- [ ] 5.1 FIX -- Add enhancement-contributed implicit edge registration
- [ ] 5.1 TEST -- Test enhancement edge attribution
- [ ] 5.2.1 -- Count phantom edges (null source/target kinds)
- [ ] 5.2.2 -- Count multi-source/target edges
- [ ] 5.2b FIX -- Fix markdown footer to use extension edge_count sum
- [ ] 5.2c FIX -- Add edge_type_owners to ModelIntermediate
- [ ] 5.2c FIX -- Fix filter recompute_extensions to use edge_type_owners
- [ ] 5.2 TEST -- Test footer matches header
- [ ] 5.3.1 -- Verify per-extension entity counts
- [ ] 5.3.2 -- Cross-reference against manifest files
- [ ] 5.3.3 -- Verify feature ownership (product not software)
- [ ] 5.3.4 -- Check governance edge count
- [ ] 5.3 VERIFY -- All counts match expected values
- [ ] ALL SNAPSHOTS UPDATED -- `cargo insta review`
- [ ] ALL TESTS PASS -- `cargo test -p specforge-emitter -p specforge-registry`

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Fix 5.1 changes EdgeRegistry contents, breaking existing tests | Run full test suite after fix; update snapshot tests in Phase 6 |
| Adding `edge_type_owners` field to ModelIntermediate breaks JSON serialization | Field is `#[serde(skip)]`, no serialization impact |
| Enhancement-contributed edges may duplicate edges already in `edge_types[]` | The `!edge_reg.contains(edge_label)` guard prevents duplicates (same as existing implicit edge logic) |
| Filter recompute changes may alter filtered model output | Filter tests already exist; update expectations |
| Footer count change affects all markdown/text snapshot tests | Batch snapshot updates in Phase 6 |

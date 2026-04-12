# Phase 6: IR Enrichments

**Goal**: Enrich the IR types and builder to surface data the 5-expert panel identified as missing — required/optional field indicators, validation rule codes at keys level, edge descriptions, shared fields, and collector contributions.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 6.1 | Add `required` field to `OutlineField` | PENDING |
| 6.2 | Add `description` to `OutlineField` | PENDING |
| 6.3 | Add `check` (rule category) to `OutlineValidationRule` | PENDING |
| 6.4 | Add `description` to `OutlineEdgeType` | PENDING |
| 6.5 | Add `shared_fields: Vec<OutlineSharedField>` to `OutlineExtension` | PENDING |
| 6.6 | Add `collector_count: usize` to `OutlineExtension` | PENDING |
| 6.7 | Add `grammar_count: usize` to `OutlineExtension` | PENDING |
| 6.8 | Populate all new fields in `build.rs` from `ManifestV2` | PENDING |
| 6.9 | Add `OutlineSharedField` struct to `mod.rs` | PENDING |
| 6.10 | Test: required field populated from manifest | PENDING |
| 6.11 | Test: validation rule codes and check categories populated | PENDING |
| 6.12 | Test: shared fields mapped | PENDING |

## Details

### 6.1-6.2: Field enrichment

`ManifestField` has `required: bool` and `description: Option<String>` that we currently discard. Add to `OutlineField`:

```rust
pub required: bool,
#[serde(skip_serializing_if = "Option::is_none")]
pub description: Option<String>,
```

### 6.3: Validation rule check category

`ManifestValidationRule` has `check: String` (e.g., "no_incoming_edges", "cycle_detection", "custom") and `message_template: String`. Currently we only map `code`, `severity`, `target_kind`. Add:

```rust
pub check: String,  // rule category — critical for understanding what each rule does
```

### 6.4: Edge description

`ManifestEdgeType` has `description: Option<String>`. Add to `OutlineEdgeType`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub description: Option<String>,
```

### 6.5 + 6.9: Shared fields

`ManifestV2.fields` are fields applied to ALL entity kinds (overridable per-kind). Currently not surfaced at all. New type:

```rust
pub struct OutlineSharedField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}
```

### 6.6-6.7: Contribution counts

`ManifestV2` has `collector_contributions` and `grammar_contributions`. Surface as counts on `OutlineExtension`.

### 6.8: Builder updates

Update `OutlineIntermediate_from_manifests` to populate all new fields from corresponding `ManifestV2` fields.

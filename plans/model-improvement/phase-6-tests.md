# Phase 6: Test & Snapshot Updates

**Status**: COMPLETE (2026-04-12) — Tests were updated inline with each phase via TDD
**Depends on**: Phase 1-5 complete
**Targets**: All axes (ensures correctness after all changes)

## Strategy

### TDD Approach

For each phase's manifest changes, the workflow is:

1. **RED** -- Update test assertions first (counts, labels, field names). Tests fail because manifests have not changed yet.
2. **GREEN** -- Apply the manifest/code changes from that phase. Tests pass.
3. **REFACTOR** -- Clean up, regenerate snapshots.

In practice, Phase 6 runs *after* Phases 1-5 are already applied. So the order is:

1. Apply all manifest + code changes (Phases 1-5).
2. Run `cargo test` to see every failure.
3. Fix each test file in dependency order (registries first, then schema, then model/snapshots, then MCP, then CLI e2e).
4. Verify green across the entire workspace.

### Execution Order

Tests must be updated in this order to avoid cascading confusion:

```
1. specforge-registry tests (manifest deserialization, registry population)
2. specforge-emitter/tests/schema.rs (schema generation from registries)
3. specforge-emitter/tests/model.rs (model builder, render, snapshot regen)
4. specforge-mcp tests (MCP tool/resource responses)
5. specforge-cli e2e tests (full pipeline)
6. specforge-graph, specforge-validator, specforge-lsp (if affected)
```

---

## Part A: Registry Tests

### A1. `crates/specforge-registry/tests/software_manifest.rs`

**Current state**: 16 tests, all passing against current manifests.

#### Phase 1 changes (Manifest Bugs)

| Test | What changes | Details |
|------|-------------|---------|
| `test_software_manifest_has_11_edge_types` | Edge count 11 -> 13 | +2 new edges: `ModuleConsumesPort`, `ModuleDefinesPort`. `UsesPort` removed from entity_enhancements but it stays as an explicit edge type, so net +2. |
| `test_software_manifest_has_11_edge_types` (labels) | Add assertions for `ModuleConsumesPort`, `ModuleDefinesPort` | These are new edge labels. |
| `test_software_manifest_enhancement_module` | `ports` field edge changes | Module enhancement `ports` field: `UsesPort` edge removed, replaced by `ModuleConsumesPort`. `ports_defined` field: edge becomes `ModuleDefinesPort`. |
| `test_software_manifest_populates_registries` | Edge assertions | Add `edge_reg.contains("ModuleConsumesPort")`, `edge_reg.contains("ModuleDefinesPort")`. |
| `test_software_manifest_validation_rules_parse` | Rule count may change | If validation rules are added/removed for new edge types, assert_eq on `manifest.validation_rules.len()` must be updated from 17. |

#### Phase 2 changes (Edge Naming)

| Test | What changes | Details |
|------|-------------|---------|
| `test_software_manifest_has_11_edge_types` (labels) | All edge label assertions | Every renamed label must be updated. E.g., if `References` -> `BehaviorReferences`, `Produces` -> `BehaviorProducesEvent`, etc. Review Phase 2 plan for exact renames. |
| `test_software_implements_edge` | `label == "Implements"` | Update if renamed (e.g., `BehaviorImplementsFeature`). |
| `test_software_produces_edge` | `label == "Produces"` | Update if renamed. |
| `test_software_enforces_edge` | `label == "Enforces"` | Update if renamed. |
| `test_software_extends_type_edge` | `label == "ExtendsType"` | Update if renamed. |
| `test_software_manifest_populates_registries` | All `edge_reg.contains(...)` | Update every edge label string. |
| `test_software_manifest_enhancement_milestone` | `beh_field.edge == "MilestoneBehavior"` | Update if renamed. |

#### Phase 3 changes (Field Consistency)

Software manifest is NOT the primary target of Phase 3. No changes expected.

#### Phase 4 changes (Cross-Extension)

Software manifest is NOT the primary target of Phase 4. No changes expected unless entity_enhancements are added.

**Checklist**:
- [ ] Update edge count assertion (11 -> new count)
- [ ] Update edge label assertions for renamed labels
- [ ] Update module enhancement field edge assertions
- [ ] Update validation rule count if changed
- [ ] Update `populate_registries` edge assertions
- [ ] Update individual edge test functions for renamed labels
- [ ] Verify `test_software_manifest_passes_schema_validation` still passes
- [ ] Verify `test_software_manifest_passes_consistency_validation` still passes

---

### A2. `crates/specforge-registry/tests/product_manifest.rs`

**Current state**: 8 tests, all passing.

#### Phase 1 changes

| Test | What changes | Details |
|------|-------------|---------|
| `test_product_manifest_has_27_edge_types` | Edge count 27 -> 28 | +1 new edge: `FeatureRelatesTo` |
| `test_product_manifest_has_27_edge_types` (labels) | Add assertion for `FeatureRelatesTo` | New label in the assertion list. |
| `test_product_manifest_populates_registries` | Add `edge_reg.contains("FeatureRelatesTo")` | |

#### Phase 2 changes

| Test | What changes | Details |
|------|-------------|---------|
| `test_product_manifest_has_27_edge_types` (labels) | All edge label assertions | Every renamed label must be updated per Phase 2 renames. |
| `test_product_manifest_populates_registries` | All `edge_reg.contains(...)` | Update every edge label string. |
| `test_product_manifest_populates_registries` | `term_module.edge == "TermBelongsToModule"` | Update if renamed. |

#### Phase 3 changes

| Test | What changes | Details |
|------|-------------|---------|
| `test_product_manifest_has_27_edge_types` | Edge count 28 -> 29 | +1 new edge: `CapabilityPersona` (capability.persona becomes reference) |
| `test_product_manifest_populates_registries` | capability.persona field type | If `persona` becomes a reference field, assertions about field_reg may need updating. |

#### Phase 4 changes

No product manifest changes expected in Phase 4.

**Checklist**:
- [ ] Update edge count assertion (27 -> final count after Phase 1+3)
- [ ] Add `FeatureRelatesTo` label assertion
- [ ] Add `CapabilityPersona` label assertion (Phase 3)
- [ ] Update all edge label assertions for Phase 2 renames
- [ ] Update `populate_registries` edge and field assertions
- [ ] Update validation rule count if changed
- [ ] Verify schema and consistency validation still pass

---

### A3. `crates/specforge-registry/tests/zero_entity_registries.rs`

**Current state**: ~60 tests using inline mini-manifests. These tests use synthetic manifests (`software_manifest()` and `product_manifest()` helper functions), NOT the real manifest.json files.

**Impact**: Minimal. The inline helper manifests won't change because they are self-contained. However:

- If new `ManifestV2` fields are added to the struct (Phase 4), the inline JSON helpers might need updating to include new required fields.
- The `ManifestEdgeType` struct already has 7 fields. If Phase 2 adds new fields (e.g., `description`), the test code constructing `ManifestEdgeType` values inline needs updating.

**Checklist**:
- [ ] Check if `ManifestV2` struct gains required fields -> update inline manifests
- [ ] Check if `ManifestEdgeType` struct changes -> update inline constructors
- [ ] Run `cargo test -p specforge-registry --test zero_entity_registries` to verify

---

### A4. `crates/specforge-registry/tests/keyword_index.rs`

**Current state**: 6 tests using synthetic manifests.

**Impact**: None. These tests use `manifest_with_kinds()` helper that builds minimal manifests. No dependency on real manifest files.

**Checklist**:
- [ ] Check if `ManifestV2` struct changes require `default_manifest()` update
- [ ] Check if `ManifestEntityKind` struct changes require `make_entity_kind()` update

---

## Part B: Schema Tests

### B1. `crates/specforge-emitter/tests/schema.rs`

**Current state**: 52 tests. Uses `sample_schema()` helper with synthetic data, NOT real manifests.

**Impact**: Minimal for most tests. But:

- `SchemaEdgeType` struct: if Phase 2 adds/changes fields, the `sample_schema()` helper must be updated.
- `SchemaField` struct: if Phase 3 adds new field attributes, test helpers need updating.
- Integration tests that build registries inline are self-contained.

**What changes**:

| Area | What changes | Details |
|------|-------------|---------|
| `sample_schema()` helper | Possibly update struct fields | If `SchemaEdgeType` gains new fields from Phase 2 |
| `make_edge_entry()` helper | Possibly update struct fields | If `EdgeRegistryEntry` changes |
| `make_field_entry()` helper | Possibly update struct fields | If `FieldRegistryEntry` changes |

**Checklist**:
- [ ] Update `sample_schema()` if struct fields change
- [ ] Update `make_edge_entry()` if `EdgeRegistryEntry` changes
- [ ] Update `make_field_entry()` if `FieldRegistryEntry` changes
- [ ] Run `cargo test -p specforge-emitter --test schema` to verify

---

## Part C: Model Tests & Snapshots

### C1. `crates/specforge-emitter/tests/model.rs`

**Current state**: 28 tests. Uses `multi_extension_schema()` helper and inline schemas. 14 snapshot tests.

**Impact from Phase 5 (Model Builder)**:

The model builder code (`build.rs`) changes in Phase 5 directly affect:

| Test | What changes | Details |
|------|-------------|---------|
| `edge_type_maps_to_relationship` | Cardinality inference | If Phase 5 improves cardinality detection |
| `reference_field_infers_many_to_one` | May stay the same | Core logic unchanged |
| `reference_singular_field_infers_many_to_one_for_term_module` | Contribution annotation | If contribution format changes |
| All snapshot tests | Content changes | If render output format changes |

**Impact from Phase 2 (Edge Naming)**:

If the `multi_extension_schema()` helper uses edge labels like `"Implements"`, `"Triggers"`, `"JourneyFeature"` and Phase 2 renames these, the helper must be updated AND all 14 snapshots regenerated.

**What changes specifically**:

- `multi_extension_schema()`: Update edge labels `"Implements"` -> new name, `"Triggers"` -> new name, `"JourneyFeature"` -> new name
- `edge_type_maps_to_relationship`: Update `rel.name == "Implements"` assertion
- `filter_by_extension`: Update `filtered.relationships[0].name == "Triggers"` assertion
- `filter_by_kinds`: Update `filtered.relationships[0].name == "Implements"` assertion

### C2. Snapshot Files (14 files)

Location: `crates/specforge-emitter/tests/snapshots/`

All snapshots depend on `multi_extension_schema()` and `build_model_keys()`. Any change to entity names, edge labels, field names, or renderer output format requires regeneration.

**Regeneration procedure**:
```bash
cd /Users/u1070457/Projects/Perso/specforge
INSTA_UPDATE=always cargo test -p specforge-emitter --test model
```

After regeneration, review each `.snap` file to confirm the changes are expected.

**Snapshot files affected**:
1. `tests__model__render_markdown_keys_grouped.snap`
2. `tests__model__render_markdown_none_fields.snap`
3. `tests__model__render_markdown_flat.snap`
4. `tests__model__render_markdown_empty.snap`
5. `tests__model__render_mermaid_keys_grouped.snap`
6. `tests__model__render_mermaid_none_fields.snap`
7. `tests__model__render_mermaid_empty.snap`
8. `tests__model__render_dot_keys_grouped.snap`
9. `tests__model__render_dot_none_fields.snap`
10. `tests__model__render_dot_empty.snap`
11. `tests__model__render_json_keys.snap`
12. `tests__model__render_json_empty.snap`
13. `tests__model__render_dbml_keys_grouped.snap`
14. `tests__model__render_dbml_empty.snap`

Empty snapshots (4, 7, 10, 12, 14) will NOT change -- they use `GraphProtocolSchema::empty()`.

Non-empty snapshots (1, 2, 3, 5, 6, 8, 9, 11, 13) WILL change if edge labels are renamed in Phase 2.

**Checklist**:
- [ ] Update `multi_extension_schema()` edge labels
- [ ] Update `edge_type_maps_to_relationship` assertions
- [ ] Update `filter_by_extension` assertions
- [ ] Update `filter_by_kinds` assertions
- [ ] Regenerate snapshots with `INSTA_UPDATE=always`
- [ ] Review all 9 non-empty snapshots for correctness
- [ ] Verify all 14 snapshot tests pass

---

## Part D: MCP Tests

### D1. `crates/specforge-mcp/tests/contracts.rs`

**Current state**: 7+ tests using in-memory test graphs.

**Impact**: These tests use synthetic graphs (not compiled from manifests), so manifest changes don't directly affect them. However:

- If MCP tool names change (unlikely)
- If export format structure changes (unlikely unless Phase 5 changes V2 format)
- If `McpServer` API changes

**Checklist**:
- [ ] Run `cargo test -p specforge-mcp` to verify all pass
- [ ] If any tool/resource names change, update call strings

### D2. `crates/specforge-mcp/tests/resources.rs`

**Current state**: Tests for `specforge://graph` and `specforge://schema` resources.

**Impact**: Minimal. Tests use injected graphs, not manifest-derived data.

**Checklist**:
- [ ] Run `cargo test -p specforge-mcp` to verify

### D3. `crates/specforge-mcp/tests/surface_wiring.rs`

**Current state**: Tests for surface contributions (MCP tools/resources from manifests).

**Impact**: If manifest surface contributions change (tool names, resource URIs), these tests must be updated.

**Checklist**:
- [ ] Review if any surface contribution names changed in Phases 1-4
- [ ] Update tool/resource name assertions if needed

### D4. Other MCP tests

Files: `prompts.rs`, `lifecycle.rs`, `tools_core.rs`, `tools_navigation.rs`, `operations_mgmt.rs`, `operations_mutation.rs`, `protocol.rs`, `events.rs`, `subscriptions.rs`, `notifications.rs`, `invariants.rs`

**Impact**: Low. These test MCP protocol mechanics, not manifest-derived data.

**Checklist**:
- [ ] Run full MCP test suite: `cargo test -p specforge-mcp`

---

## Part E: CLI E2E Tests

### E1. `crates/specforge-cli/tests/e2e_edges.rs`

**Current state**: Tests that `.spec` reference fields create graph edges.

**Impact from Phase 2**: If edge labels change in the manifests, the exported JSON edge labels change. These tests check edge source/target but NOT edge labels directly (they check `edges[0]["source"]` and `edges[0]["target"]`). So most should be unaffected.

**However**: If Phase 1 changes which fields create edges (e.g., module `ports` field now creates `ModuleConsumesPort` instead of `UsesPort`), any test that references module ports would be affected.

**Checklist**:
- [ ] Run `cargo test -p specforge-cli --test e2e_edges`
- [ ] Update edge label assertions if any test checks `edges[n]["label"]`

### E2. `crates/specforge-cli/tests/e2e_schema.rs`

**Current state**: 4 tests for `specforge schema` command.

**Impact**: Minimal. Tests check structural JSON keys, not specific entity/edge counts.

**Checklist**:
- [ ] Run `cargo test -p specforge-cli --test e2e_schema`

### E3. `crates/specforge-cli/tests/e2e_cross_extension.rs`

**Current state**: Tests cross-extension references work.

**Impact**: Tests use inline `.spec` content with known entity/field names. If field names or reference semantics change, these could break.

**Checklist**:
- [ ] Run `cargo test -p specforge-cli --test e2e_cross_extension`
- [ ] If `behaviors` field on feature changes name or semantics, update inline specs

### E4. Other CLI tests

Files: `e2e_mcp.rs`, `e2e_multi_entity.rs`, `e2e_query.rs`, `e2e_trace.rs`, `e2e_verify.rs`, `e2e_fixtures.rs`, `cli.rs`, `pipeline.rs`, `contracts.rs`, `export.rs`, `format.rs`, `extensions.rs`, `init.rs`, `query.rs`, `stats.rs`, `trace.rs`, `collect.rs`, `product_commands.rs`, `extension_authoring.rs`, `migrate.rs`

**Impact**: Variable. Most use inline `.spec` content and test structural output.

**Checklist**:
- [ ] Run `cargo test -p specforge-cli` to verify
- [ ] Fix any failures from field name or edge label changes

---

## Part F: Other Crates

### F1. `specforge-graph`

`crates/specforge-graph/tests/graph.rs` -- Tests graph data structures. No manifest dependency.

**Impact**: None unless Phase 5 changes graph types.

### F2. `specforge-validator`

`crates/specforge-validator/src/orphan.rs` -- Tests orphan detection.

**Impact**: If validation rule codes change or orphan detection logic changes, tests must be updated.

### F3. `specforge-lsp`

`crates/specforge-lsp/tests/completion.rs`, `hover.rs` -- Tests LSP features.

**Impact**: If entity kind metadata changes (semantic_token, lsp_icon), completion/hover tests may need updating.

**Checklist**:
- [ ] Run `cargo test -p specforge-lsp`

---

## Regression Verification Commands

### Per-crate verification (run in order):

```bash
# 1. Registry tests (first -- these validate manifest deserialization)
cargo test -p specforge-registry 2>&1 | tail -20

# 2. Emitter schema tests (validate schema generation)
cargo test -p specforge-emitter --test schema 2>&1 | tail -20

# 3. Emitter model tests + snapshot regeneration
INSTA_UPDATE=always cargo test -p specforge-emitter --test model 2>&1 | tail -20

# 4. Full emitter suite
cargo test -p specforge-emitter 2>&1 | tail -20

# 5. Graph tests
cargo test -p specforge-graph 2>&1 | tail -20

# 6. Validator tests
cargo test -p specforge-validator 2>&1 | tail -20

# 7. MCP tests
cargo test -p specforge-mcp 2>&1 | tail -20

# 8. LSP tests
cargo test -p specforge-lsp 2>&1 | tail -20

# 9. CLI tests (last -- full pipeline, depends on everything)
cargo test -p specforge-cli 2>&1 | tail -20
```

### Full workspace green check:

```bash
cargo test --workspace 2>&1 | tail -30
```

### Clippy (catch type errors from struct changes):

```bash
cargo clippy --workspace --all-targets 2>&1 | tail -30
```

### Snapshot review (after regeneration):

```bash
# List all pending snapshot changes
cargo insta review --workspace

# Or force-accept all (only after manual review):
INSTA_UPDATE=always cargo test -p specforge-emitter --test model
```

---

## Master Checklist by Phase

### After Phase 1 (Manifest Bugs):

- [ ] `software_manifest.rs`: edge count 11 -> 13, new labels `ModuleConsumesPort`/`ModuleDefinesPort`
- [ ] `software_manifest.rs`: module enhancement field edges updated
- [ ] `product_manifest.rs`: edge count 27 -> 28, new label `FeatureRelatesTo`
- [ ] No formal/governance manifest test files exist (only tested via `keyword_index.rs` and `zero_entity_registries.rs` with inline data)
- [ ] `cargo test -p specforge-registry` green

### After Phase 2 (Edge Naming):

- [ ] `software_manifest.rs`: ALL edge label strings updated to new names
- [ ] `product_manifest.rs`: ALL edge label strings updated to new names
- [ ] `model.rs`: `multi_extension_schema()` edge labels updated
- [ ] `model.rs`: filter/relationship assertion strings updated
- [ ] All 9 non-empty snapshots regenerated
- [ ] `cargo test -p specforge-registry -p specforge-emitter` green

### After Phase 3 (Field Consistency):

- [ ] `product_manifest.rs`: edge count updated (+1 for `CapabilityPersona` if applicable)
- [ ] `product_manifest.rs`: field type assertions for `capability.persona` updated
- [ ] `cargo test -p specforge-registry` green

### After Phase 4 (Cross-Extension):

- [ ] If governance manifest gains new edge types: no dedicated test file, but `keyword_index.rs` may need `default_manifest()` update
- [ ] `cargo test -p specforge-registry` green

### After Phase 5 (Model Builder):

- [ ] `model.rs`: cardinality/builder assertions updated if logic changes
- [ ] All 14 snapshots regenerated with `INSTA_UPDATE=always`
- [ ] Schema test helpers updated if struct fields change
- [ ] `cargo test -p specforge-emitter` green

### Final:

- [ ] `cargo test --workspace` all green
- [ ] `cargo clippy --workspace --all-targets` no errors
- [ ] All 14 snapshot files reviewed and committed
- [ ] No hardcoded edge labels or counts remain from pre-improvement state

---

## Count Impact Summary

This table summarizes expected numeric changes to test assertions:

| Test File | Assertion | Before | After (est.) | Phase |
|-----------|-----------|--------|-------------|-------|
| `software_manifest.rs` | edge_types.len() | 11 | 13 | P1 |
| `software_manifest.rs` | validation_rules.len() | 17 | TBD | P1 |
| `product_manifest.rs` | edge_types.len() | 27 | 29-30 | P1+P3 |
| `product_manifest.rs` | validation_rules.len() | >=19 | TBD | P1+P3 |
| `software_manifest.rs` | I004 info count | 2 | 2 (unchanged) | -- |
| `model.rs` | (no numeric counts, mostly structural) | -- | -- | P2+P5 |

**Note**: Exact "After" values depend on final Phase 1-5 decisions. Update this table as each phase is executed.

---

## Risk Mitigation

1. **Compile-first**: After each phase's manifest changes, run `cargo check --workspace` before attempting test updates. Struct field additions cause compile errors that must be fixed first.

2. **Snapshot discipline**: Never blindly accept snapshots. Always `cargo insta review` and verify each changed line makes sense.

3. **Isolated crate testing**: Fix one crate at a time. Don't try to fix everything at once.

4. **Git checkpoint**: Commit after each crate's tests are green:
   ```
   git add -A && git commit -m "Phase 6a: update specforge-registry tests for Phase 1-4 manifest changes"
   git add -A && git commit -m "Phase 6b: update specforge-emitter tests and regenerate snapshots"
   git add -A && git commit -m "Phase 6c: update MCP + CLI e2e tests"
   ```

5. **Formal/Governance coverage gap**: There are no dedicated `formal_manifest.rs` or `governance_manifest.rs` test files. Phase 1 changes to these manifests (RefinesTo split, supersedes removal) are only tested indirectly through `keyword_index.rs` and cross-extension integration. Consider adding `formal_manifest.rs` and `governance_manifest.rs` test files to match the pattern of `software_manifest.rs` and `product_manifest.rs`.

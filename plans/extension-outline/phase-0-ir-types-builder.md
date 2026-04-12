# Phase 0: IR Types & Builder

**Status**: NOT STARTED
**Depends on**: —
**Crate**: `specforge-emitter`
**New directory**: `crates/specforge-emitter/src/outline/`

---

## Goal

Define the intermediate representation that captures the full extension hierarchy from raw manifests, including inter-extension relationships that `GraphProtocolSchema` doesn't carry.

---

## Checklist

### 0.1: Create `outline/mod.rs` with all IR types

- [ ] Create `crates/specforge-emitter/src/outline/mod.rs`
- [ ] Define `OutlineIntermediate` (extensions, dependencies, enhancements, cross_edges)
- [ ] Define `OutlineExtension` (name, version, entity_kinds, edge_types, validation_rules, contributes, verify_kinds, surface_counts)
- [ ] Define `OutlineEntityKind` (name, keyword, testable, field_count, fields, enhanced_by)
- [ ] Define `OutlineField` (name, field_type, source_extension, edge, target_kind)
- [ ] Define `OutlineFieldAttribution` (source_extension, field_count, field_names)
- [ ] Define `OutlineEdgeType` (label, source_kind, target_kind)
- [ ] Define `OutlineValidationRule` (code, severity, target_kind)
- [ ] Define `OutlineContributes` (9 boolean flags matching ExtensionContributions)
- [ ] Define `OutlineSurfaceCounts` (cli_commands, mcp_tools, mcp_resources)
- [ ] Define `OutlineDependency` (from, to, version, optional)
- [ ] Define `OutlineEnhancement` (enhancer, owner, target_kind, field_count, field_names)
- [ ] Define `OutlineCrossEdge` (edge_label, owner_extension, source_kind, target_kind, target_extension)
- [ ] Define enums: `OutlineFormat` (Markdown/Mermaid/Dot/Json), `OutlineDetail` (None/Keys/All)
- [ ] Define `OutlineOptions` (format, detail)
- [ ] Add render dispatcher: `pub fn render(outline: &OutlineIntermediate, options: &OutlineOptions) -> String`
- [ ] All types derive `Debug, Clone, Serialize, Deserialize`

### 0.2: Create `outline/build.rs` — manifest-to-IR builder

- [ ] `OutlineIntermediate::from_manifests(&[ManifestV2]) -> OutlineIntermediate`
- [ ] Map each `ManifestV2` → `OutlineExtension` with entity/edge/rule counts
- [ ] Map each `PeerDependency` → `OutlineDependency` (track optional flag)
- [ ] Map each `FieldEnhancement` → `OutlineEnhancement` (track field names + count)
- [ ] Compute cross-extension edges: for each edge type, check if source/target kind belongs to a different extension
- [ ] For `--fields=all` path: map each entity kind's fields with `source_extension` attribution
- [ ] Map `ExtensionContributions` → `OutlineContributes`
- [ ] Map `surfaces` → `OutlineSurfaceCounts` (count cli_commands, mcp_tools, mcp_resources)
- [ ] Map `verify_kinds` → Vec<String>

### 0.3: Register module

- [ ] Add `pub mod outline;` to `crates/specforge-emitter/src/lib.rs`
- [ ] Re-export key types (`OutlineIntermediate`, `OutlineOptions`, `OutlineFormat`, `OutlineDetail`)

---

## Data Source Reference

The builder reads from `ManifestV2` (in `crates/specforge-registry/src/manifest/types.rs`):

| ManifestV2 field | → OutlineIntermediate field |
|---|---|
| `name`, `version` | `OutlineExtension.name`, `.version` |
| `entity_kinds[]` | `OutlineExtension.entity_kinds[]` → `OutlineEntityKind` |
| `edge_types[]` | `OutlineExtension.edge_types[]` → `OutlineEdgeType` |
| `validation_rules[]` | `OutlineExtension.validation_rules[]` → `OutlineValidationRule` |
| `peer_dependencies[]` | `OutlineIntermediate.dependencies[]` → `OutlineDependency` |
| `entity_enhancements[]` | `OutlineIntermediate.enhancements[]` → `OutlineEnhancement` |
| `contributes` | `OutlineExtension.contributes` → `OutlineContributes` |
| `verify_kinds[]` | `OutlineExtension.verify_kinds[]` |
| `surfaces.cli_commands[]` | `OutlineExtension.surface_counts.cli_commands` (count) |
| `surfaces.mcp_tools[]` | `OutlineExtension.surface_counts.mcp_tools` (count) |
| `surfaces.mcp_resources[]` | `OutlineExtension.surface_counts.mcp_resources` (count) |

Cross-edges are computed by matching edge source/target kinds against extension ownership:
- For each edge type in extension A, if `target_kind` belongs to extension B (B ≠ A), emit a `OutlineCrossEdge`.

---

## Verify

```bash
cargo test -p specforge-emitter  # builder unit tests pass
cargo clippy -p specforge-emitter  # zero warnings
```

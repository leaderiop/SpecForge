# Phase 3: Field Type Consistency

**Status**: COMPLETE (2026-04-12)
**Depends on**: Phase 2 (Edge Naming Standardization)
**Targets axes**: 3 (Field Completeness), 4 (Naming Consistency), 10 (Cardinality)
**Files modified**: `extensions/product/manifest.json`, `extensions/software/manifest.json`, `extensions/governance/manifest.json`, `extensions/formal/manifest.json`

---

## Summary

This phase fixes field type inconsistencies across the four extension manifests. There are two categories of work:

1. **Reference promotions** -- string fields that name entity kinds but are not typed as references (sections 3.1, 3.2)
2. **Missing enum constraints** -- string fields with known finite value sets that lack `field_value_constraint` validation rules (sections 3.3--3.8)

Total changes: 2 field type promotions, 1 new edge type, up to 8 new validation rules.

---

## 3.1: capability.persona -- string to reference

**File**: `extensions/product/manifest.json`
**Severity**: HIGH (confirmed inconsistency)
**Axes improved**: 3, 4, 9 (Connectivity), 10

### Problem

The `capability` entity declares `persona` as a plain string:
```json
{ "name": "persona", "description": "The user archetype who uses this capability", "fieldType": "string" }
```

But `journey` correctly references the `persona` entity kind:
```json
{ "name": "persona", "description": "The user archetype who undertakes this journey", "fieldType": "reference", "edge": "JourneyPersona", "targetKind": "persona" }
```

Same concept, same field name, different types. This breaks graph connectivity -- capabilities are invisible from the persona node's perspective. The persona entity was already flagged as too isolated (I046 fires when persona has no incoming edges, but only journey edges count today).

### Fix

**Step 1**: Change the `capability.persona` field definition (line ~238 in product manifest):

```diff
- { "name": "persona", "description": "The user archetype who uses this capability", "fieldType": "string" }
+ { "name": "persona", "description": "The user archetype who uses this capability", "fieldType": "reference", "edge": "CapabilityPersona", "targetKind": "persona" }
```

**Step 2**: Add the `CapabilityPersona` edge type to the `edgeTypes` array:

```json
{
  "label": "CapabilityPersona",
  "description": "Capability is used by a persona",
  "sourceKind": "capability",
  "targetKind": "persona",
  "edgeStyle": "solid",
  "edgeColor": "#E91E63"
}
```

Color `#E91E63` matches the persona entity's `dotColor` and the existing `JourneyPersona` edge color, keeping visual consistency.

**Step 3**: Update the I046 validation rule message template to reflect the new edge source:

```diff
- "messageTemplate": "persona '{id}' has no incoming edges — it may be unreferenced by any journey"
+ "messageTemplate": "persona '{id}' has no incoming edges — it may be unreferenced by any journey or capability"
```

### Impact

- Persona nodes gain edges from capabilities, improving connectivity score
- Agents querying "who uses persona X?" now get capabilities in addition to journeys
- Existing `.spec` files with `persona some_string` in capability blocks will now be resolved as references (if the persona entity exists) or emit unresolved-reference diagnostics (if not)

### Progress

- [ ] Modify `capability.persona` field in `extensions/product/manifest.json`
- [ ] Add `CapabilityPersona` edge type in `extensions/product/manifest.json`
- [ ] Update I046 message template in `extensions/product/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots
- [ ] Verify `specforge validate` still passes on example specs

---

## 3.2: capability.surface -- string_list to reference_list (channels)

**File**: `extensions/product/manifest.json`
**Severity**: MEDIUM (recommendation with trade-offs)
**Axes improved**: 3, 4, 9, 10

### Problem

The `capability` entity declares `surface` as a plain string list:
```json
{ "name": "surface", "description": "Surfaces or touchpoints through which this capability is accessed", "fieldType": "string_list" }
```

Meanwhile, `channel` is a first-class entity kind in `@specforge/product` representing "a communication or distribution channel for the product." The `journey` entity already references channels:
```json
{ "name": "channels", "description": "Communication or distribution channels used in this journey", "fieldType": "reference_list", "edge": "JourneyChannel", "targetKind": "channel" }
```

### Analysis

| Factor | Pro (make reference) | Con (keep string) |
|--------|---------------------|-------------------|
| Consistency | journey.channels is a reference_list; capability.surface should match | -- |
| Naming | "surface" and "channel" are different words -- could cause confusion | "surface" might intentionally mean something broader than "channel" |
| Connectivity | Channel nodes gain edges from capabilities | Requires channel entities to exist |
| Agent usability | Agents can traverse capability->channel->journey paths | Free-form strings are more flexible |
| Migration | Requires renaming field from `surface` to `channels` | No change needed |

### Recommendation: PROMOTE to reference_list

The `channel` entity already exists with fields like `interaction_model`, `url`, and `status`. If capability surfaces are not the same as channels, then what are they? In practice, "surface" and "channel" describe the same touchpoint concept. The field name `surface` predates the `channel` entity kind and was never updated. Renaming to `channels` and making it a reference aligns with the rest of the model.

### Fix

**Step 1**: Change the `capability.surface` field definition:

```diff
- { "name": "surface", "description": "Surfaces or touchpoints through which this capability is accessed", "fieldType": "string_list" }
+ { "name": "channels", "description": "Channels or touchpoints through which this capability is accessed", "fieldType": "reference_list", "edge": "CapabilityChannel", "targetKind": "channel" }
```

**Step 2**: Add the `CapabilityChannel` edge type to the `edgeTypes` array:

```json
{
  "label": "CapabilityChannel",
  "description": "Capability is accessed through a channel",
  "sourceKind": "capability",
  "targetKind": "channel",
  "edgeStyle": "solid",
  "edgeColor": "#00BCD4"
}
```

Color `#00BCD4` matches the channel entity's `dotColor` and the existing `JourneyChannel` edge color.

**Step 3**: Update the I047 validation rule message template:

```diff
- "messageTemplate": "channel '{id}' has no incoming edges — it may be unreferenced by any journey"
+ "messageTemplate": "channel '{id}' has no incoming edges — it may be unreferenced by any journey or capability"
```

### Migration note

Any existing `.spec` files using `surface [web, mobile, api]` inside a capability block need migration to `channels [web, mobile, api]` (where `web`, `mobile`, `api` are now entity IDs, not free-text). This is a breaking change for existing specs. Consider adding a migration rule to `specforge migrate` that detects `surface` in capability blocks and suggests the rename.

### Progress

- [ ] Rename `capability.surface` to `capability.channels` in `extensions/product/manifest.json`
- [ ] Change field type from `string_list` to `reference_list` with edge and targetKind
- [ ] Add `CapabilityChannel` edge type in `extensions/product/manifest.json`
- [ ] Update I047 message template in `extensions/product/manifest.json`
- [ ] Add migration rule for `surface` -> `channels` rename
- [ ] Update affected Rust test fixtures and snapshots
- [ ] Verify `specforge validate` still passes on example specs

---

## 3.3: condition.kind -- add enum constraint

**File**: `extensions/formal/manifest.json`
**Severity**: MEDIUM
**Axes improved**: 3, 4

### Problem

The `condition.kind` field accepts any string, but its description says "precondition, postcondition, invariant." These are the only valid values per the formal methods domain.

### Fix

Add a validation rule to `extensions/formal/manifest.json` `validationRules` array:

```json
{
  "code": "W058",
  "severity": "warning",
  "messageTemplate": "condition '{id}' has invalid kind '{value}' — expected one of: precondition, postcondition, invariant",
  "check": "field_value_constraint",
  "targetKind": "condition",
  "field": "kind",
  "constraint": { "kind": "one_of", "values": ["precondition", "postcondition", "invariant"] }
}
```

Diagnostic code W058 is within `@specforge/formal`'s allocated range (W058-W074).

### Progress

- [ ] Add W058 validation rule to `extensions/formal/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## 3.4: property.property_type -- add enum constraint

**File**: `extensions/formal/manifest.json`
**Severity**: MEDIUM
**Axes improved**: 3, 4

### Problem

The `property.property_type` field accepts any string, but its description says "safety, liveness, or fairness." These are the three categories from temporal logic.

### Fix

Add a validation rule to `extensions/formal/manifest.json` `validationRules` array:

```json
{
  "code": "W059",
  "severity": "warning",
  "messageTemplate": "property '{id}' has invalid property_type '{value}' — expected one of: safety, liveness, fairness",
  "check": "field_value_constraint",
  "targetKind": "property",
  "field": "property_type",
  "constraint": { "kind": "one_of", "values": ["safety", "liveness", "fairness"] }
}
```

### Progress

- [ ] Add W059 validation rule to `extensions/formal/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## 3.5: type.kind -- add enum constraint

**File**: `extensions/software/manifest.json`
**Severity**: MEDIUM
**Axes improved**: 3, 4

### Problem

The `type.kind` field accepts any string, but its description says "struct, enum, alias, opaque." These are the four supported type categories.

### Fix

Add a validation rule to `extensions/software/manifest.json` `validationRules` array:

```json
{
  "code": "W011",
  "severity": "warning",
  "messageTemplate": "type '{id}' has invalid kind '{value}' — expected one of: struct, enum, alias, opaque",
  "check": "field_value_constraint",
  "targetKind": "type",
  "field": "kind",
  "constraint": { "kind": "one_of", "values": ["struct", "enum", "alias", "opaque"] }
}
```

Diagnostic code W011 is the next available in `@specforge/software`'s range (W001-W010 are allocated; W011 is free).

### Progress

- [ ] Add W011 validation rule to `extensions/software/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## 3.6: decision.status -- add enum constraint

**File**: `extensions/governance/manifest.json`
**Severity**: MEDIUM
**Axes improved**: 3, 4, 14 (Governance)

### Problem

The `decision.status` field description says "proposed, accepted, deprecated" but has no validation rule. ADR status values are well-defined in the industry (proposed, accepted, deprecated, superseded).

### Fix

Add a validation rule to `extensions/governance/manifest.json` `validationRules` array:

```json
{
  "code": "W050",
  "severity": "warning",
  "messageTemplate": "decision '{id}' has invalid status '{value}' — expected one of: proposed, accepted, deprecated, superseded",
  "check": "field_value_constraint",
  "targetKind": "decision",
  "field": "status",
  "constraint": { "kind": "one_of", "values": ["proposed", "accepted", "deprecated", "superseded"] }
}
```

Diagnostic code W050 is a new code for `@specforge/governance` (no existing codes allocated).

### Progress

- [ ] Add W050 validation rule to `extensions/governance/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## 3.7: failure_mode severity/occurrence/detection -- add enum constraints

**File**: `extensions/governance/manifest.json`
**Severity**: LOW
**Axes improved**: 3, 4, 14

### Problem

The `failure_mode` entity has three rating fields (`severity`, `occurrence`, `detection`) and their post-mitigation counterparts (`post_severity`, `post_occurrence`, `post_detection`). These use a standard FMEA 1-10 scale or qualitative labels, but have no constraints. The `severity` description says "critical, high, medium, low" which is a reasonable qualitative enum.

### Fix

Add validation rules for the three primary rating fields. The post-mitigation fields share the same allowed values.

```json
{
  "code": "W051",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid severity '{value}' — expected one of: critical, high, medium, low",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "severity",
  "constraint": { "kind": "one_of", "values": ["critical", "high", "medium", "low"] }
},
{
  "code": "W051",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid post_severity '{value}' — expected one of: critical, high, medium, low",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "post_severity",
  "constraint": { "kind": "one_of", "values": ["critical", "high", "medium", "low"] }
},
{
  "code": "W052",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid occurrence '{value}' — expected one of: certain, likely, occasional, unlikely, rare",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "occurrence",
  "constraint": { "kind": "one_of", "values": ["certain", "likely", "occasional", "unlikely", "rare"] }
},
{
  "code": "W052",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid post_occurrence '{value}' — expected one of: certain, likely, occasional, unlikely, rare",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "post_occurrence",
  "constraint": { "kind": "one_of", "values": ["certain", "likely", "occasional", "unlikely", "rare"] }
},
{
  "code": "W053",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid detection '{value}' — expected one of: certain, likely, moderate, unlikely, undetectable",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "detection",
  "constraint": { "kind": "one_of", "values": ["certain", "likely", "moderate", "unlikely", "undetectable"] }
},
{
  "code": "W053",
  "severity": "warning",
  "messageTemplate": "failure_mode '{id}' has invalid post_detection '{value}' — expected one of: certain, likely, moderate, unlikely, undetectable",
  "check": "field_value_constraint",
  "targetKind": "failure_mode",
  "field": "post_detection",
  "constraint": { "kind": "one_of", "values": ["certain", "likely", "moderate", "unlikely", "undetectable"] }
}
```

### Progress

- [ ] Add W051, W052, W053 validation rules to `extensions/governance/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## 3.8: roadmap.status -- add enum constraint

**File**: `extensions/product/manifest.json`
**Severity**: LOW
**Axes improved**: 3, 4

### Problem

The `roadmap.status` field has no enum constraint. Reasonable values are: draft, active, completed, archived.

### Fix

Add a validation rule to `extensions/product/manifest.json` `validationRules` array:

```json
{
  "code": "W098",
  "severity": "warning",
  "messageTemplate": "roadmap '{id}' has invalid status '{value}' — expected one of: draft, active, completed, archived",
  "check": "field_value_constraint",
  "targetKind": "roadmap",
  "field": "status",
  "constraint": { "kind": "one_of", "values": ["draft", "active", "completed", "archived"] }
}
```

Code W098 continues after existing product codes W096-W097.

### Progress

- [ ] Add W098 validation rule to `extensions/product/manifest.json`
- [ ] Update affected Rust test fixtures and snapshots

---

## Fields Audited and Confirmed Correct

The following fields were audited and determined NOT to need changes:

| Entity | Field | Type | Reason to keep |
|--------|-------|------|----------------|
| event | channel | string | This is a pub/sub topic name (e.g., "order-events"), NOT a reference to the product `channel` entity kind (which represents business distribution channels like web/mobile). Semantically distinct concepts that share a word. |
| process | composition | string | CSP composition operators are free-form expressions (e.g., "P ||| Q"), not references. Correct as string. |
| module | family | string | No "family" entity kind exists. Free-form categorization string is appropriate. |
| library | family | string | Same as module.family. |
| constraint | scope | string | Free-form scope description (e.g., "system-wide", "per-request"). Not an entity reference. |
| constraint | category | string | Free-form classification (e.g., "performance", "security"). No entity kind for this. |
| behavior | category | string | Agent routing tag. Intentionally free-form for flexibility. |
| behavior | status | string | No constraint added here because behavior status is intentionally open-ended (lifecycle varies by project). Unlike feature/milestone/deliverable which have product-standard lifecycles, behaviors may use project-specific states. If a constraint is desired later, use W012. |
| port | direction | string | Already has implicit validation via Wasm function. Values: inbound, outbound, bidirectional. Could add a `one_of` constraint in a future pass but low priority since the Wasm validator already covers this. |
| condition | strength | string | Values "must/should/may" are well-defined but come from RFC 2119. Intentionally not constrained to allow domain-specific strength levels. |

---

## Change Summary

### New edge types (2)

| Label | Source | Target | Style | Color | File |
|-------|--------|--------|-------|-------|------|
| CapabilityPersona | capability | persona | solid | #E91E63 | product |
| CapabilityChannel | capability | channel | solid | #00BCD4 | product |

### New validation rules (up to 12)

| Code | Extension | Kind | Field | Constraint |
|------|-----------|------|-------|------------|
| W011 | software | type | kind | one_of: struct, enum, alias, opaque |
| W050 | governance | decision | status | one_of: proposed, accepted, deprecated, superseded |
| W051 | governance | failure_mode | severity, post_severity | one_of: critical, high, medium, low |
| W052 | governance | failure_mode | occurrence, post_occurrence | one_of: certain, likely, occasional, unlikely, rare |
| W053 | governance | failure_mode | detection, post_detection | one_of: certain, likely, moderate, unlikely, undetectable |
| W058 | formal | condition | kind | one_of: precondition, postcondition, invariant |
| W059 | formal | property | property_type | one_of: safety, liveness, fairness |
| W098 | product | roadmap | status | one_of: draft, active, completed, archived |

### Field type changes (2)

| Entity | Field | Old Type | New Type | File |
|--------|-------|----------|----------|------|
| capability | persona | string | reference | product |
| capability | surface | string_list | reference_list (renamed to `channels`) | product |

### Validation message updates (2)

| Code | Change |
|------|--------|
| I046 | Add "or capability" to unreferenced persona message |
| I047 | Add "or capability" to unreferenced channel message |

---

## Execution Order

1. **3.1** (capability.persona) -- confirmed fix, no ambiguity
2. **3.2** (capability.surface -> channels) -- recommended fix, review decision before applying
3. **3.3** (condition.kind constraint) -- additive, no breakage
4. **3.4** (property.property_type constraint) -- additive, no breakage
5. **3.5** (type.kind constraint) -- additive, no breakage
6. **3.6** (decision.status constraint) -- additive, no breakage
7. **3.7** (failure_mode rating constraints) -- additive, no breakage
8. **3.8** (roadmap.status constraint) -- additive, no breakage
9. **Test updates** -- run full test suite after all manifest changes

Sections 3.3--3.8 are independent and can be applied in any order or in parallel.

---

## Test Impact

### Rust crates affected

- **specforge-registry** -- manifest loading tests that count edge types, validation rules, and field definitions. Files:
  - `crates/specforge-registry/tests/software_manifest.rs`
  - `crates/specforge-registry/tests/zero_entity_registries.rs`
  - Any test asserting exact counts of edge types or validation rules for product/governance/formal manifests
- **specforge-emitter** -- schema and model tests that snapshot the full graph schema:
  - `crates/specforge-emitter/tests/schema.rs`
  - `crates/specforge-emitter/tests/main.rs`
  - Snapshot files under `crates/specforge-emitter/src/model/snapshots/` (if any)
- **specforge-graph** -- graph builder tests that assert edge type sets:
  - `crates/specforge-graph/tests/graph.rs`
- **specforge-mcp** -- MCP tool/resource tests that enumerate available edge types:
  - `crates/specforge-mcp/tests/contracts.rs`
  - `crates/specforge-mcp/tests/resources.rs`
- **specforge-validator** -- validation tests for orphan detection:
  - `crates/specforge-validator/src/orphan.rs`

### Expected snapshot updates

After applying changes, run:
```bash
cargo test
cargo insta review   # Accept snapshot updates for new edge types + validation rules
```

---

## Expert Score Impact (Projected)

| Axis | Before | After | Reason |
|------|--------|-------|--------|
| 3 (Field Completeness) | 6 | 7-8 | Enum constraints on all known-finite fields |
| 4 (Naming Consistency) | 6 | 7 | capability.persona matches journey.persona; surface renamed to channels |
| 9 (Connectivity) | 6 | 7 | Persona and channel gain capability edges |
| 10 (Cardinality) | 7 | 8 | Proper reference types enforce correct cardinality |
| 14 (Governance) | 6 | 7 | Governance fields gain enum validation |

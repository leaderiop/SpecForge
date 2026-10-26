# release

> **Declared by `@specforge/product`**, one of the builtin extensions. `release` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `release` groups the deliverables and milestones that ship together. It answers: **"What ships together?"**

Deliverables have their own versions and statuses; a release coordinates them under one version and one
changelog, and `specforge product release-completion` reads how far its deliverables have shipped.

## Syntax

```spec
release one_zero "1.0" {
  version      "1.0.0"
  status       in_progress
  deliverables [web_dashboard, cli_tool]
  milestones   [mvp_phase]
  target_date  2026-09-01
  owner        "release-team"
  changelog    "First public release."
}
```

## Fields

Every `release` has an id (the word after `release`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Semantic version identifier for this release. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of this release. |
| `status` | enum: planned, in_progress, released, recalled | Lifecycle state of the release; default `planned`. |
| `deliverables` | reference list → `deliverable` | Deliverables included in this release. |
| `milestones` | reference list → `milestone` | Milestones completed by this release. |
| `target_date` | date (`YYYY-MM-DD`) | Planned release date. |
| `release_date` | date (`YYYY-MM-DD`) | Actual date the release shipped. |
| `changelog` | string | Summary of changes included in this release. |
| `depends_on` | reference list → `release` | Prior releases that must ship before this one. |
| `owner` | string | Person or team responsible for this release. |
| `contributors` | string list | Additional people or teams contributing to this release. |
| `reason` | string | Justification for the current status or a status change. |
| `refs` | string list | External references such as issues, URLs, or documents. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [milestone](milestone.md) | `ReleaseCompletesMilestone` | `milestones` on `release` | Milestones completed by this release |
| outgoing | [release](release.md) | `ReleaseDependsOn` | `depends_on` on `release` | Prior releases that must ship before this one |
| outgoing | [deliverable](deliverable.md) | `ReleaseIncludesDeliverable` | `deliverables` on `release` | Deliverables included in this release |
| incoming | [release](release.md) | `ReleaseDependsOn` | `depends_on` on `release` | Prior releases that must ship before this one |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I080 | info | `@specforge/product` | Entity without owner |
| I082 | info | `@specforge/product` | Release without deliverables |
| I083 | info | `@specforge/product` | Release without milestones |
| I089 | info | `@specforge/product` | Recalled release without reason |
| W092 | warning | `@specforge/product` | Release dependency cycle |
| W093 | warning | `@specforge/product` | Invalid release version format |


## Status lifecycle

```
planned -> in_progress -> released -> recalled
```

- **planned**: being prepared
- **in_progress**: deliverables are being built and shipped
- **released**: everything has shipped (set `release_date`)
- **recalled**: retracted; give a `reason` (I089)

The compiler does not check the order of transitions.

## Design guidance

Create a release when several deliverables ship together and you need to track their combined completion, keep
one changelog, or check that all of them are ready before shipping. Name it for its purpose: `alpha`,
`one_zero` (phase), `q1_2026` (time), `security_patch_jan` (purpose).

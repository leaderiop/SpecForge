# release

A **release** groups deliverables and milestones for coordinated shipping. It answers "what ships together?" structurally.

> **Plugin:** `@specforge/product`
> **Testable:** no
> **Supports verify:** no

## Purpose

Deliverables have individual versions and statuses, but multiple deliverables often ship together as a coordinated release. The release entity provides:

- **Grouping**: Which deliverables and milestones belong to this release
- **Lifecycle**: planned -> in_progress -> released -> recalled
- **Completion tracking**: Aggregate deliverable shipping status
- **Version coordination**: Single version for the coordinated release
- **Changelog**: Release-level change documentation

## Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | no | What this release delivers |
| `version` | string | no | SemVer 2.0.0 version (W093 if non-conforming) |
| `status` | ReleaseStatus | no | planned, in_progress, released, recalled |
| `deliverables` | EntityId[] | no | Deliverables shipping in this release |
| `milestones` | EntityId[] | no | Milestones targeted by this release |
| `release_date` | string | no | ISO 8601 date YYYY-MM-DD (not validated) |
| `changelog` | string | no | Free-text release notes |
| `depends_on` | EntityId[] | no | Other releases that must ship first |
| `owner` | string | no | Person or team responsible |
| `contributors` | string[] | no | Additional participants |
| `reason` | string | no | Expected when status=recalled (not checked) |
| `tags` | string[] | no | Categorization tags |

## Relationships

### Outgoing edges

| Edge type | Target | Field | Description |
|-----------|--------|-------|-------------|
| `ReleaseDeliverable` | deliverable | `deliverables` | Ships this deliverable |
| `ReleaseMilestone` | milestone | `milestones` | Targets this milestone |

### Incoming edges

Releases are top-level coordination entities with no incoming edges.

## Status lifecycle

```
planned -> in_progress -> released -> recalled
```

- **planned**: Release is being prepared
- **in_progress**: Deliverables are actively being built/shipped
- **released**: All deliverables have shipped
- **recalled**: Release was retracted (terminal, requires `reason`)

Transitions are not checked by the compiler.

## Validation rules

| Code | Severity | Condition |
|------|----------|-----------|
| W092 | warning | Circular release dependency |
| W093 | warning | Non-SemVer `version` |

## Queries

| Query | Returns | Description |
|-------|---------|-------------|
| `queryReleaseDeliverables(id)` | `ReleaseDeliverablePayload` | Deliverables in this release |
| `queryReleaseMilestones(id)` | `ReleaseMilestonePayload` | Milestones targeted by this release |
| `queryReleaseCompletion(id)` | `ReleaseCompletionPayload` | Aggregate deliverable completion |

## Design guidance

### When to create a release

Create a release when multiple deliverables ship together and you need to:
- Track coordinated completion across deliverables
- Maintain a unified changelog
- Validate that all deliverables are ready before shipping

### Naming convention

Use descriptive names that communicate the release purpose:
- `alpha`, `beta`, `one_zero` -- phase-based
- `q1_2026`, `sprint_42` -- time-based
- `security_patch_jan` -- purpose-based

# milestone

> **Declared by `@specforge/product`**, one of the builtin extensions. `milestone` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `milestone` is a planning phase: the features and modules it schedules, and the exit criteria that say when
it is done. It answers: **"When does this ship, and what must be true before it does?"**

Because a milestone names its features, its progress is read from the graph: `specforge product
milestone-completion` counts the features that are `done`.

## Syntax

```spec
milestone mvp_phase "Phase 1: Core" {
  status        in_progress
  features      [user_management, password_auth]
  target_date   2026-06-30
  owner         "platform-team"

  exit_criteria [
    "All scheduled features implemented",
    "specforge check --strict passes",
  ]
}
```

## Fields

Every `milestone` has an id (the word after `milestone`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists and the extensions that enhance it add. A field no loaded extension declares is reported (W020).

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of the milestone. |
| `status` | enum: planned, in_progress, completed, blocked | Lifecycle state; default `planned`. |
| `features` | reference list → `feature` | Features that must be delivered to complete this milestone. |
| `exit_criteria` | string list | Conditions that must be met to consider the milestone complete. |
| `target_date` | date (`YYYY-MM-DD`) | Planned completion date for this milestone. |
| `start_date` | date (`YYYY-MM-DD`) | Date when work on this milestone began or is planned to begin. |
| `modules` | reference list → `module` | Modules scoped to this milestone. |
| `depends_on` | reference list → `milestone` | Other milestones that must be completed before this one. |
| `blockers` | string list | Outstanding issues preventing progress on this milestone. |
| `priority` | enum: critical, high, medium, low | Importance level. |
| `reason` | string | Justification for the current status or a status change. |
| `owner` | string | Person or team responsible for this milestone. |
| `contributors` | string list | Additional people or teams contributing to this milestone. |
| `refs` | string list | External references such as issues, URLs, or documents. |
| `behaviors` | reference list → `behavior` | Behaviors this milestone includes in its delivery scope (added by `@specforge/software`). |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [feature](feature.md) | `MilestoneDeliversFeature` | `features` on `milestone` | Features that must be delivered to complete this milestone |
| outgoing | [milestone](milestone.md) | `MilestoneDependsOn` | `depends_on` on `milestone` | Other milestones that must be completed before this one |
| outgoing | [behavior](behavior.md) | `MilestoneIncludesBehavior` | `behaviors` on `milestone` | Milestone delivers a behavior (cross-extension enhancement edge) |
| outgoing | [module](module.md) | `MilestoneScopesModule` | `modules` on `milestone` | Modules scoped to this milestone |
| incoming | [deliverable](deliverable.md) | `DeliverableTrackedByMilestone` | `milestones` on `deliverable` | Milestones this deliverable is tracked under |
| incoming | [milestone](milestone.md) | `MilestoneDependsOn` | `depends_on` on `milestone` | Other milestones that must be completed before this one |
| incoming | [release](release.md) | `ReleaseCompletesMilestone` | `milestones` on `release` | Milestones completed by this release |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| E010 | error | `@specforge/software` | Invalid milestone behavior range |
| E015 | error | `@specforge/product` | Milestone dependency cycle |
| I057 | info | `@specforge/product` | Blocked milestone without dependencies |
| I060 | info | `@specforge/product` | Blocked milestone missing blockers |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I080 | info | `@specforge/product` | Entity without owner |
| W049 | warning | `@specforge/product` | Milestone without features |
| W057 | warning | `@specforge/product` | Missing milestone exit criteria |


## Design guidance

### Granularity

A milestone should be **time-bounded** (a sprint, a phase, a quarter), **verifiable** (concrete exit criteria)
and **progressive** (it builds on earlier milestones through `depends_on`).

### Status values

| Value | Meaning |
|-------|---------|
| `planned` | Not yet started |
| `in_progress` | Being worked on |
| `completed` | Features done, exit criteria met |
| `blocked` | Cannot proceed: name what blocks it in `blockers` and `depends_on` (I060, I057) |

```
planned -> in_progress -> completed
                |
             blocked
```

### Exit criteria

Make exit criteria **measurable** ("specforge check --strict passes"), **specific** (name the features or
metrics) and, where you can, **anchored** to the graph ("all features in [user_management] done"). A
`completed` milestone without exit criteria is W057.

## Examples

### Planned, with a dependency

```spec
milestone search_analytics "Search & Analytics" {
  status        planned
  depends_on    [mvp_phase]
  features      [full_text_search, analytics_dashboard]
  start_date    2026-07-01
  target_date   2026-09-30

  exit_criteria [
    "Full-text search returns results in under 200ms",
    "The analytics dashboard renders within 3 seconds",
  ]
}
```

### Blocked

```spec
milestone launch_campaign "Launch Campaign" {
  status     blocked
  depends_on [brand_refresh]
  features   [loyalty_programme]
  blockers   ["New brand guidelines not yet approved"]
}
```

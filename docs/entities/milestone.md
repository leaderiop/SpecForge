# milestone

> **Module:** `@specforge/product`

## Purpose

A `milestone` declares a **planning phase** that schedules features and modules with exit criteria. Milestones provide the temporal dimension — when things ship — and connect planning to the compiler-validated traceability chain.

It answers: **"When does this ship and what must be true before it ships?"**

Milestones reference features and modules, making it possible to validate that a milestone is complete by checking if all features are implemented and all exit criteria are met.

## ID Pattern

```
identifier
```

Examples: `mvp_phase`, `search_analytics`, `foundation`

## Syntax

```spec
use "features/user-management"
use "features/auth"

milestone mvp_phase "Phase 1: Core" {
  status        in_progress
  features      [user_management, password_auth]
  exit_criteria [
    "All scheduled features implemented",
    "specforge check --strict passes",
  ]
  target_date   "2026-06-30"
}
```

## Fields

All fields are optional at the type level. The compiler emits warnings and info diagnostics when key fields are absent or inconsistent.

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). |
| `status` | MilestoneStatus | Current phase status: `planned`, `in_progress`, `completed`, `blocked`. Validated by W080. |
| `features` | EntityId[] | Features scheduled for this phase. Creates `MilestoneFeature` edges. |
| `modules` | EntityId[] | Modules included in this phase. Creates `MilestoneModule` edges. |
| `depends_on` | EntityId[] | Other milestones that must complete before this phase can start. Creates `MilestoneDependsOn` edges. Cycles detected by E015. |
| `exit_criteria` | string[] | Exit criteria that must be satisfied before the phase is considered complete. Completed milestones without criteria emit W057. |
| `target_date` | string | Target completion date in ISO 8601 format (`YYYY-MM-DD`, e.g., `2026-06-30`). Validated by regex `^\d{4}-\d{2}-\d{2}$`. Non-conforming formats emit I053. |
| `priority` | Priority | Importance level: `critical`, `high`, `medium`, `low`. |
| `reason` | string | Rationale for current status (expected when `blocked`, checked by I060). |
| `tags` | string[] | Free-form tags for categorization. Format validated by I068 (lowercase hyphen-separated, 2-50 chars). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this milestone. |

> **Note:** To schedule individual behaviors within a milestone, install `@specforge/software`. The `MilestoneBehavior` edge type is provided by that extension via entity_enhancement, not by `@specforge/product`.

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `MilestoneFeature` | "This phase schedules these features" |
| `module` | `MilestoneModule` | "This phase includes these modules" |
| `milestone` | `MilestoneDependsOn` | "This phase depends on that phase completing first" |
| `ref` | `links_to` | "This milestone links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `milestone` | `MilestoneDependsOn` | "Another phase depends on this phase" |
| `deliverable` | `DeliverableMilestone` | "A deliverable tracks against this milestone" |

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| E015 | error | Circular milestone dependency — `depends_on` edges between milestones form a cycle. |
| W049 | warning | Empty milestone — no features AND no modules. |
| W057 | warning | Completed milestone without exit criteria. |
| W080 | warning | Invalid `status` value (not in MilestoneStatus enum). |
| I051 | info | Milestone features not reachable from milestone modules (gap). |
| I053 | info | Invalid `target_date` format (not ISO 8601 `YYYY-MM-DD`). |
| I057 | info | Blocked milestone without a `depends_on` reference. |
| I060 | info | Blocked milestone without a `reason`. |
| I064 | info | Milestone temporal inconsistency (depends on a later-dated milestone). |
| I075 | info | Exit criterion not anchored to a graph entity — purely prose criteria are valid but weaker for automated verification. |

## Design Guidance

### Phase Granularity

A milestone should be:
- **Time-bounded** — represents a planning period (sprint, milestone, release)
- **Verifiable** — has concrete exit criteria that can be checked
- **Progressive** — builds on previous milestones

### Status Values

| Value | Meaning |
|-------|---------|
| `planned` | Not yet started |
| `in_progress` | Actively being worked on |
| `completed` | All features done, exit criteria met |
| `blocked` | Cannot proceed (should include `reason` and `depends_on`) |

A typical lifecycle:

```
planned -> in_progress -> completed
                |
              blocked (requires reason + depends_on)
```

### Exit Criteria

Exit criteria should be:
- **Measurable** — "specforge check --strict passes", "coverage >= 90%"
- **Specific** — name the behaviors, features, or metrics
- **Automated where possible** — the compiler can verify behavior coverage and test presence
- **Anchored** — reference entity IDs (e.g., "all features in [user_management] done") to enable automated verification. Prose-only criteria are valid but emit I075, encouraging anchoring to graph-verifiable properties

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `MilestoneFeature` | Features scheduled in this phase |
| outgoing | [module](module.md) | `MilestoneModule` | Modules included in this phase |
| outgoing | [milestone](milestone.md) | `MilestoneDependsOn` | Phases this phase depends on |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this milestone |
| incoming | [milestone](milestone.md) | `MilestoneDependsOn` | Phases that depend on this phase |
| incoming | [deliverable](deliverable.md) | `DeliverableMilestone` | Deliverables tracking against this milestone |

## Examples

### Active Phase

```spec
use "features/user-management"
use "features/auth"

milestone mvp_phase "MVP" {
  status        in_progress
  depends_on    [foundation]
  features      [user_management, password_auth, user_profile]
  priority      high

  exit_criteria [
    "All scheduled features implemented",
    "Coverage >= 90%",
    "Zero open E-level diagnostics",
  ]

  refs [jira.epic:PROJ-001]
}
```

### Planned Phase

```spec
use "features/search"
use "features/analytics"

milestone search_analytics "Search & Analytics" {
  status        planned
  depends_on    [mvp_phase]
  features      [full_text_search, analytics_dashboard]
  target_date   "2026-09-30"

  exit_criteria [
    "Full-text search returns results in < 200ms",
    "Analytics dashboard renders within 3 seconds",
  ]
}
```

### Completed Phase

```spec
milestone foundation "Foundation" {
  status        completed
  features      [project_scaffold]

  exit_criteria [
    "Project scaffold complete",
    "CI/CD pipeline operational",
    "specforge init passes",
  ]
}
```

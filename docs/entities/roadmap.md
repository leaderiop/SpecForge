# roadmap

> **Module:** `@specforge/product`

## Purpose

A `roadmap` declares a **planning phase** that schedules features and deliverables with exit criteria. Roadmap phases provide the temporal dimension — when things ship — and connect planning to the compiler-validated traceability chain.

It answers: **"When does this ship and what must be true before it ships?"**

Roadmap phases reference behaviors, features, and deliverables, making it possible to validate that a phase is complete by checking if all referenced behaviors pass, all features are implemented, and all exit criteria are met.

## ID Pattern

```
identifier
```

Examples: `mvp_phase`, `search_analytics`, `foundation`

## Syntax

```spec
use features/user-management
use features/auth

roadmap mvp_phase "Phase 1: Core" {
  status    in_progress
  behaviors [create_user, read_user, update_email, place_order]
  features  [user_management, password_auth]
  criteria  [
    "All BEH covered by tests",
    "specforge check --strict passes",
  ]
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `status` | enum | Current phase status: `planned`, `in_progress`, `completed`, `blocked`. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). |
| `behaviors` | reference list | Explicit list of behaviors scheduled for this phase. |
| `features` | reference list | Features scheduled for this phase. |
| `criteria` | string list | Exit criteria that must be satisfied before the phase is considered complete. |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this roadmap phase. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `schedules` | "This phase schedules these features" |
| `deliverable` | `schedules` | "This phase schedules these deliverables" |
| `ref` | `links_to` | "This roadmap phase links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `feature` | `roadmap` | "A feature is planned for this phase" |
| `deliverable` | `roadmap` | "A deliverable is planned for this phase" |

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` must resolve to an existing `feature`. |
| E001 | Every behavior ID in `behaviors` must resolve to an existing behavior. |
| E002 | No two roadmap phases may share the same ID. |

## Design Guidance

### Phase Granularity

A roadmap phase should be:
- **Time-bounded** — represents a planning period (sprint, milestone, release)
- **Verifiable** — has concrete exit criteria that can be checked
- **Progressive** — builds on previous phases

### Status Lifecycle

```
planned → in_progress → completed
                ↓
              blocked
```

- `planned` — Phase is scheduled but work has not started
- `in_progress` — Active development
- `completed` — All exit criteria met
- `blocked` — Waiting on external dependency or decision

### Exit Criteria

Exit criteria should be:
- **Measurable** — "specforge check --strict passes", "coverage >= 90%"
- **Specific** — name the behaviors, features, or metrics
- **Automated where possible** — the compiler can verify behavior coverage and test presence

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `schedules` | Features scheduled in this phase |
| outgoing | [deliverable](deliverable.md) | `schedules` | Deliverables scheduled in this phase |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this roadmap phase |

## Examples

### Active Phase

```spec
use features/user-management
use features/auth

roadmap mvp_phase "MVP" {
  status     in_progress
  behaviors  [create_user, read_user, update_email, place_order]
  features   [user_management, password_auth, user_profile]

  criteria [
    "All scheduled behaviors passing",
    "Coverage >= 90%",
    "Zero open E-level diagnostics",
  ]

  refs [jira.epic:PROJ-001]
}
```

### Planned Phase

```spec
use features/search
use features/analytics

roadmap search_analytics "Search & Analytics" {
  status    planned
  features  [full_text_search, analytics_dashboard]

  criteria [
    "Full-text search returns results in < 200ms",
    "Analytics dashboard renders within 3 seconds",
  ]
}
```

### Completed Phase

```spec
roadmap foundation "Foundation" {
  status     completed
  features   [project_scaffold]

  criteria [
    "Project scaffold complete",
    "CI/CD pipeline operational",
    "specforge init passes",
  ]
}
```

---
name: specforge-roadmaps-dsl
description: "Write roadmap blocks in .spec DSL files (@specforge/product plugin). Each roadmap declares a planning phase with free-form snake_case IDs, status lifecycle, behavior and feature scheduling, and exit criteria. Use when defining when things ship and what must be true before a phase is complete."
---

# SpecForge Roadmaps DSL

Rules and conventions for authoring **`roadmap` blocks** in `.spec` files. Roadmap phases provide the temporal dimension -- when things ship -- with verifiable exit criteria.

**Requires:** `@specforge/product` plugin.

## When to Use

- Defining planning phases (MVP, v2, etc.) with verifiable exit criteria
- Scheduling features and deliverables into release milestones
- Tracking behavior coverage by explicit references
- Connecting planning to the compiler-validated traceability chain

## Block Syntax

```spec
use features/user-management
use features/auth

roadmap mvp "Phase 1: Core" {
  status    in_progress
  behaviors [create_user, read_user_by_id, update_user_email, authenticate_user, hash_password]
  features  [user_management, password_authentication]
  criteria  [
    "All behaviors covered by tests",
    "specforge check --strict passes",
  ]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `status` | enum | `planned`, `in_progress`, `completed`, `blocked`. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `behaviors` | reference list | Individual behavior entity IDs included in this phase. |
| `features` | reference list | Features scheduled for this phase. |
| `criteria` | string list | Exit criteria for phase completion. |
| `refs` | reference list | External references linked to this roadmap phase. |

### Status Lifecycle

```
planned -> in_progress -> completed
                |
              blocked
```

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `schedules` | Phase schedules these features |
| `deliverable` | `schedules` | Phase schedules these deliverables |
| `ref` | `links_to` | External references linked to this phase |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `feature` | `roadmap` | Feature planned for this phase |
| `deliverable` | `roadmap` | Deliverable planned for this phase |

## Writing Rules

1. **Verifiable criteria** -- "specforge check --strict passes", "coverage >= 90%", not vague goals.
2. **Reference behaviors individually** -- list specific behavior entity IDs, not numeric ranges.
3. **Progressive phases** -- each phase builds on previous ones.
4. **Import feature files** -- `use` the files that declare referenced features.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` and `behaviors` must resolve. |
| E002 | No duplicate roadmap IDs. |

## Examples

### Active Phase

```spec
roadmap mvp "MVP" {
  status     in_progress
  behaviors  [create_user, read_user_by_id, update_user_email, delete_user, authenticate_user]
  features   [user_management, password_authentication, basic_search]

  criteria [
    "All MVP behaviors passing",
    "Coverage >= 90%",
    "Zero open E-level diagnostics",
  ]

  refs [jira.epic:PROJ-001]
}
```

### Planned Phase

```spec
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

## What NOT to Do

- Do not write roadmaps without the `@specforge/product` plugin installed
- Do not set vague criteria like "system is ready" -- use measurable, automatable checks
- Do not reference features from other files without a `use` import

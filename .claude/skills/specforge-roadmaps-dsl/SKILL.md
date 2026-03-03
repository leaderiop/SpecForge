---
name: specforge-roadmaps-dsl
description: "Write roadmap blocks in .spec DSL files (@specforge/product plugin). Each roadmap declares a planning phase with RM-{n} IDs, status lifecycle, behavior ranges, feature scheduling, and exit criteria. Use when defining when things ship and what must be true before a phase is complete."
---

# SpecForge Roadmaps DSL

Rules and conventions for authoring **`roadmap` blocks** in `.spec` files. Roadmap phases provide the temporal dimension — when things ship — with verifiable exit criteria.

**Requires:** `@specforge/product` plugin.

## When to Use

- Defining planning phases (MVP, v2, etc.) with verifiable exit criteria
- Scheduling features and deliverables into release milestones
- Tracking behavior coverage by range
- Connecting planning to the compiler-validated traceability chain

## Block Syntax

```spec
use features/user-management
use features/auth

roadmap RM-01 "Phase 1: Core" {
  status    in_progress
  behaviors [1, 8]
  features  [FEAT-MS-001, FEAT-MS-002]
  criteria  [
    "All BEH covered by tests",
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
| `behaviors` | range or list | Behavior range `[start, end]` or explicit list. Range expands using project infix. |
| `features` | reference list | Features scheduled for this phase. |
| `criteria` | string list | Exit criteria for phase completion. |
| `refs` | reference list | External references linked to this roadmap phase. |

### Behavior Ranges

```spec
behaviors [1, 50]  // expands to BEH-{infix}-001 through BEH-{infix}-050
```

The range uses the project infix from the `spec` root block.

### Status Lifecycle

```
planned → in_progress → completed
                ↓
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

1. **Project-wide IDs** — `RM-{n}` omits the infix (roadmap phases are project-wide, not domain-scoped).
2. **Verifiable criteria** — "specforge check --strict passes", "coverage >= 90%", not vague goals.
3. **Behavior ranges are inclusive** — `[1, 50]` includes both BEH-{infix}-001 and BEH-{infix}-050.
4. **Progressive phases** — each phase builds on previous ones.
5. **Import feature files** — `use` the files that declare referenced features.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` and `behaviors` must resolve. |
| E002 | No duplicate roadmap IDs. |
| E010 | Behavior range invalid — start > end, or range contains non-existent behaviors. |

## Examples

### Active Phase

```spec
roadmap RM-01 "MVP" {
  status     in_progress
  behaviors  [1, 50]
  features   [FEAT-MS-001, FEAT-MS-002, FEAT-MS-003]

  criteria [
    "All BEH-MS-001 through BEH-MS-050 passing",
    "Coverage >= 90%",
    "Zero open E-level diagnostics",
  ]

  refs [jira.epic:PROJ-001]
}
```

### Planned Phase

```spec
roadmap RM-02 "Search & Analytics" {
  status    planned
  features  [FEAT-MS-005, FEAT-MS-006]

  criteria [
    "Full-text search returns results in < 200ms",
    "Analytics dashboard renders within 3 seconds",
  ]
}
```

### Completed Phase

```spec
roadmap RM-00 "Foundation" {
  status     completed
  features   [FEAT-MS-000]

  criteria [
    "Project scaffold complete",
    "CI/CD pipeline operational",
    "specforge init passes",
  ]
}
```

## What NOT to Do

- Do not write roadmaps without the `@specforge/product` plugin installed
- Do not use behavior ranges where start > end
- Do not set vague criteria like "system is ready" — use measurable, automatable checks
- Do not reference features from other files without a `use` import
- Do not add an infix to roadmap IDs — use `RM-01`, not `RM-MS-01`

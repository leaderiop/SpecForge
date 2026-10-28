# Project, requirements, milestones, phases

```
 project ─1──*─ requirement ─*──1─ phase           (each v1 requirement maps to exactly one phase)
    │
    ├─1──*─ milestone ─1──*─ phase ─*──*─ phase     (depends_on: a DAG, earlier phases only)
    │           └─0..1─ retrospective ─1──*─ lesson
    ├─0..*─ workstream (a scope: its own milestones and phases)
    └─0..*─ methodology
```

Files: `spec/gsd/project.spec`, `spec/gsd/requirements/<category>.spec`, `spec/gsd/milestones/<v>/milestone.spec`,
`spec/gsd/milestones/<v>/phases/<NN-slug>/phase.spec`.

## project

Singleton. GSD: `PROJECT.md`. Name: any word, by convention the slug of its `name`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| name | str | R | `# [Project Name]` |
| what | text | R | `## What This Is` |
| core_value | str | R | `## Core Value` |
| customer, revenue_model, success_metric | str | | `## Business Context` |
| context | text | R | `## Context` |
| constraints | {kind enum{tech_stack, timeline, budget, dependencies, compatibility, performance, security, other}, what str, why str} | R | `## Constraints` |
| out_of_scope | {item str, why str R} | | `### Out of Scope` |
| key_decisions | →* decision | | `## Key Decisions` |
| project_code | str `^[A-Z][A-Z0-9_]*$` | | config `project_code` |
| validated, active | →* requirement | C | `### Validated` / `### Active`: from requirement status |
| updated | ts | C | `*Last updated*`: the latest `at` among the recorded commits and the `completed` of the summaries (the git collector records commits as entities; a pass sees no git history); uncommitted edits do not move it |
| windows_open, windows_waived, windows_fixed, windows_total | int | C | the broken-windows ledger counts ([verify.md](verify.md#broken_window)) |

## requirement

GSD: one bullet of `REQUIREMENTS.md`. Name: `AUTH-01`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| text | str | R | description after the ID |
| category | str | R | `### [Category]` |
| tier | enum{v1, v2, out_of_scope} | R | `## v1` / `## v2` / `## Out of Scope` |
| reason | str | | Out of Scope `Reason` (required when `out_of_scope`) |
| phase | → phase | | Traceability `Phase` |
| defined | date | I | `**Defined:**` |
| status | enum{pending, in_progress, complete, blocked, gaps_found} | C | Traceability `Status`: complete when every plan carrying it has a complete summary listing it; `gaps_found` when a verification of its phase does |

## milestone

GSD: a milestone section of `ROADMAP.md`, `MILESTONES.md`, `milestones/<v>-*`. Name: `v1.0`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| name | str | R | `MVP` |
| delivered | str | | `**Delivered:**` |
| accomplishments | strs | | `**Key accomplishments:**` |
| whats_next | str | | `**What's next:**` |
| abandoned | {reason str R, ruled_by str R, at date R} | | gsd-path's abandon ruling |
| workstream | → workstream | I | the workstream it belongs to (K2: it scopes the milestone, `auth-rework/v1.0`); absent outside one, and the milestone keeps its short id |
| completion | {at date, audit → milestone_audit} | H | written by `/gsd-complete-milestone` once its audit passes |
| state | enum{planned, in_progress, shipped, abandoned} | C | ✅ 🚧 📋: from its phases and `/gsd-complete-milestone`'s record |
| shipped | date | C | `(Shipped: …)`: the completion record's date |
| stats | {files int, loc int, phases int, plans int, tasks int, days int} | C | `**Stats:**`: from the graph and the recorded commits (`loc` sums their `insertions` and `deletions`) |
| git_range | str | C | `**Git range:**`: from the recorded commits (first and last hash) |

GSD's archive copies (`milestones/<v>-ROADMAP.md` …) have no counterpart: a shipped milestone's entities stay where
they are, and `specforge query` answers what an archive answered.

## phase

GSD: `### Phase N: Name` in `ROADMAP.md` and its directory. Name: `03`, `03.1` (inserted), `999.1` (backlog).
Scoped by milestone.

| Field | Type | Flags | GSD |
|---|---|---|---|
| milestone | → milestone | R I | the section it sits in |
| name | str | R | heading text |
| slug | str | R | directory `NN-slug` |
| goal | str | R | `**Goal**:` |
| depends_on | →* phase | | `**Depends on**:` |
| requirements | →* requirement | | `**Requirements**:` |
| success_criteria | strs | R | `**Success Criteria**` (2–5) |
| mode | enum{standard, mvp} | | `**Mode**:` (`/gsd-mvp-phase`) |
| tag | str | | the parenthetical, `(Cluster B)` |
| inserted | bool | C | `(INSERTED)`: a decimal name |
| backlog | bool | C | a `999.x` name |
| frontend | bool | C | the ui gate's `frontend` detection |
| status | enum{not_started, planned, in_progress, executed, needs_review, complete, deferred} | C | the GSD phase-status ladder, from plan and summary counts and verification status |
| plans_complete | str `N/M` | C | Progress `Plans Complete` |
| completed | date | C | Progress `Completed` |

## workstream

GSD: `.planning/workstreams/<name>/`. A scope (K2): its milestones, phases and plans are unique within it.

| Field | Type | Flags | GSD |
|---|---|---|---|
| purpose | str | R | the workstream's description |
| status | enum{active, complete} | C | all its milestones shipped |

Which workstream is active is session state, not a field ([execute.md](execute.md#session-state)).

## retrospective and lesson

**retrospective**: one `## Milestone:` section of `RETROSPECTIVE.md`. Name `v1.0-RETROSPECTIVE`.
`milestone → milestone R I`, `built strs`, `worked strs`, `inefficient strs`, `patterns strs`,
`cost {model_mix str, sessions int, notable str}`; `phases`, `plans`, `sessions` are `C`. Its lessons are read from
`lesson.retrospective` (the relation's inverse is named `lessons`).

**lesson**: name `L1`, scoped by its retrospective. `retrospective → retrospective` (absent for what
`/gsd-extract-learnings` writes, which is in the project scope), `text str R`, `milestone → milestone`, `top bool`.
Also holds what `/gsd-extract-learnings` writes (`LEARNINGS.md`), with `kind enum{decision, lesson, pattern, surprise}`.

## methodology

GSD: `METHODOLOGY.md`. Name `M1`. `name str R`, `diagnoses str`, `recommends str`, `apply_when str`,
`status enum{active, superseded}`, `superseded_by → methodology`.

## Computed, project-wide

What `STATE.md` holds that can be derived is never stored; `specforge query` and the `gsd_progress` tool answer it:

| GSD `STATE.md` | Derived from |
|---|---|
| `progress{total_phases, completed_phases, total_plans, completed_plans, percent}` | phase and plan status |
| `milestone`, `current_phase`, `status` (planning, executing, verifying…) | the first phase not complete, and its status |
| `### Decisions` (recent) | decisions ordered by the edit plan that wrote them |
| `### Pending Todos` | ops todos not done |
| `## Performance Metrics` | summary `duration` and the git collector |

What cannot be derived is in [execute.md](execute.md): the committed pause, blockers and handoff, and the session
state that is never committed.

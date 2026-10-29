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

Singleton: one per repository. The kernel keeps `singleton` as metadata (it reaches the LSP hover only), so `gsd/E108` reports the second and later `project` entities. GSD: `PROJECT.md`. Name: any word, by convention the slug of its `name`.

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
| validated, active | →* requirement | C | `### Validated` (requirements with status `complete`) / `### Active` (`v1` requirements that are not); a list with no item is absent |
| updated | ts | C | `*Last updated*`: the latest `at` among the recorded commits and the `completed` of the summaries (the git collector records commits as entities; a pass sees no git history); uncommitted edits do not move it |
| windows_open, windows_waived, windows_fixed, windows_total | int | C | the broken-windows ledger counts ([verify.md](verify.md#broken_window)) |

## requirement

GSD: one bullet of `REQUIREMENTS.md`. Name: `AUTH-01`. A requirement id is unique across the whole project, workstreams included: GSD gives each workstream its own `REQUIREMENTS.md`, but SpecForge scopes only milestones and phases by workstream, and `gsd/E101` maps requirements project-wide. Two workstreams that both need `AUTH-01` name them apart (`AUTH-01`, `AUTH-R01`).

| Field | Type | Flags | GSD |
|---|---|---|---|
| text | str | R | description after the ID |
| category | str | R | `### [Category]` |
| tier | enum{v1, v2, out_of_scope} | R | `## v1` / `## v2` / `## Out of Scope` |
| reason | str | | Out of Scope `Reason` (required when `out_of_scope`) |
| defined | date | I | `**Defined:**` |
| status | enum{pending, in_progress, complete, blocked, gaps_found} | C | Traceability `Status`: complete when a plan naming it is complete (GSD marks it when the plan's summary is written, `gsd-executor.md:801-813`); blocked when it is not and every plan naming it is halted or blocked; in_progress when one is executing; else pending. `gaps_found` when the requirement would be complete and a phase it traces to has a `gaps_found` verification (`milestone.cts:453-514`) |

The requirement's phase (Traceability `Phase`) is the inverse of `phase.requirements`, read back as `phase`: a relation
is written from one side, never both (W162).

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
| completion | {at date, audit → milestone_audit} | H | written by `close_milestone` (`/gsd-complete-milestone`) once its audit passes: `audit` is the latest audit it closed on |
| state | enum{planned, in_progress, shipped, abandoned} | C | ✅ 🚧 📋: `abandoned` when `abandoned` is written, `shipped` when `completion` is, `in_progress` when a phase is past `not_started`, else `planned` |
| shipped | date | C | `(Shipped: …)`: the completion record's date |
| stats | {files int, loc int, phases int, plans int, tasks int, days int} | C | `**Stats:**`: from the graph and the recorded commits (`loc` sums their `insertions` and `deletions`; `files` counts distinct paths; `days` runs from the first commit to `completion.at`, or to the last commit); the commit-based members are absent with no commit |
| git_range | str | C | `**Git range:**`: the first and last recorded `feat` commit as `` `feat(01-01)` → `feat(01-02)` `` (the scope is the name of the plan the commit names); absent with none |
| progress | {phases int, phases_complete int, plans int, plans_complete int, percent int} | C | `STATE.md`'s `progress`, per milestone: `percent` is the smaller of plans_complete/plans and phases_complete/phases (a side with nothing counts 1, `state-document.cts:861-878`); `phases_complete` counts the phases whose verification passed, which needs r40's verdicts. `plans_complete` counts every plan with a summary, a halted one too (#2830), and the sentinel `999.x` backlog phases are left out (#3167) |

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
| requirements | →* requirement | | `**Requirements**:`; a requirement reads its phase back as `phase` |
| success_criteria | strs | R | `**Success Criteria**` (2–5) |
| mode | enum{standard, mvp} | | `**Mode**:` (`/gsd-mvp-phase`) |
| tag | str | | the parenthetical, `(Cluster B)` |
| inserted | bool | C | `(INSERTED)`: a name that holds a `.`, but not a `999.x` one |
| backlog | bool | C | a `999.x` name |
| frontend | bool | C | the ui gate's `frontend` detection (`@gsd/ui`'s; core never answers it) |
| status | enum{not_started, planned, in_progress, executed, needs_review, complete, deferred} | C | the GSD phase-status ladder (`phase-status.cts:160-196`): no plan is `not_started`, no summary `planned`, fewer summaries than plans `in_progress`, `needs_review` when the verification is `human_needed`, `complete` when it passed (`phase-status.cts:174-197`; UAT does not feed it), else `executed`. `passed` needs r40's verdicts; `deferred` is never derived |
| plans_complete | str `N/M` | C | Progress `Plans Complete`: plans with a summary over plans |
| completed | date | C | Progress `Completed` |

## workstream

GSD: `.planning/workstreams/<name>/`. A scope (K2): its milestones, phases and plans are unique within it.

| Field | Type | Flags | GSD |
|---|---|---|---|
| purpose | str | R | the workstream's description |
| status | enum{active, complete} | C | all its milestones shipped |

Which workstream is active is session state, not a field ([execute.md](execute.md#session-state)): `workstreams switch` and `resume` write `active_workstream` to the extension's project state. A command cannot read state or a computed field, so `status` does not name the active workstream and a workstream is `complete` when every milestone has a `completion`; its progress counts the plans that have a summary.

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
| `progress{total_phases, completed_phases, total_plans, completed_plans, percent}` | `milestone.progress` (phase and plan status) |
| `milestone`, `current_phase`, `status` (planning, executing, verifying…) | the first phase not complete, and its status |
| `### Decisions` (recent) | decisions ordered by the edit plan that wrote them |
| `### Pending Todos` | ops todos not done |
| `## Performance Metrics` | summary `duration` and the git collector |

What cannot be derived is in [execute.md](execute.md): the committed pause, blockers and handoff, and the session
state that is never committed.

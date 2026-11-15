# Project, requirements, milestones, phases

```
 project ─1──*─ requirement ─*──1─ phase           (each v1 requirement maps to exactly one phase)
    │
    ├─1──*─ milestone ─1──*─ phase ─*──*─ phase     (depends_on: a DAG, earlier phases only)
    │           ├─*──*─ milestone                   (depends_on: acyclic, beyond GSD)
    │           └─0..1─ retrospective ─1──*─ lesson
    ├─0..*─ workstream (a scope: its own milestones and phases)
    └─0..*─ methodology
```

Files: `spec/gsd/project.spec`, `spec/gsd/requirements/<area>.spec`, `spec/gsd/milestones/<v>/milestone.spec`,
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
| project_code | str `^[A-Z][A-Z0-9_]*$` | | config `project_code`; `gsd-git` strips the setting `project_code`, else this field, from a commit's scope (`feat(PRJ-01-01)`, #5271) |
| validated, active | →* requirement | C | `### Validated` (requirements with status `complete`) / `### Active` (`v1` requirements that are not); a list with no item is absent |
| updated | ts | C | `*Last updated*`: the latest `at` among the recorded commits and the `completed` of the summaries (the git collector records commits as entities; a pass sees no git history); uncommitted edits do not move it |
| windows_open, windows_waived, windows_fixed, windows_total | int | C | the broken-windows ledger counts ([verify.md](verify.md#broken_window)) |

## requirement

GSD: one bullet of `REQUIREMENTS.md`. Name: `AUTH-01`. A requirement id is unique across the whole project, workstreams included: GSD gives each workstream its own `REQUIREMENTS.md`, but SpecForge scopes only milestones and phases by workstream, and `gsd/E101` maps requirements project-wide. Two workstreams that both need `AUTH-01` name them apart (`AUTH-01`, `AUTH-R01`).

| Field | Type | Flags | GSD |
|---|---|---|---|
| text | str | R | description after the ID |
| area | str | R | `### [Category]` (the word `decision.area` uses) |
| tier | enum{v1, v2, out_of_scope} | R | `## v1` / `## v2` / `## Out of Scope` |
| reason | str | | Out of Scope `Reason` (required when `out_of_scope`) |
| defined | date | I | `**Defined:**` |
| category | enum{functional, performance, compatibility, usability, reliability, security, maintainability, portability, safety, regulatory} | | beyond GSD: its quality class: ISO/IEC 25010:2011's eight product quality characteristics by their short names (interoperability is part of compatibility), `safety`, which ISO/IEC 25010:2023 added (that revision also renamed usability to interaction capability and portability to flexibility; the 2011 words are kept), or `regulatory` |
| priority | enum{must, should, could} | | beyond GSD: MoSCoW, the journey's list ([product.md](product.md#journey)); "won't" is `tier out_of_scope` |
| metric | {name str R, threshold num R, unit str, direction enum{at_most, at_least, exactly} R} | | beyond GSD: the measure it is held to; the threshold is a decimal with no bounds (`direction` carries the sense), and a count has no `unit` |
| source | strs | | beyond GSD: where it came from: a person, quoted (`"Alice Martin"`), or a declared `ref`, bare (`gh.issue:42`; E003 when undeclared) |
| personas | →* persona | | beyond GSD: the personas it serves ([product.md](product.md#persona)) |
| constrains | →* requirement | | beyond GSD: the requirements it limits |
| protects | →* invariant | | beyond GSD: the invariants it guards ([governance.md](governance.md#invariant)) |
| tags | strs | | beyond GSD: words to group it by |
| effort | enum{xs, s, m, l, xl} | | beyond GSD: how large it is to build, a T-shirt size |
| status | enum{pending, in_progress, complete, blocked, gaps_found} | C | Traceability `Status`: complete when a plan naming it is complete (GSD marks it when the plan's summary is written, `gsd-executor.md:801-813`); blocked when it is not and every plan naming it is halted or blocked; in_progress when one is executing; else pending. `gaps_found` when the requirement would be complete and a phase it traces to has a `gaps_found` verification (`milestone.cts:453-514`) |
| satisfied_by | →* item | C | beyond GSD: the code items the tasks of its plans name in `items`, as written (held by a collect or not, a `manual` task's too), sorted, each once; absent when none. The items `specforge trace <id>` reaches through its plans |
| proof | enum{no_task, no_commit, not_run, no_test, failed, proven} | C | beyond GSD: how far its thread reaches, the first missing link of a task of its plans (not `manual`), a recorded commit of one, a collected test run (`not_run` without one), a passing test linked to a committed task or the requirement or named in the `proof` of a truth that names it (`no_test` with none, `failed` when a linked one failed and none passed); `proven` when whole. `gsd/E515` refuses a `complete` requirement that is not `proven` |
| shapes | enum{numeric_range, collection, text, stateful, io}* | | the spec-less edge probe's authored shapes (an empty list proposes no edge) |
| text_en | str | | an English translation the spec-less edge probe classifies instead |

The requirement's phase (Traceability `Phase`) is the inverse of `phase.requirements`, read back as `phase`: a relation
is written from one side, never both (W162). Its own relations name their reverse ([ADR 0354](../../../docs/adr/0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md)):
a persona reads `personas` back as `requirements`, a requirement reads `constrains` back as `constrained_by` (a
self-relation's named reverse, [ADR 0359](../../../docs/adr/0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md)),
and an invariant reads `protects` back as `protected_by`: that name written on the other side is E079, naming the
owning field. A requirement also reads a decision's `affects` back as `affected_by`, and a risk's `affects` as `risks`.

Why each field beyond GSD has its shape, and why `proof` is the one reading of the requirement's thread:
[ADR 0359](../../../docs/adr/0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md).

## milestone

GSD: a milestone section of `ROADMAP.md`, `MILESTONES.md`, `milestones/<v>-*`. Name: `v1.0`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| name | str | R | `MVP` |
| delivered | str | | `**Delivered:**` |
| accomplishments | strs | | `**Key accomplishments:**` |
| whats_next | str | | `**What's next:**` |
| goal | str | | the Current Milestone's `**Goal:**` (PROJECT.md), recorded by `/gsd-new-milestone` |
| target_features | strs | | the Current Milestone's `**Target features:**` (PROJECT.md) |
| exit_criteria | strs | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): what must hold to call it done |
| starts | date | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): when work on it starts |
| targets | date | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): the day it aims to ship; on or after `starts` (both written; the same day is clean), gsd/E537 |
| depends_on | →* milestone | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): the milestones that ship before it; acyclic, gsd/E102 |
| owner | str | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): the person or team that answers for it |
| contributors | strs | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): the others working on it |
| artifact_types | strs | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): the kinds of thing it ships, as open words (`cli`, `library`, `extension`, `service`); one milestone may ship several |
| tags | strs | | beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): free words |
| abandoned | {reason str R, ruled_by str R, at date R} | | gsd-path's abandon ruling |
| workstream | → workstream | I | the workstream it belongs to (K2: it scopes the milestone, `auth-rework/v1.0`); absent outside one, and the milestone keeps its short id |
| completion | {at date, audit → milestone_audit} | H | written by `close_milestone` (`/gsd-complete-milestone`) once its audit passes: `audit` is the latest audit it closed on |
| state | enum{planned, in_progress, shipped, abandoned} | C | ✅ 🚧 📋: `abandoned` when `abandoned` is written, `shipped` when `completion` is, `in_progress` when a phase is past `not_started`, else `planned` |
| first_commit | str | C | its first recorded commit, oldest by `at`; absent without one |
| shipped | date | C | `(Shipped: …)`: the completion record's date |
| stats | {files int, loc int, phases int, plans int, tasks int, days int} | C | `**Stats:**`: from the graph and the recorded commits (`loc` sums their `insertions` and `deletions`; `files` counts distinct paths; `days` runs from the first commit to `completion.at`, or to the last commit); the commit-based members are absent with no commit |
| git_range | str | C | `**Git range:**`: the first and last recorded `feat` commit as `` `feat(01-01)` → `feat(01-02)` `` (the scope is the name of the plan the commit names); absent with none |
| progress | {phases int, phases_complete int, plans int, plans_complete int, percent int} | C | `STATE.md`'s `progress`, per milestone: `percent` is the smaller of plans_complete/plans and phases_complete/phases (a side with nothing counts 1, `state-document.cts:861-878`); `phases_complete` counts the phases whose verification passed, which needs r40's verdicts. `plans_complete` counts every plan with a summary, a halted one too (#2830), and the sentinel `999.x` backlog phases are left out (#3167) |
| branch | str | C | the branch it is executed on, from `git.branching_strategy` and its template (`{milestone}`, `{slug}`); under the `milestone` strategy, else absent |
| risk_level | int | C | the highest current rpn of the risks it carries: its phases' `risk_level`, and each risk whose `milestone` names it ([governance.md](governance.md#risk-on-phases-and-milestones)); absent with none (beyond GSD) |

Upstream GSD's milestone has none of the fields marked beyond GSD ([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)): they are optional, no GSD file renders them and no
ported prompt asks for them.

The milestone reads back the journeys that ship in it as `journeys` (`journey.milestones`, [product.md](product.md)):
writing `journeys` on a milestone is E079.

GSD's archive copies (`milestones/<v>-ROADMAP.md` …) have no counterpart: a shipped milestone's entities stay where
they are, and `specforge query` answers what an archive answered.

## phase

GSD: `### Phase N: Name` in `ROADMAP.md` and its directory. Name: `03`, `03.1` (inserted), `999.1` (backlog; `/gsd-phase --promote 999.1` renames it, and its children, to the next integer of the milestone, and `--remove 999.1` deletes it without renumbering).
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
| backlog | bool | C | a `999.x` name; only `--promote` and `--remove` of `phase` act on it |
| branch | str | C | the branch it is executed on, from `git.branching_strategy` and its template (`{phase}`, `{slug}`); `phase`: each non-backlog phase; `milestone`: its milestone's; `none`: absent |
| frontend | bool | C | the ui gate's `frontend` detection (`@gsd/ui`'s; core never answers it) |
| status | enum{not_started, planned, in_progress, executed, needs_review, complete, deferred} | C | the GSD phase-status ladder (`phase-status.cts:160-196`): no plan is `not_started`, no summary `planned`, fewer summaries than plans `in_progress`, `needs_review` when the verification is `human_needed`, `complete` when it passed (`phase-status.cts:174-197`; UAT does not feed it), else `executed`. `passed` needs r40's verdicts; `deferred` is never derived. The phase is complete (ADR 0350 owner state) once `executed`, `needs_review` or `complete`: its scoped kinds' planned paths are checked from then on |
| plans_complete | str `N/M` | C | Progress `Plans Complete`: plans with a summary over plans |
| completed | date | C | Progress `Completed` |
| risk_level | int | C | the highest current rpn of the risks a plan of the phase mitigates, through a task or a truth ([governance.md](governance.md#risk-on-phases-and-milestones)); absent with none (beyond GSD) |

## workstream

GSD: `.planning/workstreams/<name>/`. A scope (K2): its milestones, phases and plans are unique within it.

| Field | Type | Flags | GSD |
|---|---|---|---|
| purpose | str | R | the workstream's description |
| status | enum{active, complete} | C | all its milestones shipped |

Which workstream is active is session state, not a field ([execute.md](execute.md#session-state)): `workstreams switch` and `resume` write `active_workstream` to the extension's project state. `status` and `list` name the active workstream (the command reads the state); a workstream is `complete` when every milestone has a `completion`; its progress counts the plans that have a summary.

## retrospective and lesson

**retrospective**: one `## Milestone:` section of `RETROSPECTIVE.md`. Name `v1.0-RETROSPECTIVE`.
`milestone → milestone R I`, `built strs`, `worked strs`, `inefficient strs`, `patterns strs`,
`cost {model_mix str, sessions int, notable str}`; `phases`, `plans`, `sessions` are `C`. Its lessons are read from
`lesson.retrospective` (the relation's inverse is named `lessons`).

**lesson**: name `L1`, scoped by its retrospective. `retrospective → retrospective` (absent for what
`/gsd-extract-learnings` writes, which is in the project scope), `text str R`, `milestone → milestone`, `top bool`.
Also holds what `/gsd-extract-learnings` writes (`LEARNINGS.md`), with `kind enum{decision, lesson, pattern, surprise}`:
those lessons are named `L-<phase>-<n>` (the phase is in the id, not a field), live in `spec/lessons.spec`, and have no counts (they are the lessons of each kind).
`graduated {disposition enum{promoted, deferred, dismissed} R, target → decision, until → phase, at ts R} H`: what
`/gsd-graduation` chose for the lesson's cluster (upstream's `**Graduated:**` mark and `graduation_backlog` row),
written by `specforge gsd graduate-lessons '<json list>' --disposition <d>`: a promotion names its decision
(`--target`), a deferral its phase (`--until`), a dismissal neither; a promoted or dismissed lesson is not graduated
again. `status enum{open, deferred, promoted, dismissed} C`: open with no graduation, and open again once a deferral's
phase is complete. Known difference: the state is per lesson, not per cluster. Upstream keys its backlog by a sha256 of
a lexical cluster the next run recomputes; the lesson is what persists, so each lesson of the cluster records it.

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

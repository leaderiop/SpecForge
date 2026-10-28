# Execution, results and state

```
 plan ─1──0..1─ summary ─1──*─ deliverable
                   ├──*─ deviation
                   └── requirements_completed →* requirement
 task ─1──*─ commit                    (collected from git by the gsd-git collector)
 phase ─0..1─ pause ─0..1─ handoff     (committed: what a teammate must see)
 project ─*─ blocker
 .specforge/state/gsd/                 (session state: never committed)
```

## summary

GSD: `NN-MM-SUMMARY.md`, written by the executor when a plan ends. It lives in the plan's own file, so executors of
one wave never write the same file. Name: the GSD file stem (`03-02-SUMMARY`), scoped by the plan.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R I | `phase:` + `plan:` |
| status | enum{complete, halted} | R | `status:` |
| one_liner | str | R | the bold one-liner |
| subsystem | str | R | `subsystem:` |
| tags | strs | | `tags:` |
| requires | {phase → phase R, provides str R} | | `requires:` |
| provides, affects | strs | | `provides:` / `affects:` |
| tech_added, tech_patterns | strs | | `tech-stack:` |
| key_decisions | →* decision | | `key-decisions:` (decisions with `origin summary`) |
| patterns_established | strs | | `patterns-established:` |
| requirements_completed | →* requirement | R | `requirements-completed:` |
| threat_flags | strs | | `## Threat Flags` |
| next_readiness | str | | `## Next Phase Readiness` |
| halted_because | str | | required when `status halted` |
| started, completed | ts | H | `completed:`; written by the command that opens and closes the plan, with the host's clock (`now`, protocol 1.11.0, K14) |
| duration | str | C | `duration:`: `completed` − `started` |
| actuals | {tasks int, commits int} | C | `actuals:`: tasks from the plan, commits from the recorded commits. GSD's `tokens` is absent until a harness exists (it stays out of the core rounds, decision U4) |
| files_created, files_modified | strs | C | `key-files:`: from the plan's commits. Paths are text, not `files`: a commit may delete or rename a path, and the host's E016 would fire on it |

## deliverable

GSD: a `coverage:` item. Name `D1`, scoped by the summary.
`summary → summary R I`, `description str R`, `requirement → requirement`, `human_judgment bool R`,
`rationale str` (required when `human_judgment`), `checks {kind enum{unit, integration, e2e, automated_ui, manual_procedural, other} R, ref str R, status enum{pass, fail, unknown} C}`.
Each check's `status` is a **computed member** of an authored block: the derive pass lists it as `deliverable.checks.status`
(`dsl::computed`), and a status written inside `checks` is E082. It is computed from the test report (K12: a compute pass
that declares `reads_tests` receives it). A check's `ref` is GSD's own coverage reference, `path#test name`
(`test/tasks.test.js#adds a task`), or a test tagged with its entity (the test link, under commit below).

## deviation

GSD: `## Deviations from Plan` and `## Issues Encountered`. Append-only. Name `DV1`, scoped by the summary.
`summary → summary R I`, `kind enum{deviation, issue} R I`, `rule str I`, `text str R I`, `files strs I`: a deviation is a log line, so
every field is immutable and the kind is append-only (E084, E085).

## commit

Written by the git collector, not by an agent: every field is `H` (a command's or collector's plan sets it; the host
refuses any other writer), `hash` also `I`. The collector (`gsd-git`) runs the whole `git log` with `capture_stdout`
and is named explicitly (`specforge collect --runner gsd-git`); an incremental `{since}` is not needed at core's scale.
It reads GSD's own subject convention, `{type}({phase}-{plan}): {task-name}` (`feat(03-02): issue a token on sign-in`),
and matches the task by name; a `GSD-Task: 03-02/T1` trailer, when present, wins over the name match. Only commits that
name a task or a plan are recorded, each in its plan's file (so executors of one wave never write the same file), named by
the first 12 hex digits of its hash. `hash str R H I`, `task → task H`, `plan → plan H`, `type str H` (`feat`, `fix` …),
`subject str H`, `at ts H`, `files files H`, `insertions int H`, `deletions int H`. Schema drift reads the
subjects, and a milestone's `stats.loc` the line counts.
`/gsd-undo` reads the commits of a plan or a phase from here instead of a manifest file.

**Test link.** A test names its entity with a tag in its name, `[gsd:<id>]` (`[gsd:03-02/TR1] stays signed in`);
`@gsd/core`'s JUnit collector (`gsd-junit`) links it, and the host re-keys a short form to the full path. The host's own
fallback links a test only by `entity_id__slug` or a module named after an entity, which cannot spell a scoped id.

## pause, handoff and blocker

These are what `STATE.md` and `.continue-here.md` hold that cannot be derived and that a teammate pulling the repository
must see. They are committed, in `spec/gsd/ops/`.

**pause**: written by `/gsd-pause-work`, removed by `/gsd-resume-work`. Name `PS1`. `phase → phase`, `plan → plan`,
`at ts H`, `by str H`, `reason str R`, `handoff → handoff`.

**handoff**: GSD `.continue-here.md`. Name `HO1`. `phase → phase`, `task → task`,
`state enum{in_progress, blocked, almost_done} R`, `current_state text R`,
`completed_work {task → task, state enum{done, in_progress, not_started}}`, `remaining_work strs`,
`decisions_made →* decision`, `blockers →* blocker`, `context text`, `next_action str R`.

**blocker**: GSD `### Blockers/Concerns`. Name `BL1`, project scope. `text str R`, `phase → phase`, `plan → plan`, `raised ts H`,
`resolved {at ts, how str}`.

## Session state

Not committed, not `.spec`: the host keeps it under `.specforge/state/gsd/` (git-ignored), as it keeps its compiled baselines.
It is what one person's session is doing.

| GSD | Here |
|---|---|
| `STATE.md` `current_plan`, `stopped_at`, `last_activity` | the session cursor |
| the active workstream pointer (already session-scoped in GSD) | the session's active scope |
| `/gsd-autonomous` and harness checkpoints | the harness's run log |
| `STATE.md.lock` | not needed: each plan's results go to its own file and edit plans carry a revision (K5) |
| "a workflow is running" (what `gsd-workflow-guard` reads) | a marker a `user_prompt` hook sets when the prompt is a `/gsd-…` command (`HookInput.prompt`) and the `stop` hook clears; a prompt contribution answers messages only and cannot set it |

`/gsd-progress` and `/gsd-next` answer from the graph and the cursor together.

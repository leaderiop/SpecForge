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
| duration | str | C | `duration:`: `completed` − `started`, as `45 min` under an hour and `1h 5m` from there; absent until both are recorded |
| actuals | {tasks int, commits int} | C | `actuals:`: tasks from the plan, commits from the recorded commits (0 with none). GSD's `tokens` is absent until a harness exists (it stays out of the core rounds, decision U4) |
| files_created, files_modified | strs | C | `key-files:`: from the plan's commits: `created` is the sorted union of their `created`, `modified` the other paths they touch; both absent with no commit. Paths are text, not `files`: a commit may delete or rename a path, and the host's E016 would fire on it |

## deliverable

GSD: a `coverage:` item. Name `D1`, scoped by the summary.
`summary → summary R I`, `description str R`, `requirement → requirement`, `human_judgment bool R`,
`rationale str` (required when `human_judgment`), `checks {kind enum{unit, integration, e2e, automated_ui, manual_procedural, other} R, ref str R, status enum{pass, fail, unknown} C}`.
Each check's `status` is a **computed member** of an authored block: the derive pass lists it as `deliverable.checks.status`
(`dsl::computed`), and a status written inside `checks` is E082. It is computed from the test report (K12: a compute pass
that declares `reads_tests` receives it): `fail` when a test named exactly its `ref` failed, `pass` when one passed, `unknown`
when none was recorded, and no status without a report. Because the derive pass reads the report, a compile without
`specforge-report.json` says I205 once. A check's `ref` is GSD's own coverage reference, `path#test name`
(`test/tasks.test.js#adds a task`), or a test tagged with its entity (the test link, under commit below).

## deviation

GSD: `## Deviations from Plan` and `## Issues Encountered`. Append-only. Name `DV1`, scoped by the summary.
`summary → summary R I`, `kind enum{deviation, issue} R I`, `rule str I`, `text str R I`, `files strs I`: a deviation is a log line, so
every field is immutable and the kind is append-only (E084, E085).

## commit

Written by the git collector, not by an agent: every field is `H` (a command's or collector's plan sets it; the host
refuses any other writer). The collector (`gsd-git`) runs the whole `git log` with `capture_stdout` and is named
explicitly (`specforge collect --runner gsd-git`); an incremental `{since}` is not needed at core's scale. It reads GSD's
own subject convention, `{type}({phase}-{plan}): {task-name}` (`feat(03-02): issue a token on sign-in`), and matches the
task by its title. **There is no `GSD-Task:` trailer**: GSD has none, so a commit names its task by the subject alone,
and a commit whose description is no task's title (the metadata commit `docs(03-02): complete … plan`) names its plan
only. A commit that names no plan is not recorded. Each is recorded in its plan's file (so executors of one wave never
write the same file), in the project scope, named by the first 12 hex digits of its hash.

| Field | Type | Flags | |
|---|---|---|---|
| hash | str | H I | the full 40-hex hash. Not `R`: a required harness field cannot be declared, and the name already carries it |
| task | → task | H | the inverse `task.commits` |
| plan | → plan | H | the inverse `plan.commits` |
| type | str | H | `feat`, `fix` … |
| subject | str | H | |
| at | ts | H | the author time, UTC |
| files | strs | H | repository-relative paths, as git prints them; text, not `files`, since a later commit may delete one |
| created | strs | H | the paths `git log --summary` reports `create mode` for (GSD's `key-files.created`) |
| insertions, deletions | int | H | `git log --numstat` |

**Collecting.** `specforge collect --runner gsd-git --yes` (or without `--yes`, to be asked) runs
`git -c core.quotepath=off log --no-merges --no-renames --numstat --summary --format=%x1e%H%x1f%at%x1f%s` once in the
project root and hands the captured output to the collector, which is a *writing* one: it is also given the graph, and
answers an edit plan (`Record N commits`) that creates the new `commit` entities in their plans' files, oldest first.
The host applies the plan and records each field in `specforge-harness`. A commit whose 12-hex name the graph already
holds is not recorded again, so a second run changes nothing. The collector is named, never detected, so a plain
`specforge collect` does not run it. A repository with no commit (or a directory that is none) prints nothing, and
`collect` says so with E045 ("produced no report"). The header is read as `/gsd-undo` reads it: one `fixup! `, `squash! `
or `Revert "…"` layer is unwrapped, then `type[(scope)][!]: description`, case-insensitively; the scope is the plan's GSD
id (`03-02`, `03.1-01`, a quick id), resolved over the graph as a path, a rooted path or a unique short form. A scope that
names no plan, several plans (`01-01` in two milestones: write the full path) or a non-plan is skipped. The task is the one
of that plan whose title equals the description, trimmed, case-folded and with whitespace runs collapsed; none, or two,
leaves the commit naming its plan. A plan op refused `stale_revision` (the plan's file changed since the compile) writes
nothing, and the next run records the same commits.

Schema drift reads the subjects, and a milestone's `stats.loc` the line counts. `/gsd-undo` reads the commits of a plan or a
phase from here instead of a manifest file.

**Test link.** A test links to an entity two ways, and the JUnit collector (`gsd-junit`) does both.
- A **tag** in the test's name, `[gsd:<id>]` (`[gsd:03-02/TR1] stays signed in`), links it to any entity. The id is a path,
  a rooted path or a unique suffix; the collector resolves it over the graph as the host's `find` does. A tag that names
  nothing, or several entities, links nothing.
- A **coverage reference** links a deliverable's check, GSD's own test link (`gsd-core/templates/summary.md:59`): the check's
  `ref` is `test/tasks.test.js#a task is added`, and a test named `a task is added` whose `file` attribute (made
  project-relative by stripping the project root) is `test/tasks.test.js` proves it. A `ref` with an empty file part
  (`#name`) matches on the name alone. The result is recorded under the deliverable and named by the `ref` exactly, which
  is what the derive pass looks a check's status up by.

A test that links neither way is reported unlinked, for the host's naming convention (`entity_id__slug`, a module named
after an entity), which cannot spell a scoped id. The collector has no command (GSD is language-agnostic) and reads the
report the project's own test run wrote: `specforge collect --runner gsd-junit --no-run` (the declared report is
`junit.xml` at the project root), or `--report <file>`. It declares `writes` only to be handed the graph and the project
root: the host's `collect` keeps a full entity path and drops any other id as W115, so a collector that is given a short
form resolves it itself. It answers results, never a plan. A report that stops short of its last tag is an error naming
the file, not a smaller report.

## pause, handoff and blocker

These are what `STATE.md` and `.continue-here.md` hold that cannot be derived and that a teammate pulling the repository
must see. They are committed, in `spec/gsd/ops/`.

**pause**: written by `/gsd-pause-work`, removed by `/gsd-resume-work`. Name `PS1`, scoped by its phase (`phase → phase I`,
optional: a pause with no phase is in the project scope). `plan → plan`, `at ts H`, `by str H`, `reason str R`,
`handoff → handoff`.

**handoff**: GSD `.continue-here.md`. Name `HO1`, scoped by its phase as a pause is. `phase → phase I`, `task → task`,
`state enum{in_progress, blocked, almost_done} R`, `current_state str R`,
`completed_work {task → task, state enum{done, in_progress, not_started}}`, `remaining_work strs`,
`decisions_made →* decision`, `blockers →* blocker`, `context str`, `next_action str R`.

**blocker**: GSD `### Blockers/Concerns`. Name `BL1`, project scope. `text str R`, `phase → phase`, `plan → plan`, `raised ts H`,
`resolved {at ts, how str}`. `resolved` is authored, so a person writes `resolved.at` and `gsd/E107` refuses a time that
is not RFC 3339 UTC (`2026-10-29T10:00:00Z`).

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

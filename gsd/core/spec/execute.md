# Execution, results and state

**Status: built (r38): the seven kinds below, `gsd/E401` to `gsd/E405` (E403 is `gsd/W403`), `gsd/W407` and `gsd/W408`, the computed values of execution (plan, task and check status, duration, actuals, key files) and the collectors `gsd-git` and `gsd-junit`. The session state is not modelled.**

```
 plan ─1──0..1─ summary ─1──*─ deliverable
                   ├──*─ deviation
                   └── requirements_completed →* requirement
 task ─1──*─ commit                    (collected from git by the gsd-git collector)
 phase ─0..1─ pause ─0..1─ handoff     (committed: what a teammate must see)
 project ─*─ blocker
 .specforge/state/gsd__core/                 (session state: never committed)
```

## plan_start

Upstream: `PLAN_START_TIME`, a shell variable. Named `START`, scoped by its plan, in the plan's own file. Its own entity:
a plan's plan-check verdicts read the plan and its siblings (`self`, `siblings`), so a field on the plan would make them
stale.

| Field | Type | Flags | GSD |
|---|---|---|---|
| plan | → plan | R, I | the plan it opened |
| at | ts | H | written once by `start_plan`, with the host's clock; a second call answers it, and `complete_plan` copies it to the summary's `started` |
| head | string | H | the commit the plan opened at: written once by `start_plan --head`, whichever call first brings it; none on an unborn branch. The executor's `commits:` counts from it. Upstream's `PLAN_HEAD_BEFORE` file |

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
| started, completed | ts | H | `completed:`; both written by `complete_plan` (`started` from the plan's `plan_start`, `completed` the host's clock, `now`, protocol 1.11.0, K14): no summary exists when a plan starts, and one made without its required fields is refused |
| duration | str | C | `duration:`: `completed` − `started`, as `45 min` under an hour and `1h 5m` from there; absent until both are recorded |
| actuals | {tasks int, commits int} | C | `actuals:`: tasks from the plan, commits from the recorded commits (0 with none). GSD's `tokens` is absent until a harness exists (it stays out of the core rounds, decision U4) |
| files_created, files_modified | strs | C | `key-files:`: from the plan's commits: `created` is the sorted union of their `created`, `modified` the other paths they touch; both absent with no commit. Paths are text, not `files`: a commit may delete or rename a path, and the host's E016 would fire on it |

## deliverable

GSD: a `coverage:` item. Name `D1`, scoped by the summary.
`summary → summary R I`, `description str R`, `requirement → requirement`, `human_judgment bool R`,
`rationale str` (required when `human_judgment`), `checks {kind enum{unit, integration, e2e, automated_ui, manual_procedural, other} R, ref str R, status enum{pass, fail, unknown} C}`.
Each check's `status` is a **computed member** of an authored block: the derive pass lists it as `deliverable.checks.status`
(`dsl::computed`), and a status written inside `checks` is E082. It is computed from the test report (K12: a compute pass
that declares the `tests` input receives it): `fail` when a test named exactly its `ref` failed, `pass` when one passed, `unknown`
when none was recorded, and no status without a report. Because the derive pass reads the report, a compile without
`specforge-report.json` says I205 once. A check's `ref` is GSD's own coverage reference, `path#test name`
(`test/tasks.test.js#adds a task`), or a test tagged with its entity (the test link, under commit below).

## deviation

GSD: `## Deviations from Plan` and `## Issues Encountered`. Append-only. Name `DV1`, scoped by the summary.
`summary → summary R I`, `kind enum{deviation, issue} R I`, `rule str I`, `text str R I`, `files paths<history> I` (never file-checked: a later commit may delete one): a deviation is a log line, so
every field is immutable and the kind is append-only (E084, E085).

## commit

Written by the git collector, not by an agent: every field is `H` (a command's or collector's plan sets it; the host
refuses any other writer). The collector (`gsd-git`) reads the whole log through the host's git port (ADR 0352) and is
named explicitly (`specforge collect --runner gsd-git`) or detected; an incremental `{since}` is not needed at core's
scale. It reads GSD's
own subject convention, `{type}({phase}-{plan}): {task-name}` (`feat(03-02): issue a token on sign-in`), and matches the
task by its title. A `Gsd-Task: <plan>/<task>` trailer comes first (ADR 0331 D3): one that names a task fixes the plan
and the task, whatever the subject says (`linked_by Gsd-Task`); otherwise the subject decides (`linked_by title`). A
commit whose description is no task's title (the metadata commit `docs(03-02): complete … plan`) names its plan
only. A commit that names no plan is not recorded. Each is recorded in its plan's file (so executors of one wave never
write the same file), in the project scope, named by the first 12 hex digits of its hash.

| Field | Type | Flags | |
|---|---|---|---|
| hash | str | H I | the full 40-hex hash. Not `R`: a required harness field cannot be declared, and the name already carries it |
| task | → task | H | the inverse `task.commits` |
| plan | → plan | H | the inverse `plan.commits` |
| linked_by | enum{Gsd-Task, title} | H | how `task` was found: the trailer or the subject's title; written only with `task` (04-U1) |
| type | str | H | `feat`, `fix` …; absent for a subject with no `type:` header (a commit its trailer links) |
| subject | str | H | |
| at | ts | H | the author time, UTC |
| files | strs | H | the paths it touched under the project, project-relative (a path outside it is left out); a rename touches both paths; text, not `files`, since a later commit may delete one |
| created | strs | H | the paths under the project it added (GSD's `key-files.created`), project-relative |
| insertions, deletions | int | H | the lines it added and removed, of every file it touched |
| descends_from | →* commit | H | the recorded commits of the same plan this one reaches through its parents (`%P`, merges included), oldest first; `[]` for none, absent on a commit recorded before ancestry was collected. W403 reads it (#5275) |

**Collecting.** `specforge collect --runner gsd-git --yes` (or without `--yes`, to be asked) runs the collector, which
runs no program: it asks the host's git port for the typed log from `HEAD` (its trailers and author times), then one
diff of each commit it records against its first parent (a root commit's against the empty tree), renames apart, so a
rename touches both paths (ADR 0352). It is a *writing* collector: it is also given the graph, and answers an edit plan (`Record N commits`) that creates the new `commit` entities in their plans' files, oldest first.
The host applies the plan and records each field in `specforge-harness`. A commit whose 12-hex name the graph already
holds is not recorded again, so a second run changes nothing. The collector is detected by `.git` (a directory, or a
linked worktree's file, ADR 0352), or named (`--runner gsd-git`): a plain `specforge collect` in any repository runs it,
and an unborn one (no commit yet) records nothing. It needs neither a run nor a `git` on PATH, so `--no-run` records the
commits too; named, a directory in no repository fails ("the project is in no git repository"). The header is read as `/gsd-undo` reads it: one `fixup! `, `squash! `
or `Revert "…"` layer is unwrapped, then `type[(scope)][!]: description`, case-insensitively; the scope is the plan's GSD
id (`03-02`, `03.1-01`, a quick id), resolved over the graph as a path, a rooted path or a unique short form. A scope that
names no plan, several plans (`01-01` in two milestones: write the full path) or a non-plan is skipped, except that a
scope that starts with the project's `project_code` (the setting, else the `project` entity's field) and a dash
(`feat(PRJ-01-01)` with `project_code "PRJ"`) is resolved again without them (#5271): only the configured code, case-sensitively, so `AUTH-01-01` stays literal. The task is the one
of that plan whose title equals the description, trimmed, case-folded and with whitespace runs collapsed; none, or two,
leaves the commit naming its plan.

**The trailer.** git prints the `Gsd-Task:` trailer values unfolded on the header line, before the subject, so a value
never starts a line of its own and a unit separator in a subject stays in the subject. The values are trimmed, blank
ones dropped and repeats counted once. One value of the form `<plan>/<task>` (each side ASCII letters, digits, `.`, `_`
and `-`) is resolved as a task over the graph, as written or with the `project_code` prefix stripped from its plan.
Two different values, or one of another form, is unreadable (as upstream's `ship.md:335` counts a doubled
`gate-status:` as missing). An unreadable trailer, or one that names no task, falls back to the subject, and the plan's
title notes `(trailer of <12 hex> names no task: <value or "unreadable">)`. A revert names no task, even with a trailer.
GSD's executor writes the trailer: `gsd-executor.md`'s commit step ends the message with it as a second `-m`, its last
paragraph (ADR 0351).

A plan op refused `stale_revision` (the plan's file changed since the compile) writes
nothing, and the next run records the same commits. Merges are read for their parents and never recorded.

Schema drift reads the subjects, and a milestone's `stats.loc` the line counts. `/gsd-undo` reads the commits of a plan or a
phase from here instead of a manifest file.

**Test link.** A test links to an entity two ways, and the JUnit collector (`gsd-junit`) does both.
- A **tag** in the test's name, `[gsd:<id>]` (`[gsd:03-02/TR1] stays signed in`), links it to any entity. The id is a path,
  a rooted path or a unique suffix; the collector resolves it over the graph as the host's `find` does. A tag that names
  nothing, or several entities, links nothing.
- A **coverage reference** links a deliverable's check, GSD's own test link (`gsd-core/templates/summary.md:59`): the check's
  `ref` is `test/tasks.test.js#a task is added`, and a test named `a task is added` whose `file` attribute (made
  project-relative by stripping the project root) is `test/tasks.test.js` proves it. A `ref` with an empty file part
  (`#name`) matches on the name alone. The file part ends at the first `#`, so a test name may hold one (`fixes #42`). The result is recorded under the deliverable and named by the `ref` exactly, which
  is what the derive pass looks a check's status up by.

  Limits of the link, by design: a reporter that writes no `file` attribute (jest-junit and pytest by default) matches
  only a `ref` with an empty file part; the root is stripped as the collector is given it, so a report written under a
  symlinked root (`/tmp` and `/private/tmp`) does not link by file; a `time` that is negative, not a number or spelled
  with a thousands separator gives no duration. A report cut inside a `<testcase>`, or that holds no `<testsuite>`, is
  refused; a tag that names no entity, or several, lists the test as unlinked even when another tag linked it.

`gsd-git` reads the whole log on every run (no `--since`): a repository of ten thousand commits costs seconds, and the
port's answer cap and the host's 512 MB / 30 s guest limit are the bound (a log past one answer fails the collect).
Control characters in a subject or path become U+FFFD, and a commit whose time is out of range is skipped and named in
the plan's title. The host decides the repository; the guest names commits by id only.

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

Not committed, not `.spec`: the host keeps it under `.specforge/state/gsd__core/` (git-ignored), as it keeps its compiled baselines.
It is what one person's session is doing.

| GSD | Here |
|---|---|
| `STATE.md` `current_plan`, `stopped_at`, `last_activity` | the session cursor |
| the active workstream pointer (already session-scoped in GSD) | the session's active scope |
| `/gsd-autonomous` and harness checkpoints | the harness's run log |
| `STATE.md.lock` | not needed: each plan's results go to its own file and edit plans carry a revision (K5) |
| "a workflow is running" (what `gsd-workflow-guard` reads) | a marker a `user_prompt` hook sets when the prompt is a `/gsd-…` command (`HookInput.prompt`) and the `stop` hook clears; a prompt contribution answers messages only and cannot set it |

`/gsd-progress` and `/gsd-next` answer from the graph and the cursor together.

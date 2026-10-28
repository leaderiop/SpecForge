# Execution, results and state

```
 plan ─1──0..1─ summary ─1──*─ deliverable
                   ├──*─ deviation
                   └── requirements_completed →* requirement
 task ─1──*─ commit                    (collected from git, never authored)
 phase ─0..1─ pause ─0..1─ handoff     (committed: what a teammate must see)
 project ─*─ blocker
 .specforge/state/gsd/                 (session state: never committed)
```

## summary

GSD: `NN-MM-SUMMARY.md`, written by the executor when a plan ends. It lives in the plan's own file, so executors of
one wave never write the same file. Name: the plan's name with `S` (`03-02-S`), scoped by the plan.

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
| next_readiness | text | | `## Next Phase Readiness` |
| halted_because | str | | required when `status halted` |
| started, completed | ts | H | `completed:`; written by the command that opens and closes the plan |
| duration | str | C | `duration:`: `completed` − `started` |
| actuals | {tokens int, tasks int, commits int} | C | `actuals:`: tasks from the plan, commits from the collector, tokens from the harness |
| files_created, files_modified | files | C | `key-files:`: from the plan's commits |

## deliverable

GSD: a `coverage:` item. Name `D1`, scoped by the summary.
`summary → summary R I`, `description str R`, `requirement → requirement`, `human_judgment bool R`,
`rationale str` (required when `human_judgment`), `checks {kind enum{unit, integration, e2e, automated_ui, manual_procedural, other} R, ref str R}`;
each check's `status enum{pass, fail, unknown}` is `C`, from the test collectors' reports.

## deviation

GSD: `## Deviations from Plan` and `## Issues Encountered`. Append-only.
`summary → summary R`, `kind enum{deviation, issue} R`, `rule str`, `text str R`, `files files`.

## commit

Never authored: the git collector writes it from `git log` (K5's host write, not an agent's). It reads GSD's own
subject convention, `{type}({phase}-{plan}): {task-name}` (`feat(03-02): issue a token on sign-in`), and matches the
task by name; a `GSD-Task: 03-02/T1` trailer, when present, wins over the name match.
`hash str R I`, `task → task`, `plan → plan`, `type str` (`feat`, `fix` …), `at ts`, `files files`.
`/gsd-undo` reads the commits of a plan or a phase from here instead of a manifest file.

## pause, handoff and blocker

These are what `STATE.md` and `.continue-here.md` hold that cannot be derived and that a teammate pulling the repository
must see. They are committed, in `spec/gsd/ops/`.

**pause**: written by `/gsd-pause-work`, removed by `/gsd-resume-work`. `phase → phase`, `plan → plan`,
`at ts H`, `by str H`, `reason str R`, `handoff → handoff`.

**handoff**: GSD `.continue-here.md`. `phase → phase`, `task → task`,
`state enum{in_progress, blocked, almost_done} R`, `current_state text R`,
`completed_work {task → task, state enum{done, in_progress, not_started}}`, `remaining_work strs`,
`decisions_made →* decision`, `blockers →* blocker`, `context text`, `next_action str R`.

**blocker**: GSD `### Blockers/Concerns`. `text str R`, `phase → phase`, `plan → plan`, `raised ts H`,
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

`/gsd-progress` and `/gsd-next` answer from the graph and the cursor together.

# GSD A/B protocol (proof lane, draft)

**Status:** draft, `gsd-ab-v1-draft`. It gates nothing in R1. It is reviewed at gate A (end of R2), then frozen and
tagged `gsd-ab-v1` before any run. A change after the tag is a new version, and the report says so.

This is the pre-registered protocol of the A/B the build plan's gate B reads: does an agent that keeps its project
state in a checked graph do better than one that keeps it in Markdown? The files beside this one are the corpus
(`tasks.json`) and the scorer (`score.py`). Run files and transcripts are never committed.

## 1. Question and hypotheses

- **H1 (gate B, decision D7).** Checked state (arm B) beats Markdown state (arm A) on false "verified" (M3) and on
  state drift (M2), without costing more than 10% in tokens per completed plan (M4).
- **H2 (secondary, R1's question).** The requirement thread helps an agent: arm B beats arm B0 on false "verified"
  for the defects the thread exists to catch (S1, S4, S5), counted alone (M3 over those three, `M3_thread`). H2 is
  reported and gates nothing. B0 keeps `gsd/E521` (it reads commits, not test facts), so S4 is reported in both arms
  and H2's difference can come from S1 and S5 only.

## 2. Arms

Same model, same temperature, same tool budget, same task prompts (each arm's prompt names its own commands, nothing
else differs). Each arm's configuration is frozen in `tasks.json#arms` (`config`: the extensions by name, whether the
fixture's thread packs load, the settings; `drop`: the fixture's paths the arm does not see), and `seed.py` writes it
into every copy.

No configuration turns the route off: `@gsd/core` declares it whatever is loaded. What B0 lacks is what the route's
`item` and `test` steps read, and what `code/E101` and `gsd/E501`'s `tests` input read. So in B0 a verifier verdict
recorded over tests reads stale from the start (its `tests` input reads none), on the seeded and the unseeded copy
alike: it is no signal of a seed.

| Arm | State | Tools |
|---|---|---|
| **A** | gsd-core 1.19.0: `.planning/` Markdown | `gsd-tools` |
| **B** | SpecForge `@gsd/core` with the gsd-strict profile and `specforge.task_trailers` on (E521: a task commit links its task by a `Gsd-Task:` trailer): the `.spec` graph | `specforge` CLI and MCP, `collect`, `trace <requirement>` |
| **B0** | B without the test facts: no `@code/core` and no pack that writes its facts (the fixture's `thread` in `tasks.json`), so no test or item facts and no `proof` links. The requirement route is `@gsd/core`'s and stays: `trace` follows it with its `item` and `test` steps skipped (`I210`) | as B |

## 3. Fixtures

- **taskboard**: `gsd/parity/fixtures/taskboard` (the twin) and its `.planning/` (arm A).
- **vault**: `gsd/parity/fixtures/vault`, the security twin, and its `.planning/`. Its four tasks are over phase 01,
  its threat-model phase.
- **ledger**: `gsd/parity/fixtures/ledger`, a Rust project whose CI is red, and its `.planning/`. Phase 01 was
  verified, then a quick task folded the total in an `i32`: `total_beyond_i32_is_exact` fails and `.planning/` still
  says passed. Its thread loads the Rust packs (`@rust/cargo`, `@rust/source`, `@rust/test`). Every ledger task's
  ground truth holds that truth (`v1.0/01/01-02/TR2`) not verified, besides its seeds.

Each arm starts from the same commit of the fixture, in its own format. The importer may not be used to make arm A's
input: arm A's `.planning/` is the recorded one, not a conversion of the twin.

## 4. Tasks

`tasks.json#tasks`: 4 per fixture, 12 in all. Each has an `id`, a `fixture`, a `workflow` (execute, verify,
progress/resume, ship), a `prompt`, the defects `seeded` in its copy, the `acceptance` checks run after the agent
stops, and the `ground_truth` (the requirement, plan and verdict states after a correct run).

## 5. Seeded defects

`tasks.json#defects`, each with how to seed it in arm A and in arm B and what B's checker says. A defect is seeded in a
copy of the fixture, never in the committed one.

| Id | Defect | B's signal |
|---|---|---|
| S1 | a test that never ran: a summary or verdict claims a test that is not in the run | `code/E101` on the proof |
| S2 | a requirement id that does not exist (`requirements [TASK-09]`) | `E003` |
| S3 | a status written ahead of the work: phase 02 marked complete before its plan ran | `E082` |
| S4 | a commit trailer naming a task that does not exist: `Gsd-Task: 01-01/T9` under GSD's subject `feat(01-01): <T1's title>` | `gsd/E521` on the commit (its task found by the subject's title, not by the trailer) |
| S5 | a truth verified over a failing proof | `gsd/E501` stale, shown in `trace` |
| S6 | a proof naming a deleted test | `code/E101` |

## 6. Measures

Per arm, per (task, seed) run. The operator's scripts write a run file (section 7); `score.py` computes:

| Id | Measure | From the run file |
|---|---|---|
| M1 | first-pass acceptance | every `acceptance` check passed on the first delivery |
| M2 | state-drift incidents | claims the agent leaves (status, progress, "complete", "verified") that differ from the task's `ground_truth`, recomputed from the repository after the run |
| M3 | false "verified" | seeded defects still present at the end of a run whose final report says verified, complete or passed |
| M4 | tokens per completed plan | total tokens over plans completed, per arm |
| M5 | wall time and human interventions | `wall_seconds`, `interventions` |
| M6 | rework commits after verify | `rework_commits` |

## 7. Run procedure

- The model, temperature and tools are fixed before the first run and recorded in every run file.
- 3 seeds per (task, arm); arms are interleaved by task, so drift in the model's service hits every arm alike.
- The operator follows a script. An intervention is counted, never improvised.
- Every transcript is kept under `runs/<arm>/<task>/<seed>.json` (git-ignored), archived with the report.

A run file:

```json
{
  "arm": "B", "task": "taskboard-execute-02-01", "seed": 1, "model": "<id>",
  "acceptance": { "npm test": true, "specforge check": true },
  "claims": { "v1.0/02/02-01": "complete" },
  "final_report": "verified",
  "defects_present": ["S4"],
  "tokens": 182000, "plans_completed": 1,
  "wall_seconds": 840, "interventions": 0, "rework_commits": 0
}
```

## 8. Analysis

- Runs are paired by (task, seed): an A run and a B run (and a B0 run, for H2) of the same task and seed. A run with
  no partner is refused, never dropped.
- Per measure, the paired difference B − A, with a 95% percentile bootstrap interval: 10,000 resamples over tasks, and
  over seeds within each resampled task, with a fixed RNG seed so a report reproduces.
- H2 is reported the same way: B − B0 on `M3_thread` (false "verified" counting only S1, S4 and S5), with its
  interval and whether its upper bound is below 0. It gates nothing.

## 9. Gate B (D7)

**Pass** when the upper bound of B − A is below 0 for M3 and for M2 (B has fewer incidents), and M4(B) ≤ 1.10 × M4(A).
**Fail** otherwise: R9 is dropped, and the result is published as it is.

## 10. Pre-registration

This README, `tasks.json` and `score.py` are tagged `gsd-ab-v1` at gate A, before any run. A change after the tag is
a new version, and the report names it.

## 11. Threats

| Threat | Mitigation |
|---|---|
| One model | name it; a second model's run is a new version, not a retry |
| Fixtures written by the authors | the third fixture comes from outside (failed CI or `@gsd/import`); seeds are applied by script |
| Arm B's prompts are newer than arm A's | both arms' prompts are frozen at gate A from their releases (gsd-core 1.19.0, `@gsd/core` at the tag) |
| The operator's hand in interventions | a written script for when to step in; every intervention logged with its reason and counted in M5 |

## Score

```
python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs
python3 -I evaluation/gsd-ab/score.py --self-test
```

`--self-test` scores an embedded synthetic set where B clearly wins and one where it doesn't, checks the D7 verdict of
each, and checks that an unpaired run is refused.

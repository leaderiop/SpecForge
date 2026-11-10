# GSD A/B protocol (proof lane)

**Status:** v1, ready for review (`tasks.json` version `gsd-ab-v1`). It is reviewed at gate A ([REVIEW.md](REVIEW.md)),
then tagged `gsd-ab-v1` by the owner before any run. A change after the tag is a new version, and the report says so.

This is the pre-registered protocol of the A/B the build plan's gate B reads: does an agent that keeps its project
state in a checked graph do better than one that keeps it in Markdown? The files beside this one are the corpus
(`tasks.json`), the seeding script (`seed.py`), the scorer (`score.py`), the arms' prompts (`prompts/`), the
operator's script (`OPERATOR.md`) and the review record (`REVIEW.md`). Run files and transcripts are never
committed.

## 1. Question and hypotheses

- **H1 (gate B, decision D7).** Checked state (arm B) beats Markdown state (arm A) on false "verified" (M3) and on
  state drift (M2), without costing more than 10% in tokens per completed plan (M4).
- **H2 (secondary, R1's question).** The requirement thread helps an agent: arm B beats arm B0 on false "verified"
  for the defects the thread exists to catch (S1, S4, S5), counted alone (M3 over those three, `M3_thread`). H2 is
  reported and gates nothing. B0 keeps `gsd/E521` (it reads commits, not test facts), so S4 is reported in both arms
  and H2's difference can come from S1 and S5 only.

## 2. Arms

Same model, same temperature, same tool budget, same task prompts. The prompt is `prompts/<arm>.md`, the task's
`prompt` put in place of `{task.prompt}`: the three files are one text except their commands block (between
`<!-- commands -->` and `<!-- /commands -->`), which names the arm's tools (B0's is B's), and all end asking for
one `RESULT:` line, whose value is the run's `final_report`. Each arm's configuration is frozen in `tasks.json#arms`
(`config`: the extensions by name, whether the fixture's thread packs load, the settings; `drop`: the fixture's paths
the arm does not see), and `seed.py` writes it into every copy.

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
  ground truth holds that truth (`v1.0/01/01-02/TR2`) not verified, besides its seeds. Why this fixture: a "verified"
  is false there before any seed, on a path GSD really takes (a quick task does not re-verify the phase it changes);
  it is a second language, so the thread's packs are not only Node's; and it is small enough to read whole.

Each arm starts from the same commit of the fixture, in its own format. The importer may not be used to make arm A's
input: arm A's `.planning/` is the recorded one, not a conversion of the twin.

## 4. Tasks

`tasks.json#tasks`: 4 per fixture, 12 in all. Each has an `id`, a `fixture`, a `workflow` (execute, verify,
progress/resume, ship), a `prompt`, the defects `seeded` in its copy, the `acceptance` checks run after the agent
stops, and the `ground_truth` (the requirement, plan and verdict states after a correct run). An acceptance row has a
`text` and either `run`, a command run in the copy (passes when it exits 0), or `judge`, the one-line rubric the
operator reads the copy against.

## 5. Seeded defects

`tasks.json#defects`, each with how to seed it in arm A and in arm B and what B's checker says. A defect is seeded in a
copy of the fixture, never in the committed one.

| Id | Defect | B's signal | When |
|---|---|---|---|
| S1 | a test that never ran: a summary or verdict claims a test that is not in the run | `code/E101` on the proof | `complete`: once the cited truth's plan is complete |
| S2 | a requirement id that does not exist (`requirements [TASK-09]`) | `E003` | `seed` |
| S3 | a status written ahead of the work: phase 02 marked complete before its plan ran | `E082` | `seed` |
| S4 | a commit trailer naming a task that does not exist: `Gsd-Task: 01-01/T9` under GSD's subject `feat(01-01): <T1's title>` | `gsd/E521` on the commit (its task found by the subject's title, not by the trailer) | `seed`, after `collect` |
| S5 | a truth verified over a failing proof | `gsd/E501` stale, shown in `trace` | `seed`, after `collect` |
| S6 | a proof naming a deleted test | `code/E101` | `seed` |

`tasks.json`'s `b_signals` is the data, per task: the code, the entity, the count and when, and `ab_seeds` is the
proof: on a copy `seed.py` made, B reports each row's count where the unseeded copy of the same task reports none,
and B0 reports none of the thread's (S1, S5, S6). `seed` means after seeding and a collect; `complete` means only
after the task's `completion`, the edits a correct run leaves. S1's holder is a truth of the plan the run executes,
and `@gsd/core` checks what a pending plan cites only once the plan completes: a run that ends with that plan still
pending has no B signal for S1, and that is correct, since it did not claim the plan complete.

A verdict goes stale when anything it read changes, the cited test file included: S5's and S6's edits to a test file
stale the verdicts of every truth that file's tests prove. The signal is counted on the seeded entity, so a row still
checks the truth it names; the other stale verdicts are the same on every arm's copy of the task.

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
- Each run starts a fresh agent session in the arm's copy and sends one message: `prompts/<arm>.md` with the task's
  `prompt` in place of `{task.prompt}`. Nothing else is said unless the script calls for an intervention.
- The operator follows a script, [OPERATOR.md](OPERATOR.md): the copies, each arm's tools, the prompt, when a run
  ends, how each field of the run file is read. An intervention is one of its four, with its text, logged with its
  turn and counted, never improvised.
- Every run file is `runs/<arm>/<task>/<seed>.json` and its transcript `runs/transcripts/<arm>/<task>/<seed>.jsonl`
  (both git-ignored), archived with the report. `score.py` reads only `<arm>/<task>/<number>.json`, so a transcript
  is never read as a run.

A run file:

```json
{
  "arm": "B", "task": "taskboard-execute-02-01", "seed": 1, "model": "<id>", "temperature": 0,
  "host": { "agent": "<program>", "version": "<version>", "machine": "<os, cpu, memory>" },
  "budget": { "max_turns": 200, "max_tokens": 400000 }, "permission_mode": "auto-approve inside the copy",
  "acceptance": { "npm test passes": true, "tasks.json is written after every change": true },
  "claims": { "v1.0/02/02-01": "complete" },
  "final_report": "verified",
  "defects_present": ["S4"],
  "tokens": 182000, "plans_completed": 1,
  "wall_seconds": 840, "rework_commits": 0,
  "interventions": 1, "intervention_log": [{ "reason": "question", "turn": 7 }]
}
```

Every field is required. `final_report` is the value of the run's last `RESULT:` line, or `none`; an intervention's
`reason` is one of OPERATOR.md's (`question`, `stopped`, `budget`, `operator`) and `interventions` is the log's
length. `acceptance` holds one boolean per acceptance row of the task, keyed by its text; `claims` one entry per key
of the task's `ground_truth`; `defects_present` only ids the task seeds. `score.py` refuses a run file that breaks
any of this, or whose path is not its arm, task and seed.

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
| Fixtures written by the authors: all three are | every fixture, seed, completion, acceptance row and prompt is in the tagged commit; each `.planning/` is hand-written from gsd-core's templates and read by gsd-tools 1.19.0 (the `record*.sh` scripts fail on a verb that errors); one fixture (ledger) is red from the start, so a "verified" is false there with no seed. A project from outside, replayed through `@gsd/import`, cannot be in v1: the importer is built after the tag, and arm B's input would come from code the tag does not hold (section 10). It is the threat a `gsd-ab-v2` closes |
| Arm B's prompts are newer than arm A's | both arms' prompts are frozen at gate A (`prompts/`), and each arm's agents and skills come from its release unchanged (gsd-core 1.19.0, `@gsd/core` at the tag) |
| The operator's hand in interventions | [OPERATOR.md](OPERATOR.md): four reasons, each with its text, logged with its turn and counted in M5; `score.py` refuses any other |

## Score

```
python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs
python3 -I evaluation/gsd-ab/score.py --self-test
```

`--self-test` writes an embedded synthetic set (4 tasks, 3 seeds, 3 arms) as run files and reads it back, scores it
where B clearly wins and where it doesn't, checks the gate B verdict of each and that an unpaired run is refused, and
refuses exactly eight bad run files: one without its host, one whose interventions are not logged, one whose
intervention has a reason outside the list, one whose acceptance value is not a boolean, one with an acceptance row
its task does not have, one with a claim on a key outside the task's ground truth, and two whose `defects_present`
holds what the task does not seed (an unknown id, a number).

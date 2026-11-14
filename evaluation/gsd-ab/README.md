# GSD A/B protocol (proof lane)

**Status:** v2, a draft (`tasks.json` version `gsd-ab-v2`). v1 is frozen by the tag `gsd-ab-v1`, which the owner sets
at v1's gate A on a commit before this version. v2 adds arm Bv1, arm B as that tag froze it, and the measure M7 of the
context an executor starts on. It is reviewed at its own gate A ([REVIEW.md](REVIEW.md)), then tagged `gsd-ab-v2`
(section 10). A change after a tag is a new version, and the report says so.

This is the pre-registered protocol of the A/B the build plan's gate B reads: does an agent that keeps its project
state in a checked graph do better than one that keeps it in Markdown? The files beside this one are the corpus
(`tasks.json`), the seeding script (`seed.py`), the scorer (`score.py`), the arms' prompts (`prompts/`), the
operator's script (`OPERATOR.md`) and the review record (`REVIEW.md`). Run files and transcripts are never
committed.

## 1. Question and hypotheses

- **H1 (gate B).** Checked state (arm B) beats Markdown state (arm A) on false "verified" (M3) and on
  state drift (M2), without costing more than 10% in tokens per completed plan (M4).
- **H2 (secondary, the requirement thread).** The requirement thread helps an agent: arm B beats arm B0 on false "verified"
  for the defects the thread exists to catch, counted alone (M3 over those, `M3_thread`). They are the defects marked
  `"thread": true` in `tasks.json#defects`: S1, S5 and S6, whose signals read the test facts B0 lacks. H2 is reported
  and gates nothing. S4 is not a thread defect: B0 keeps `gsd/E521` (it reads commits, not test facts) and reports
  it as B does.
- **H3 (the context compiler's hypothesis).** 30% fewer context tokens per executor spawn: arm B's `gsd-executor`
  spawns start on at most 0.70 of arm Bv1's context (M7). H3 is reported and gates nothing.

## 2. Arms

Same model, same temperature, same tool budget, same task prompts. The prompt is `prompts/<arm>.md` with the task's
`prompt` in place of `{task.prompt}` and its workflow's `RESULT:` values in place of `{task.results}`, as `seed.py
prompt` prints it: the four files are one text except their commands block (between `<!-- commands -->` and
`<!-- /commands -->`), which names the arm's tools (B0's is B's, and Bv1's file is B's byte for byte), and all end asking for one `RESULT:` line, whose
value is the run's `final_report`. `tasks.json#results` gives each workflow's values, what each means, and whether it
claims the work done (`complete` for execute, `verified` for verify, `passed` for progress/resume and ship). Each arm's configuration is frozen in `tasks.json#arms`
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
| **Bv1** | arm B as the tag `gsd-ab-v1` froze it: the fixtures' trees, the `@gsd/*` components and the `specforge` binary built from the tag, with v2's tasks, seeds, prompt (B's) and measures | as B, with its own binary |
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

Each arm starts from the same commit of the fixture, in its own format; Bv1 starts from the tree the tag `gsd-ab-v1`
holds for it (`git rev-parse gsd-ab-v1:<fixture path>`), and `seed.py` refuses Bv1 where the tag is missing. The importer may not be used to make arm A's
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
| S1 | a test that never ran: a must-have or verdict claims a test that is not in the run | `code/E101` on the truth | `complete` on taskboard and ledger, whose cited truth is of the plan the run executes; `seed` on vault, whose cited truth's plan is already complete |
| S2 | a requirement id that does not exist (`requirements [TASK-09]`) | `E003` | `seed` |
| S3 | a status written ahead of the work: phase 02 marked complete before its plan ran | `E082` | `seed` |
| S4 | a commit trailer naming a task that does not exist: `Gsd-Task: 01-01/T9` under GSD's subject `feat(01-01): <T1's title>` | `gsd/E521` on the commit (its task found by the subject's title, not by the trailer) | `seed`, after `collect` |
| S5 | a truth verified over a failing proof | `code/W401` on the truth: a complete entity links a failed test | `seed`, after `collect` |
| S6 | a proof naming a deleted test | `code/E101` | `seed` |

`tasks.json`'s `b_signals` is the data, per task: the code, the entity, the count and when, and `ab_seeds` is the
proof: on a copy `seed.py` made, B reports each row's count where the unseeded copy of the same task reports none,
and B0 reports none of the thread's (`"thread": true`: S1, S5, S6). A seed that changes the test run (S5 fails a
test, S6 removes one: `run` in its seed) is also checked against its file edit alone: the seeded copy collected over
the recorded run unchanged reports none of its row, so the signal reads the failed or missing test, not the edited
file. `seed` means after seeding and a collect; `complete` means only after the task's `completion`, the edits a
correct run leaves. S1's holder is a truth of the plan the run executes,
and `@gsd/core` checks what a pending plan cites only once the plan completes: a run that ends with that plan still
pending has no B signal for S1, and that is correct, since it did not claim the plan complete.

The tests: `gsd/core/tests/cli/ab_seeds.rs` (taskboard and ledger) and `gsd/security/tests/cli/ab_seeds.rs` (vault)
check the table on every task, `gsd/core/tests/cli/ab_protocol.rs` checks `tasks.json`, the prompts and the fixtures'
pins and runs both self-tests, and `gsd/core/tests/cli/harness_ab.rs` holds what they share. `cargo gate full --only
gsd` runs them all, with the components built as [OPERATOR.md](OPERATOR.md) says.

A verdict goes stale when anything it read changes, the cited test file included: S5's and S6's edits to a test file
stale (`gsd/E501`) the verdicts of every truth that file's tests prove, the seeded truth's among them. That is
context, not a signal in the table: it fires on a harmless edit to the file as well. The signal is counted on the
seeded entity, so a row still checks the truth it names; the other stale verdicts are the same on every arm's copy.

## 6. Measures

Per arm, per (task, seed) run. The operator's scripts write a run file (section 7); `score.py` computes M1 to M7:
v1's fixed set, M1 to M6, and v2's M7 (measures of autonomy may join v2 before its tag, section 10):

| Id | Measure | From the run file |
|---|---|---|
| M1 | first-pass acceptance | every `acceptance` check passed on the first delivery |
| M2 | state-drift incidents | claims the agent leaves (status, progress, "complete", "verified") that differ from the task's `ground_truth`, each read from the copy after the run as `tasks.json#states` says for its kind and arm |
| M3 | false "verified" | seeded defects a run can remove still present at the end of a run whose `RESULT:` claims the work done (`"claims": true` in `tasks.json#results`) |
| M4 | tokens per completed plan | total tokens over plans completed, per arm |
| M5 | wall time and human interventions | `wall_seconds`, `interventions` |
| M6 | rework commits after verify | `rework_commits`: the run's commits after its first one that writes verification state (A: a `VERIFICATION.md`; B: a verdict, a `verdict:` line of `specforge-harness`), 0 when none does; `seed.py rework` counts them |
| M7 | context tokens per gsd-executor spawn | `spawns`: the context tokens a gsd-executor spawn holds at its first state change: a file tool, a mutating engine tool or a writing command, by MCP or by CLI (`tasks.json#state_writes`); its last request's when it changes none; `seed.py spawns` reads them from the run's transcript, cutting a Bash call into its `specforge` segments with its heredoc bodies dropped, at control operators, parentheses and backticks, past leading assignments, keywords and global flags; M7 is the mean over the run's `gsd-executor` spawns (none: no M7), and H3 pairs the runs' M7 |

M2 is mechanical. A ground truth key's kind (`tasks.json#states.kinds`) is the one whose `id` pattern it matches; its
value and the run's claim on it are words of that kind's `values`, and `score.py` refuses any other. Each kind says,
per arm, what is read (a command or a file), where in it, and the map from the tool's words to the kind's; B0 reads as
B. In B a value the agent wrote on a computed field (`E082`) is its claim, and a verdict gone stale reads as its
recorded outcome in both arms: a run that leaves a stale "verified" in place has left that claim.

S4 is seeded but not counted in M3: its commit stays in history whatever a run does, so every correct execute run
would score it in every arm (`"fixable": false` in `tasks.json#defects`). Each execute task judges it instead, by the
acceptance row "the report names the commit whose trailer names the wrong task", which M1 counts.

## 7. Run procedure

- The model, temperature and tools are fixed before the first run and recorded in every run file.
- 3 seeds per (task, arm); arms are interleaved by task, so drift in the model's service hits every arm alike.
- Each run starts a fresh agent session in the arm's copy and sends one message: `prompts/<arm>.md` with the task's
  `prompt` in place of `{task.prompt}`. Nothing else is said unless the script calls for an intervention.
- The operator follows a script, [OPERATOR.md](OPERATOR.md): the copies, each arm's tools, the prompt, when a run
  ends, how each field of the run file is read. An intervention is one of its four, with its text, logged with its
  turn and counted, never improvised.
- Every run file is `runs/<arm>/<task>/<seed>.json` and its transcript the directory
  `runs/transcripts/<arm>/<task>/<seed>/`: the host's session file `session.jsonl` and its subagents' files
  `subagents/agent-<id>.jsonl` with `agent-<id>.meta.json`, as the host wrote them (both git-ignored), archived with
  the report. `score.py` reads only `<arm>/<task>/<number>.json`, so a transcript is never read as a run.
  `seed.py spawns --transcript <dir>` reads that directory in Claude Code's layout (as of Claude Code 2.1.316) and
  refuses any other; the study's `host.agent` names the host.

A run file:

```json
{
  "arm": "B", "task": "taskboard-execute-02-01", "seed": 1, "model": "<id>", "temperature": 1,
  "protocol": { "commit": "<sha seed.py seed reports>", "specforge": "<specforge --version>" },
  "host": { "agent": "<program>", "version": "<version>", "machine": "<os, cpu, memory>" },
  "budget": { "max_turns": 200, "max_tokens": 400000 }, "permission_mode": "auto-approve inside the copy",
  "acceptance": { "npm test passes": true, "tasks.json is written after every change": true },
  "claims": { "v1.0/02/02-01": "complete" },
  "final_report": "verified",
  "defects_present": ["S4"],
  "tokens": 182000, "plans_completed": 1,
  "wall_seconds": 840, "rework_commits": 0,
  "interventions": 1, "intervention_log": [{ "reason": "question", "turn": 7 }],
  "spawns": [{ "agent": "gsd-executor", "context_tokens": 21400, "total_tokens": 96300 }]
}
```

Every field is required. `final_report` is the value of the run's last `RESULT:` line, or `none`; an intervention's
`reason` is one of OPERATOR.md's (`question`, `stopped`, `budget`, `operator`) and `interventions` is the log's
length. `acceptance` holds one boolean per acceptance row of the task, keyed by its text; `claims` one entry per key
of the task's `ground_truth`; `defects_present` only ids the task seeds; `spawns` one `{agent, context_tokens,
total_tokens}` per subagent, counts, the total never below the context. `score.py` refuses a run file that breaks
any of this, or whose path is not its arm, task and seed.

## 8. Analysis

- Runs are paired by (task, seed): an A run and a B run (and a B0 run, for H2) of the same task and seed. A run with
  no partner is refused, never dropped.
- Per measure, the paired difference B − A, with a 95% percentile bootstrap interval: 10,000 resamples over tasks, and
  over seeds within each resampled task, with a fixed RNG seed so a report reproduces.
- H2 is reported the same way: B − B0 on `M3_thread` (false "verified" counting only the thread's defects, S1, S5 and S6), with its
  interval and whether its upper bound is below 0. It gates nothing.
- H3 pairs B and Bv1 by (task, seed) where both have M7, takes each pair's ratio M7(B) / M7(Bv1), and bootstraps the
  mean ratio the same way; it is shown when the interval's upper bound is at most 0.70. It gates nothing.
- The seeds of a task are replicates only when the model samples: the operator fixes `temperature` above 0 for the
  study. At 0 the three seeds differ only by the host's own nondeterminism, and the interval over them is optimistic.

## 9. Gate B

**Pass** when the upper bound of B − A is below 0 for M3 and for M2 (B has fewer incidents), and M4(B) ≤ 1.10 × M4(A).
An arm that completed no plan has no M4, and the M4 clause then fails: no evidence is not a pass.
**Fail** otherwise: the work that waits on gate B does not start, and the result is published as it is.

## 10. Pre-registration

The tag `gsd-ab-v1` is set at v1's gate A, before any run, on the commit the review read; v2's tag `gsd-ab-v2` is set at
v2's gate A, after `gsd-ab-v1` exists. Each freezes:

- `evaluation/gsd-ab/`: this README, `tasks.json`, `seed.py`, `score.py`, `prompts/` (A.md, B.md, B0.md, Bv1.md),
  [OPERATOR.md](OPERATOR.md) and [REVIEW.md](REVIEW.md);
- the three fixtures, `gsd/parity/fixtures/{taskboard,vault,ledger}`, each pinned in `tasks.json` by its git tree
  (`tree`): `seed.py` copies that tree, refuses while the fixture or this directory has uncommitted changes, and
  refuses a fixture whose tree at HEAD is not the pinned one; `ab_protocol` fails on a fixture edited after its pin;
- the components arm B and B0 load, `@gsd/core` and `@gsd/security`, and the `specforge` binary, all built from the
  tagged commit (OPERATOR.md), and gsd-core 1.19.0 for arm A;
- the measures, M1 to M6 for v1 and M1 to M7 for v2 (section 6), with gate B's rule (section 9);
- for v2, the reference to the tag Bv1 reads (`tasks.json#arms`, `"tag": "gsd-ab-v1"`).

Every run file records its `protocol`: the commit `seed.py seed` reports and `specforge --version`. `score.py`
refuses a run set that mixes two protocol commits or two `specforge` versions of one build (A, B and B0 share one,
Bv1 records the tag's), a set whose Bv1 runs record B's, and a run whose copy came from a working tree
(`seed.py --worktree`, for the tests). A change after the tag, a fixture's included, is a new version, and the report names it.

Open before v2's tag: `gsd-ab-v1` set; Bv1's preflight run on the tag's build (`seed.py preflight --arm Bv1`); the
autonomy measures and a project from outside may join v2 before its tag, or come in a later version. Open before
v1's tag, each needing gsd-core 1.19.0 installed (`gsd-tools`, node 24):

- the ledger's JSON goldens, recorded by `gsd/parity/record-ledger.sh`, and the comparison of the statuses gsd-tools
  reads from its `.planning/` with the ledger's `.spec` twin;
- a second `gsd/parity/record-security.sh` run, which must give the vault's goldens unchanged;
- arm A's seeds read through gsd-tools: each seeded A copy reads without an error, and the read shows the seed;
- the A reads in `tasks.json#states` (each kind's command, path and map of words) checked against what gsd-tools
  1.19.0 prints on the fixtures.

## 11. Threats

| Threat | Mitigation |
|---|---|
| One model | name it; a second model's run is a new version, not a retry |
| Fixtures written by the authors: all three are | every fixture, seed, completion, acceptance row and prompt is in the tagged commit; each `.planning/` is hand-written from gsd-core's templates; taskboard's and vault's are read by gsd-tools 1.19.0 (the `record*.sh` scripts fail on a verb that errors), the ledger's not yet (its goldens are open, section 10); one fixture (ledger) is red from the start, so a "verified" is false there with no seed. A project from outside, replayed through `@gsd/import`, cannot be in v1: the importer is built after the tag, and arm B's input would come from code the tag does not hold (section 10). It is a threat a later version may close (section 10) |
| Arm B's prompts are newer than arm A's | both arms' prompts are frozen at gate A (`prompts/`), and each arm's agents and skills come from its release unchanged (gsd-core 1.19.0, `@gsd/core` at the tag) |
| B and Bv1 start from different fixture trees when v2 moved a pin | Bv1 runs the trees its own build reads (a digest the newer build records would not match the older), so each arm meets the fixture its build was made for; the tag names the trees |
| The operator's hand in interventions | [OPERATOR.md](OPERATOR.md): four reasons, each with its text, logged with its turn and counted in M5; `score.py` refuses any other |

## Score

```
python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs
python3 -I evaluation/gsd-ab/score.py --self-test
```

`--runs` exits 0 when gate B passes, 1 when it fails, and 2 when the run set is refused (a bad or unpaired run file,
or none), with the reason on standard error.

`--self-test` writes an embedded synthetic set (4 tasks, 3 seeds, 4 arms) as run files and reads it back, scores it
where B clearly wins and where it doesn't, checks the gate B verdict of each and that an unpaired run is refused, and
refuses exactly fourteen bad run files: one without its host, one without its spawns, one whose spawn's context is
not a count, one whose spawn's total is below its context, one whose interventions are not logged, one whose
intervention has a reason outside the list, one whose acceptance value is not a boolean, one with an acceptance row
its task does not have, one with a claim on a key outside the task's ground truth, two whose `defects_present` holds
what the task does not seed (an unknown id, a number), one whose claim is outside its key's vocabulary, one whose
`final_report` is not a `RESULT:` value of its task's workflow, and one whose copy was a working tree's
(`protocol.commit` ending in `+worktree`). It also refuses a set whose runs record two protocol commits, and one whose
Bv1 runs record B's build.

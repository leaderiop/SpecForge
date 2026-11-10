# GSD A/B operator script

What the operator does, in order, for every run of the A/B ([README.md](README.md)). Nothing outside this script is
done during a study, and everything in it is done the same way in every arm. `score.py` refuses a run file that
breaks it.

## Before the first run

1. Fix, once for the whole study, what every run file records unchanged:
   - `model` (the model id) and `temperature`;
   - `host`: `agent` (the agent program), `version` (its version) and `machine` (OS, CPU, memory);
   - `budget`: `max_turns` and `max_tokens`, the tool and turn budget of one run;
   - `permission_mode`: the agent's tools are auto-approved inside the copy and nowhere else.
2. Install outside the copies: Python 3 (it runs `seed.py` and `score.py`), git, node (arm A's gsd-core and the Node
   fixtures' tests), cargo with `cargo-nextest` (the ledger's tests), and the `specforge` binary built from the tagged
   commit with its `@gsd/core` and `@gsd/security` components.
3. Print the run order: `python3 -I evaluation/gsd-ab/seed.py schedule`. Each line is a task, a seed and its arms in
   the order to run them; runs follow the lines, so drift in the model's service reaches every arm alike.

## One run

For each `(task, seed, arm)` in the schedule's order:

1. **The copy.** `python3 -I evaluation/gsd-ab/seed.py seed --task <task> --arm <arm> --out <copy> --gsd-core
   <gsd_core.wasm> --gsd-security <gsd_security.wasm>`, `<copy>` outside this repository. It is a git repository whose
   commits hold the seeds; its HEAD is the same for every seed number of one (task, arm).
2. **The arm's tools, in the copy only.** Arm A: gsd-core 1.19.0's own installer, local to the copy
   (`npx @opengsd/gsd-core@1.19.0 --claude --local`, or the flag of the agent the host names). Arms B and B0:
   `specforge install claude-code` (or the host's name) from the tagged commit, then `specforge collect --yes`. Agents
   and skills are each arm's release's, unchanged.
3. **The prompt.** A fresh agent session in the copy, no memory of another run. One message: `prompts/<arm>.md` with
   the task's `prompt` (`tasks.json`) in place of `{task.prompt}`. Start the clock.
4. **During the run** the operator says nothing but an intervention (below).
5. **The end.** The run ends when the agent's turn ends with a `RESULT:` line, after a second stop without one, or
   when the budget is reached. Stop the clock (`wall_seconds`).
6. **The run file**, `evaluation/gsd-ab/runs/<arm>/<task>/<seed>.json` (git-ignored; the transcript beside it as
   `<seed>.transcript.json`), with the fields of README section 7:
   - `final_report`: the value of the agent's last `RESULT:` line (`verified`, `complete`, `passed`, `not verified`,
     `blocked`), or `none` when the run ended without one.
   - `acceptance`: one entry per acceptance row of the task, keyed by its `text`. A row with `run` is that command, run
     in the copy: `true` when it exits 0. A row with `judge` is the operator's reading of the copy against that
     one-line rubric, never of the agent's report.
   - `claims`: for each key of the task's `ground_truth`, the state the agent left, in `ground_truth`'s words, read
     from the arm's own store: A with gsd-tools (`roadmap analyze`, `state json`, `progress json`, and the phase's
     `VERIFICATION.md` for a truth); B and B0 with `specforge query <id> --depth 0 --format json`.
   - `defects_present`: `python3 -I evaluation/gsd-ab/seed.py present --task <task> --arm <arm> --dir <copy>`,
     computed, never typed.
   - `tokens` from the host's usage report; `plans_completed`: the plans the store holds complete that were not
     before the run; `rework_commits`: the commits after the first one, or the first message, that claimed a
     verification.
   - `interventions` and `intervention_log` (below), and the fixed fields of "Before the first run".

## Interventions

The operator steps in on these triggers only, with the text given, and appends `{"reason": ..., "turn": <the agent's
turn number>}` to `intervention_log`; `interventions` is its length.

| Reason | Trigger | The operator |
|---|---|---|
| `question` | the agent asks the person something and waits | says "Proceed with your best judgement." |
| `stopped` | the agent's turn ends with no `RESULT:` line | says "Continue until you can report.", once; a second stop ends the run, `final_report` `none` |
| `budget` | the run reaches `max_turns` or `max_tokens` | ends the run, `final_report` `none` |
| `operator` | the operator made a mistake (a wrong message, a late stop) | logs it; the run is kept |

Nothing else is said, and nothing is done to the copy during a run.

## Scoring

Once every scheduled run has its file: `python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs`. It
checks every run file, pairs the runs by task and seed, and refuses an unpaired one: a missing run is run, never
dropped.

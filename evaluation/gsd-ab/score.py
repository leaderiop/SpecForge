#!/usr/bin/env python3
"""Score the GSD A/B (evaluation/gsd-ab/README.md): M1-M6 per run, paired B - A and B - B0
differences with bootstrap intervals, and the gate B (D7) verdict.

Usage:
    python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs
    python3 -I evaluation/gsd-ab/score.py --self-test

`--runs` reads every `<arm>/<task>/<seed>.json` under the directory (README section 7), `<seed>` a number:
another file there (a note) is skipped and named, and transcripts live under `transcripts/`, outside the
pattern. It refuses a run file that lacks a field, has one of the wrong type, or logs an intervention the
operator's script (OPERATOR.md) does not list. Runs pair by (task, seed); a run whose partner is missing is
refused.
Standard library only, Python 3.11+.
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import random
import re
import sys
import tempfile
from pathlib import Path

TASKS = Path(__file__).resolve().parent / "tasks.json"
RESAMPLES = 10_000
RNG_SEED = 20261102
# A run's `final_report` is a value of its workflow's `results` (tasks.json), or this when it ended without one.
NO_RESULT = "none"
# Every run file holds these fields, of these types (README section 7, OPERATOR.md).
FIELDS = {
    "arm": str, "task": str, "seed": int, "model": str, "temperature": (int, float), "host": dict, "budget": dict,
    "permission_mode": str, "acceptance": dict, "claims": dict, "final_report": str, "defects_present": list,
    "tokens": int, "plans_completed": int, "wall_seconds": (int, float), "interventions": int,
    "intervention_log": list, "rework_commits": int,
}
HOST = ("agent", "version", "machine")
BUDGET = ("max_turns", "max_tokens")
# The operator's only reasons to step in (OPERATOR.md); `operator` logs the operator's own error.
REASONS = {"question", "stopped", "budget", "operator"}
# The paired measures, each "lower is better" except M1.
MEASURES = ("M1", "M2", "M3", "M5_wall", "M5_interventions", "M6")


class Unpaired(ValueError):
    """A run with no partner run of the same (task, seed) in the other arm."""


def claiming(corpus: dict, task: dict) -> set[str]:
    """The `RESULT:` values of the task's workflow that claim the work done (`"claims": true` in tasks.json)."""
    return {result["value"] for result in corpus["results"][task["workflow"]] if result["claims"]}


def kind_of(corpus: dict, key: str) -> dict | None:
    """The kind of `states` (tasks.json) whose id pattern a ground truth key matches, or None."""
    kinds = [kind for kind in corpus["states"]["kinds"] if re.fullmatch(kind["id"], key)]
    return kinds[0] if len(kinds) == 1 else None


def measures(run: dict, task: dict, corpus: dict) -> dict:
    """M1-M6 of one run (README section 6), `corpus` the tasks.json it belongs to. M4 is per arm, so it is the run's
    tokens and plans. M3 counts only the defects a run can remove: one marked `"fixable": false` (a seeded commit
    stays in history) is judged by an acceptance row instead."""
    defects = {defect["id"]: defect for defect in corpus.get("defects", [])}
    truth = task.get("ground_truth", {})
    claims = run.get("claims", {})
    drift = sum(1 for key, state in claims.items() if key in truth and truth[key] != state)
    seeded = {defect for defect in task.get("seeded", []) if defects.get(defect, {}).get("fixable", True)}
    # The defects the requirement thread exists to catch (`"thread": true`): H2 counts false "verified" over these.
    thread = {defect for defect, about in defects.items() if about.get("thread")}
    claimed = run.get("final_report") in claiming(corpus, task)
    present = seeded & set(run.get("defects_present", [])) if claimed else set()
    return {
        "M1": 1.0 if all(run.get("acceptance", {}).values()) and run.get("acceptance") else 0.0,
        "M2": float(drift),
        "M3": float(len(present)),
        "M3_thread": float(len(present & thread)),
        "tokens": float(run.get("tokens", 0)),
        "plans": float(run.get("plans_completed", 0)),
        "M5_wall": float(run.get("wall_seconds", 0)),
        "M5_interventions": float(run.get("interventions", 0)),
        "M6": float(run.get("rework_commits", 0)),
    }


def pair(runs: list[dict], tasks: dict, treated: str, control: str) -> dict:
    """{task: {seed: (treated measures, control measures)}} over the runs of the two arms. Raises
    Unpaired when a run of either arm has no partner, and ValueError for an unknown task."""
    by_id = {task["id"]: task for task in tasks["tasks"]}
    held: dict[tuple[str, str, int], dict] = {}
    for run in runs:
        if run["task"] not in by_id:
            raise ValueError(f"{run['arm']}/{run['task']}/{run['seed']}: no such task in tasks.json")
        if run["arm"] in (treated, control):
            held[(run["arm"], run["task"], run["seed"])] = measures(run, by_id[run["task"]], tasks)
    pairs: dict[str, dict[int, tuple[dict, dict]]] = {}
    for (arm, task, seed), _ in sorted(held.items()):
        other = control if arm == treated else treated
        if (other, task, seed) not in held:
            raise Unpaired(f"{arm}/{task}/{seed}: no {other} run of the same task and seed")
        if arm == treated:
            pairs.setdefault(task, {})[seed] = (held[(arm, task, seed)], held[(other, task, seed)])
    return pairs


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def difference(pairs: dict, measure: str) -> float:
    return mean([t[measure] - c[measure] for seeds in pairs.values() for t, c in seeds.values()])


def interval(pairs: dict, measure: str, rng: random.Random) -> tuple[float, float]:
    """The 95% percentile bootstrap interval of the mean paired difference: tasks resampled, then
    seeds within each resampled task (README section 8)."""
    tasks = sorted(pairs)
    means = []
    for _ in range(RESAMPLES):
        values = []
        for task in rng.choices(tasks, k=len(tasks)):
            seeds = list(pairs[task].values())
            for t, c in rng.choices(seeds, k=len(seeds)):
                values.append(t[measure] - c[measure])
        means.append(mean(values))
    means.sort()
    return means[int(0.025 * RESAMPLES)], means[int(0.975 * RESAMPLES) - 1]


def tokens_per_plan(pairs: dict, side: int) -> float | None:
    """M4 of one side of `pairs` (0 the treated arm, 1 the control): its tokens over its completed plans, or None
    when it completed none, which is no evidence either way."""
    runs = [seeds[seed][side] for seeds in pairs.values() for seed in seeds]
    plans = sum(run["plans"] for run in runs)
    return sum(run["tokens"] for run in runs) / plans if plans else None


def report(pairs: dict, label: str) -> dict:
    """Print each measure's paired difference and interval; return {measure: (diff, low, high)}."""
    rng = random.Random(RNG_SEED)
    out = {}
    print(f"{label}: {sum(len(s) for s in pairs.values())} pairs over {len(pairs)} tasks")
    for measure in MEASURES:
        low, high = interval(pairs, measure, rng)
        out[measure] = (difference(pairs, measure), low, high)
        print(f"  {measure:<17} {out[measure][0]:+8.3f}  [{low:+.3f}, {high:+.3f}]")
    for side, arm in ((0, label[0:2].strip()), (1, label[-2:].strip())):
        m4 = tokens_per_plan(pairs, side)
        print(f"  M4 {arm:<14} " + ("no completed plan" if m4 is None else f"{m4:8.0f} tokens per completed plan"))
    return out


def d7(pairs: dict, diffs: dict) -> bool:
    """Gate B: B - A on M3 and M2 has an upper bound below 0, and M4(B) <= 1.10 x M4(A). An arm that completed no
    plan has no M4, and the M4 clause fails: no evidence is not a pass."""
    b, a = tokens_per_plan(pairs, 0), tokens_per_plan(pairs, 1)
    return diffs["M3"][2] < 0 and diffs["M2"][2] < 0 and a is not None and b is not None and b <= 1.10 * a


def h2(runs: list[dict], tasks: dict) -> bool:
    """H2 (README section 1): B - B0 on false "verified" over the thread's defects (`"thread": true`) only; True when
    its upper bound is below 0. Reported, gates nothing."""
    pairs = pair(runs, tasks, "B", "B0")
    low, high = interval(pairs, "M3_thread", random.Random(RNG_SEED))
    holds = high < 0
    print(
        f"H2: B - B0 on M3_thread {difference(pairs, 'M3_thread'):+.3f} [{low:+.3f}, {high:+.3f}]: "
        f"{'B beats B0' if holds else 'not shown'}"
    )
    return holds


def score(runs: list[dict], tasks: dict) -> tuple[bool, bool | None]:
    """The D7 verdict, and H2's (None without B0 runs)."""
    ab = pair(runs, tasks, "B", "A")
    passed = d7(ab, report(ab, "B - A"))
    thread = h2(runs, tasks) if any(run["arm"] == "B0" for run in runs) else None
    print(f"D7: {'pass' if passed else 'fail'}")
    return passed, thread


def check_run(run: dict, tasks: dict, where: str) -> None:
    """Raises ValueError naming the first thing a run file lacks or gets wrong (README section 7), against its task
    in `tasks`: one boolean per acceptance row, keyed by its text; one claim per ground truth key; and seeded
    defects only in `defects_present`."""
    for key, kind in FIELDS.items():
        if key not in run:
            raise ValueError(f"{where}: no `{key}`")
        if isinstance(run[key], bool) or not isinstance(run[key], kind):
            raise ValueError(f"{where}: `{key}` is {type(run[key]).__name__}, not {kind}")
    if run["arm"] not in ("A", "B", "B0"):
        raise ValueError(f"{where}: arm {run['arm']!r} is none of A, B and B0")
    for block, keys in (("host", HOST), ("budget", BUDGET)):
        missing = [key for key in keys if key not in run[block]]
        if missing:
            raise ValueError(f"{where}: `{block}` has no {', '.join(missing)}")
    log = run["intervention_log"]
    if run["interventions"] != len(log):
        raise ValueError(f"{where}: {run['interventions']} interventions counted, {len(log)} logged")
    for entry in log:
        if not (isinstance(entry, dict) and isinstance(entry.get("turn"), int)):
            raise ValueError(f"{where}: an intervention is logged as {{reason, turn}}, not {entry!r}")
        if entry.get("reason") not in REASONS:
            raise ValueError(f"{where}: intervention reason {entry.get('reason')!r} is none of {sorted(REASONS)}")
    task = next((task for task in tasks["tasks"] if task["id"] == run["task"]), None)
    if task is None:
        raise ValueError(f"{where}: no task {run['task']!r} in tasks.json")
    results = [result["value"] for result in tasks["results"][task["workflow"]]] + [NO_RESULT]
    if run["final_report"] not in results:
        raise ValueError(f"{where}: final_report {run['final_report']!r} is none of {task['workflow']}'s {results}")
    for row, passed in run["acceptance"].items():
        if not isinstance(passed, bool):
            raise ValueError(f"{where}: acceptance {row!r} is {passed!r}, not true or false")
    rows = sorted(row["text"] for row in task["acceptance"])
    if sorted(run["acceptance"]) != rows:
        raise ValueError(f"{where}: acceptance is keyed {sorted(run['acceptance'])}, not by the task's rows {rows}")
    if sorted(run["claims"]) != sorted(task["ground_truth"]):
        truth = sorted(task["ground_truth"])
        raise ValueError(f"{where}: claims are keyed {sorted(run['claims'])}, not by the ground truth's {truth}")
    for key, state in run["claims"].items():
        kind = kind_of(tasks, key)
        if kind is None or state not in kind["values"]:
            words = kind["values"] if kind else "no kind's"
            raise ValueError(f"{where}: claim {key}: {state!r} is none of its vocabulary, {words}")
    for defect in run["defects_present"]:
        if not isinstance(defect, str) or defect not in task["seeded"]:
            raise ValueError(f"{where}: defects_present holds {defect!r}, not one of the task's seeds {task['seeded']}")


def read_runs(root: Path, tasks: dict) -> list[dict]:
    """Every `<arm>/<task>/<seed>.json` under `root` whose `<seed>` is a number, each checked (check_run) and
    matching its path. Another file there is skipped, and named on standard error."""
    runs = []
    for path in sorted(root.glob("*/*/*.json")):
        if not path.stem.isdigit():
            print(f"skipped {path}: not a run file (<arm>/<task>/<seed>.json)", file=sys.stderr)
            continue
        run = json.loads(path.read_text())
        check_run(run, tasks, str(path))
        arm, task, seed = path.parts[-3], path.parts[-2], path.stem
        if (run["arm"], run["task"], str(run["seed"])) != (arm, task, seed):
            raise ValueError(f"{path}: arm, task and seed differ from the file's path")
        runs.append(run)
    return runs


def status(root: Path, tasks: dict) -> int:
    """The exit status of scoring the run files under `root`: 0 when gate B passes, 1 when it fails, and 2 when the
    run set is refused (a bad or unpaired run file, or none), with the reason on standard error."""
    try:
        runs = read_runs(root, tasks)
        if not runs:
            raise ValueError(f"no run file under {root}")
        return 0 if score(runs, tasks)[0] else 1
    except ValueError as refusal:
        print(f"score.py: {refusal}", file=sys.stderr)
        return 2


def write_runs(root: Path, runs: list[dict]) -> None:
    for run in runs:
        path = root / run["arm"] / run["task"] / f"{run['seed']}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(run))


# The synthetic corpus's defects, results and states: one thread defect, a verify workflow, and one kind of key.
VOCABULARY = {
    "defects": [{"id": "S1", "thread": True}, {"id": "S2"}, {"id": "S4", "fixable": False}],
    "results": {"verify": [{"value": "verified", "claims": True}, {"value": "not verified", "claims": False}]},
    "states": {"kinds": [{"kind": "plan", "id": "^p$", "values": ["planned", "complete"]}]},
}


def synthetic(b_wins: bool, b_tokens: float = 1.0, a_plans: int = 1) -> tuple[list[dict], dict]:
    """Four tasks seeded with S1 (a thread defect) and S2 (not one), three seeds, arms A, B and B0. A leaves both
    defects and drifts. B leaves none and no drift when it wins, and repeats A when it doesn't. B0 always leaves S1
    but not S2, so B beats B0 on the thread's defects exactly when B wins. B spends `b_tokens` times A's tokens, and A
    completes `a_plans` plans a run (every other arm one)."""
    tasks = {**VOCABULARY, "tasks": [
        {"id": f"t{i}", "workflow": "verify", "seeded": ["S1", "S2"], "ground_truth": {"p": "complete"},
         "acceptance": [{"text": "tests", "run": "true"}]}
        for i in range(4)
    ]}
    runs = []
    for i in range(4):
        for seed in range(1, 4):
            bad = {"claims": {"p": "planned"}, "defects_present": ["S1", "S2"]}
            good = {"claims": {"p": "complete"}, "defects_present": []}
            thread_off = {"claims": {"p": "complete"}, "defects_present": ["S1"]}
            arms = (("A", bad), ("B", good if b_wins else bad), ("B0", thread_off))
            for arm, state in arms:
                runs.append({
                    "arm": arm, "task": f"t{i}", "seed": seed, "model": "synthetic", "temperature": 0,
                    "host": {"agent": "synthetic", "version": "0", "machine": "none"},
                    "budget": {"max_turns": 200, "max_tokens": 400_000}, "permission_mode": "auto in the copy",
                    "final_report": "verified",
                    "acceptance": {"tests": arm == "B" and b_wins},
                    "tokens": round((100_000 + seed) * (b_tokens if arm == "B" else 1.0)),
                    "plans_completed": a_plans if arm == "A" else 1, "wall_seconds": 600, "interventions": 0,
                    "intervention_log": [], "rework_commits": 0, **state,
                })
    return runs, tasks


def refused_file(what: str, run: dict, tasks: dict) -> None:
    """check_run must refuse `run`; its message is printed."""
    try:
        check_run(run, tasks, f"{run.get('arm')}/{run.get('task')}/{run.get('seed')}.json")
    except ValueError as refusal:
        print(f"refused run file: {refusal}")
    else:
        raise AssertionError(f"{what} must be refused")


def self_test() -> None:
    runs, tasks = synthetic(b_wins=True)
    with tempfile.TemporaryDirectory() as tmp:
        write_runs(Path(tmp), runs)
        # A file beside the runs that is not one (an operator's note) is skipped, not read as a run.
        (Path(tmp) / "A" / "t0" / "1.notes.json").write_text("{}")
        read = read_runs(Path(tmp), tasks)
    assert len(read) == len(runs), "every written run file reads back, and the stray file is skipped"
    assert score(read, tasks) == (True, True), "a set B clearly wins must pass D7 and show H2"
    runs, tasks = synthetic(b_wins=False)
    assert score(runs, tasks) == (False, False), "a set B does not win must fail D7 and not show H2"
    # B wins on M2 and M3 but spends 20% more tokens per plan, or A completes no plan, so there is no M4 to hold B
    # to: gate B fails either way.
    for costly in (synthetic(b_wins=True, b_tokens=1.2), synthetic(b_wins=True, a_plans=0)):
        assert score(*costly)[0] is False, "M4 holds gate B: over 1.10 x A, or no plan completed, fails"
    # A bootstrap interval against a hand-computed one: one task, ten seeds whose paired differences are 0 to 9. The
    # mean of ten draws from them has mean 4.5 and standard deviation sqrt(8.25 / 10) = 0.908, so its 95% interval is
    # about 4.5 -+ 1.96 x 0.908 = [2.72, 6.28].
    spread = {"t": {seed: ({"M2": float(seed)}, {"M2": 0.0}) for seed in range(10)}}
    low, high = interval(spread, "M2", random.Random(RNG_SEED))
    print(f"bootstrap over 0..9: [{low:.2f}, {high:.2f}]")
    assert difference(spread, "M2") == 4.5 and abs(low - 2.72) <= 0.2 and abs(high - 6.28) <= 0.2, (low, high)
    # H2 counts the thread's defects only: S2 left by B0 alone would not count.
    measured = measures(
        {"final_report": "verified", "defects_present": ["S2"]}, {"workflow": "verify", "seeded": ["S1", "S2"]},
        VOCABULARY,
    )
    assert (measured["M3"], measured["M3_thread"]) == (1.0, 0.0), measured
    # A defect no run can remove (its seeded commit stays in history) is out of M3.
    measured = measures(
        {"final_report": "verified", "defects_present": ["S1", "S4"]}, {"workflow": "verify", "seeded": ["S1", "S4"]},
        VOCABULARY,
    )
    assert measured["M3"] == 1.0, measured
    runs, tasks = synthetic(b_wins=True)
    try:
        score([run for run in runs if run["arm"] != "B0"][:-1], tasks)
    except Unpaired as refusal:
        print(f"refused: {refusal}")
    else:
        raise AssertionError("an unpaired run must be refused")
    # A refused set exits 2 with its reason on standard error, never 1 (gate B fails) or a traceback.
    with tempfile.TemporaryDirectory() as tmp:
        said = io.StringIO()
        with contextlib.redirect_stderr(said):
            empty = status(Path(tmp), tasks)
            write_runs(Path(tmp), [run for run in runs if run["arm"] != "B0"][:-1])
            unpaired = status(Path(tmp), tasks)
        assert (empty, unpaired) == (2, 2), (empty, unpaired)
        assert said.getvalue().count("score.py: ") == 2 and "no B run" in said.getvalue(), said.getvalue()
    # The run files' own refusals, counted: exactly these ten.
    run = runs[0]
    for what, bad in (
        ("a run file without its host", {key: value for key, value in run.items() if key != "host"}),
        ("an intervention counted and not logged", {**run, "interventions": 1}),
        ("an intervention for a reason outside the list",
         {**run, "interventions": 1, "intervention_log": [{"reason": "nudge", "turn": 3}]}),
        ("an acceptance row that is not true or false", {**run, "acceptance": {"tests": "yes"}}),
        ("an acceptance row the task does not have", {**run, "acceptance": {"tests": True, "lint": True}}),
        ("a claim on a key the ground truth does not have", {**run, "claims": {"p": "complete", "q": "complete"}}),
        ("a defect the task does not seed", {**run, "defects_present": ["S9"]}),
        ("a defect that is not a name", {**run, "defects_present": [1]}),
        ("a claim outside its key's vocabulary", {**run, "claims": {"p": "done"}}),
        ("a RESULT the task's workflow does not have", {**run, "final_report": "complete"}),
    ):
        refused_file(what, bad, tasks)
    print("self-test: ok")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--runs", type=Path, help="the runs directory: <arm>/<task>/<seed>.json")
    group.add_argument("--self-test", action="store_true", help="score the embedded synthetic sets")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    sys.exit(status(args.runs, json.loads(TASKS.read_text())))


if __name__ == "__main__":
    main()

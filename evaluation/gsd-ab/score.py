#!/usr/bin/env python3
"""Score the GSD A/B (evaluation/gsd-ab/README.md): M1-M6 per run, paired B - A and B - B0
differences with bootstrap intervals, and the gate B (D7) verdict.

Usage:
    python3 -I evaluation/gsd-ab/score.py --runs evaluation/gsd-ab/runs
    python3 -I evaluation/gsd-ab/score.py --self-test

`--runs` reads every `<arm>/<task>/<seed>.json` under the directory (README section 7). Runs pair
by (task, seed); a run whose partner is missing is refused. Standard library only, Python 3.11+.
"""

from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path

TASKS = Path(__file__).resolve().parent / "tasks.json"
RESAMPLES = 10_000
RNG_SEED = 20261102
CLAIMED = {"verified", "complete", "passed"}
# The seeded defects the requirement thread exists to catch: H2 counts false "verified" over these only.
THREAD = {"S1", "S4", "S5"}
# The paired measures, each "lower is better" except M1.
MEASURES = ("M1", "M2", "M3", "M5_wall", "M5_interventions", "M6")


class Unpaired(ValueError):
    """A run with no partner run of the same (task, seed) in the other arm."""


def measures(run: dict, task: dict) -> dict:
    """M1-M6 of one run (README section 6). M4 is per arm, so it is the run's tokens and plans."""
    truth = task.get("ground_truth", {})
    claims = run.get("claims", {})
    drift = sum(1 for key, state in claims.items() if key in truth and truth[key] != state)
    seeded = set(task.get("seeded", []))
    claimed = run.get("final_report") in CLAIMED
    present = seeded & set(run.get("defects_present", [])) if claimed else set()
    return {
        "M1": 1.0 if all(run.get("acceptance", {}).values()) and run.get("acceptance") else 0.0,
        "M2": float(drift),
        "M3": float(len(present)),
        "M3_thread": float(len(present & THREAD)),
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
            held[(run["arm"], run["task"], run["seed"])] = measures(run, by_id[run["task"]])
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


def tokens_per_plan(pairs: dict, side: int) -> float:
    runs = [seeds[seed][side] for seeds in pairs.values() for seed in seeds]
    plans = sum(run["plans"] for run in runs)
    return sum(run["tokens"] for run in runs) / plans if plans else float("inf")


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
        print(f"  M4 {arm:<14} {tokens_per_plan(pairs, side):8.0f} tokens per completed plan")
    return out


def d7(pairs: dict, diffs: dict) -> bool:
    """Gate B: B - A on M3 and M2 has an upper bound below 0, and M4(B) <= 1.10 x M4(A)."""
    return (
        diffs["M3"][2] < 0
        and diffs["M2"][2] < 0
        and tokens_per_plan(pairs, 0) <= 1.10 * tokens_per_plan(pairs, 1)
    )


def h2(runs: list[dict], tasks: dict) -> bool:
    """H2 (README section 1): B - B0 on false "verified" over the thread's defects (S1, S4, S5) only; True when
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


def read_runs(root: Path) -> list[dict]:
    runs = []
    for path in sorted(root.glob("*/*/*.json")):
        run = json.loads(path.read_text())
        arm, task, seed = path.parts[-3], path.parts[-2], path.stem
        if (run.get("arm"), run.get("task"), str(run.get("seed"))) != (arm, task, seed):
            raise ValueError(f"{path}: arm, task and seed differ from the file's path")
        runs.append(run)
    return runs


def synthetic(b_wins: bool) -> tuple[list[dict], dict]:
    """Four tasks seeded with S1 (a thread defect) and S2 (not one), three seeds, arms A, B and B0. A leaves both
    defects and drifts. B leaves none and no drift when it wins, and repeats A when it doesn't. B0 always leaves S1
    but not S2, so B beats B0 on the thread's defects exactly when B wins."""
    tasks = {"tasks": [
        {"id": f"t{i}", "seeded": ["S1", "S2"], "ground_truth": {"p": "complete"}} for i in range(4)
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
                    "arm": arm, "task": f"t{i}", "seed": seed, "final_report": "verified",
                    "acceptance": {"tests": arm == "B" and b_wins}, "tokens": 100_000 + seed,
                    "plans_completed": 1, "wall_seconds": 600, "interventions": 0,
                    "rework_commits": 0, **state,
                })
    return runs, tasks


def self_test() -> None:
    runs, tasks = synthetic(b_wins=True)
    assert score(runs, tasks) == (True, True), "a set B clearly wins must pass D7 and show H2"
    runs, tasks = synthetic(b_wins=False)
    assert score(runs, tasks) == (False, False), "a set B does not win must fail D7 and not show H2"
    # H2 counts the thread's defects only: S2 left by B0 alone would not count.
    measured = measures(
        {"final_report": "verified", "defects_present": ["S2"]}, {"seeded": ["S1", "S2"]}
    )
    assert (measured["M3"], measured["M3_thread"]) == (1.0, 0.0), measured
    runs, tasks = synthetic(b_wins=True)
    try:
        score([run for run in runs if run["arm"] != "B0"][:-1], tasks)
    except Unpaired as refused:
        print(f"refused: {refused}")
    else:
        raise AssertionError("an unpaired run must be refused")
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
    tasks = json.loads(TASKS.read_text())
    sys.exit(0 if score(read_runs(args.runs), tasks)[0] else 1)


if __name__ == "__main__":
    main()

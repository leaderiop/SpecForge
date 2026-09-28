#!/usr/bin/env python3
"""C1-07: score first-attempt accuracy for the north-star eval.

Usage:
    python3 evaluation/score.py --runs evaluation/runs/<label>.json

Each run file maps task id -> {"output": "<agent's first-delivered text or
patch>"}. The scorer loads evaluation/tasks.json, checks that every
expected entity id is referenced (file path or symbol containment) and none
of the forbidden ids are, and prints per-task and aggregate accuracy for
the label. Compare the with-graph label against the without-graph label:

    python3 evaluation/score.py --runs evaluation/runs/with-graph.json
    python3 evaluation/score.py --runs evaluation/runs/without-graph.json

The north-star claim (70-85% with graph vs ~30% baseline) is falsifiable
once both labels exist for all tasks.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

TASKS = Path(__file__).resolve().parent / "tasks.json"


def load_tasks() -> dict:
    return json.loads(TASKS.read_text())


def referenced(output: str, entity_id: str) -> bool:
    # Semantic containment: the id itself, or its snake/camel spellings,
    # anywhere in the delivered text.
    patterns = [
        re.escape(entity_id),
        re.escape(entity_id.replace("_", "")),
        re.escape(entity_id.replace("_", "-")),
    ]
    return any(re.search(p, output, re.IGNORECASE) for p in patterns)


def score_run(run_path: Path, tasks: dict) -> float:
    runs = json.loads(run_path.read_text())
    passed = 0
    total = 0
    for task in tasks["tasks"]:
        task_id = task["id"]
        entry = runs.get(task_id)
        if entry is None:
            print(f"  MISS  {task_id} (no run recorded)")
            total += 1
            continue
        output = entry.get("output", "")
        ok = all(referenced(output, e) for e in task["expected_entities"]) and not any(
            referenced(output, e) for e in task["forbidden_entities"]
        )
        print(f"  {'PASS' if ok else 'FAIL'}  {task_id}")
        passed += ok
        total += 1
    accuracy = passed / total if total else 0.0
    print(f"\n{run_path.name}: {passed}/{total} first-attempt accuracy = {accuracy:.0%}")
    return accuracy


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", required=True, type=Path, help="run file to score")
    args = parser.parse_args()

    tasks = load_tasks()
    score_run(args.runs, tasks)


if __name__ == "__main__":
    main()

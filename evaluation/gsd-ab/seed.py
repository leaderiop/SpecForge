#!/usr/bin/env python3
"""Make each arm's copy of a GSD A/B task's fixture (evaluation/gsd-ab/README.md), with its seeded defects.

Usage:
    python3 -I evaluation/gsd-ab/seed.py seed --task <id> --arm A|B|B0 --out <dir>
        [--gsd-core <wasm>] [--gsd-security <wasm>] [--state complete] [--unseeded]
    python3 -I evaluation/gsd-ab/seed.py present --task <id> --arm A|B|B0 --dir <dir>
    python3 -I evaluation/gsd-ab/seed.py schedule
    python3 -I evaluation/gsd-ab/seed.py --self-test

`seed` copies the fixture's tracked files without the paths the arm drops, writes the arm's `specforge.json` (B and
B0; the component paths of the `@gsd/*` extensions from the flags) and applies each seed's file edits in `tasks.json`
order, then makes the copy a git repository with one base commit whose subject names no plan (it holds the seeded
files, so the copy starts clean), then makes each seed's commits. `--state complete` then
applies the task's `completion` (what a correct run leaves; the tests only). `--unseeded` applies no seed: the control
copy the tests compare a seeded one with. Two seedings of one (task, arm) with the same flags have one HEAD.

`present` names the seeded defects still in a copy after a run (the run file's `defects_present`): a defect is present
while any of its edits holds (a text it inserted is still in its file, a text or file it deleted is still gone, a commit
it made is reachable from HEAD) and none of its `resolved_if` does. A defect seeded by several edits (arm A's S3 lies
in ROADMAP.md and STATE.md) is fixed only when every lie is. `schedule` prints the run order: each (task, seed) once, its arms in an order drawn
with a fixed RNG seed, no arm first twice in a row.

Standard library and the `git` program only (arm A's gsd-core needs git too), Python 3.11+.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks.json"
REPO = HERE.parent.parent
ARMS = ("A", "B", "B0")
SEEDS = 3
RNG_SEED = 20261102
IDENTITY = ("gsd-ab", "gsd-ab@example.com")
BASE_AT = "2026-10-28T08:00:00Z"


class Refused(ValueError):
    """An edit, task or arm the seeding cannot apply as written."""


def git(cwd: Path, *args: str, at: str = BASE_AT) -> str:
    """`git <args>` in `cwd` with a fixed identity and date, never the operator's configuration or hooks."""
    env = {
        **os.environ,
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_AUTHOR_DATE": at,
        "GIT_COMMITTER_DATE": at,
    }
    for name in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
        env.pop(name, None)
    config = [
        "-c", f"user.name={IDENTITY[0]}", "-c", f"user.email={IDENTITY[1]}",
        "-c", "commit.gpgsign=false", "-c", f"core.hooksPath={os.devnull}",
        "-c", f"core.excludesFile={os.devnull}",
    ]
    done = subprocess.run(
        ["git", *config, *args], cwd=cwd, env=env, capture_output=True, text=True, stdin=subprocess.DEVNULL
    )
    if done.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {done.stderr.strip()}")
    return done.stdout


def find(items: list[dict], key: str, what: str) -> dict:
    for item in items:
        if item["id"] == key:
            return item
    raise Refused(f"no such {what}: {key}")


def tracked(source: Path) -> list[str]:
    """The fixture's files git tracks or would track, relative to it."""
    listed = git(source, "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", ".")
    return sorted(path for path in listed.split("\0") if path and (source / path).is_file())


def dropped(path: str, drops: list[str]) -> bool:
    return any(path == drop or (drop.endswith("/") and path.startswith(drop)) for drop in drops)


def manifest(tasks: dict, fixture: dict, arm: dict, components: dict) -> dict:
    """The arm's `specforge.json` for the fixture: its extensions, the fixture's thread packs when the arm loads them,
    the fixture's own extensions, and the arm's settings. An `@gsd/*` extension is loaded from its component."""
    config = arm["config"]
    names = list(config["extensions"])
    if config["thread"]:
        names += fixture["thread"]
    names += fixture.get("extensions", [])
    extensions = []
    for name in names:
        if name.startswith("@gsd/"):
            flag = "--" + name[1:].replace("/", "-")
            if not components.get(name):
                raise Refused(f"arm {arm['id']} on {fixture['id']} loads {name}: pass {flag} <wasm>")
            extensions.append(f"{name}={Path(components[name]).resolve()}")
        else:
            extensions.append(name)
    return {
        "name": fixture["id"],
        "version": "0.1.0",
        "spec_root": "spec",
        "extensions": extensions,
        "settings": config["settings"],
    }


def apply(out: Path, edit: dict, label: str) -> None:
    """One seed edit in the copy `out`. Refuses, naming the edit, an `old` found other than once or a `write` over a
    file that exists."""
    (kind, body), = edit.items()
    if kind == "replace":
        path = out / body["file"]
        text = path.read_text() if path.is_file() else ""
        found = text.count(body["old"])
        if found != 1:
            raise Refused(f"{label}: `{body['old'][:60]}` is in {body['file']} {found} times, not once")
        path.write_text(text.replace(body["old"], body["new"], 1))
    elif kind == "write":
        path = out / body["file"]
        if path.exists():
            raise Refused(f"{label}: {body['file']} exists; a write makes a new file")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body["text"])
    elif kind == "delete":
        path = out / body["file"]
        if not path.is_file():
            raise Refused(f"{label}: {body['file']} is not there to delete")
        path.unlink()
    elif kind == "commit":
        for file, text in body["files"].items():
            (out / file).parent.mkdir(parents=True, exist_ok=True)
            (out / file).write_text(text)
            git(out, "add", "--", file, at=body["at"])
        message = body["subject"] + (f"\n\n{body['body']}" if body.get("body") else "")
        git(out, "commit", "-q", "--allow-empty", "-m", message, at=body["at"])
    else:
        raise Refused(f"{label}: unknown edit `{kind}`")


def edits_of(task: dict, defect: str, arm: str) -> list[dict]:
    """A defect's edits in an arm: B0 takes B's."""
    return task["seeds"][defect]["B" if arm == "B0" else arm]


def seed(
    tasks: dict, root: Path, task_id: str, arm_id: str, out: Path,
    components: dict | None = None, complete: bool = False, unseeded: bool = False,
) -> int:
    """Makes the copy at `out` (absent or empty). Answers the number of edits applied."""
    task = find(tasks["tasks"], task_id, "task")
    arm = find(tasks["arms"], arm_id, "arm")
    fixture = find(tasks["fixtures"], task["fixture"], "fixture")
    if fixture["status"] != "ready":
        raise Refused(f"fixture {fixture['id']} is not ready: {fixture['status']}")
    if out.exists() and any(out.iterdir()):
        raise Refused(f"{out} is not empty")
    source = root / fixture["path"]
    out.mkdir(parents=True, exist_ok=True)
    for path in tracked(source):
        if not dropped(path, arm["drop"]):
            (out / path).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source / path, out / path)
    if arm["config"] is not None:
        text = json.dumps(manifest(tasks, fixture, arm, components or {}), indent=2) + "\n"
        (out / "specforge.json").write_text(text)
    # The file edits go into the base commit and the commits come after it, so `git status` and `git diff` show the
    # agent no seed: the copy starts clean, as the fixture would.
    edits = [
        (f"{task_id} {defect} {arm_id} edit {index + 1}", edit)
        for defect in ([] if unseeded else task["seeded"])
        for index, edit in enumerate(edits_of(task, defect, arm_id))
    ]
    for label, edit in edits:
        if "commit" not in edit:
            apply(out, edit, label)
    git(out, "init", "-q", "-b", "main")
    git(out, "add", "-A")
    git(out, "commit", "-q", "-m", f"chore: the {fixture['id']} fixture")
    for label, edit in edits:
        if "commit" in edit:
            apply(out, edit, label)
    applied = len(edits)
    if complete:
        if arm_id == "A":
            raise Refused("--state complete is the B edits a correct run leaves; arm A has none")
        for index, edit in enumerate(task.get("completion", [])):
            apply(out, edit, f"{task_id} completion edit {index + 1}")
            applied += 1
    return applied


def present(tasks: dict, task_id: str, arm_id: str, copy: Path) -> list[str]:
    """The seeded defects of the task still in `copy`: those with an edit that still holds and no `resolved_if` that
    does."""
    task = find(tasks["tasks"], task_id, "task")
    find(tasks["arms"], arm_id, "arm")
    messages = set(git(copy, "log", "-z", "--format=%B", "HEAD").split("\0")) if (copy / ".git").exists() else set()
    messages = {message.strip() for message in messages}

    def holds(edit: dict) -> bool:
        (kind, body), = edit.items()
        path = copy / body.get("file", "")
        if kind == "replace":
            text = path.read_text() if path.is_file() else ""
            old, new = body["old"], body["new"]
            # A deletion holds while the old text is gone (its file too); an insertion that keeps the old text around
            # it holds while the new text is there; a rewrite holds while the new text is there and the old is not.
            return (not new or new in text) and (old in new or old not in text)
        if kind == "write":
            return path.is_file() and body["text"] in path.read_text()
        if kind == "delete":
            return not path.exists()
        message = body["subject"] + (f"\n\n{body['body']}" if body.get("body") else "")
        return message.strip() in messages

    def resolved(rule: dict) -> bool:
        path = copy / rule["file"]
        return path.is_file() and rule["contains"] in path.read_text()

    return [
        defect
        for defect in task["seeded"]
        if any(holds(edit) for edit in edits_of(task, defect, arm_id))
        and not any(resolved(rule) for rule in task["seeds"][defect].get("resolved_if", []))
    ]


def schedule(tasks: dict, seeds: int = SEEDS) -> list[tuple[str, int, list[str]]]:
    """Every (task, seed) once, with all arms, in an order drawn per (task, seed) by a fixed RNG; when a draw would
    put the previous first arm first again, it is rotated by one."""
    rng = random.Random(RNG_SEED)
    order, last = [], None
    for task in tasks["tasks"]:
        for number in range(1, seeds + 1):
            arms = list(ARMS)
            rng.shuffle(arms)
            if arms[0] == last:
                arms = arms[1:] + arms[:1]
            last = arms[0]
            order.append((task["id"], number, arms))
    return order


SYNTHETIC = {
    "arms": [
        {"id": "A", "config": None, "drop": ["spec/", "specforge-harness", "README.md"]},
        {"id": "B", "config": {"extensions": ["@gsd/core"], "thread": True, "settings": {}},
         "drop": [".planning/", "README.md"]},
        {"id": "B0", "config": {"extensions": ["@gsd/core"], "thread": False, "settings": {}},
         "drop": [".planning/", "README.md"]},
    ],
    "fixtures": [{"id": "fx", "path": "fx", "status": "ready", "thread": ["@code/core"]}],
    "tasks": [
        {
            "id": "t1", "fixture": "fx", "seeded": ["D1", "D2"],
            "seeds": {
                "D1": {
                    "A": [{"replace": {"file": ".planning/STATE.md", "old": "planned", "new": "complete"}}],
                    "B": [
                        {"replace": {"file": "spec/a.spec", "old": "status planned", "new": "status complete"}},
                        {"write": {"file": "src/new.js", "text": "export const x = 1;\n"}},
                    ],
                },
                "D2": {
                    "A": [{"commit": {"subject": "feat(01-01): x", "body": "Gsd-Task: 01-01/T9", "files": {},
                                      "at": "2026-10-28T09:00:00Z"}}],
                    "B": [{"commit": {"subject": "feat(01-01): x", "body": "Gsd-Task: 01-01/T9", "files": {},
                                      "at": "2026-10-28T09:00:00Z"}}],
                },
            },
        },
        {"id": "t2", "fixture": "fx", "seeded": ["D3"], "seeds": {"D3": {
            "A": [{"replace": {"file": "src/x.js", "old": "nothing", "new": "x"}}],
            "B": [{"replace": {"file": "src/x.js", "old": "same", "new": "x"}}],
        }}},
        {"id": "t3", "fixture": "fx", "seeded": ["D4"], "seeds": {"D4": {
            "A": [{"write": {"file": "src/x.js", "text": "x"}}], "B": [],
        }}},
        {"id": "t4", "fixture": "fx", "seeded": ["D5", "D6"], "seeds": {
            "D5": {"A": [{"replace": {"file": "test/t.js", "old": "drop\n", "new": ""}}], "B": []},
            "D6": {"A": [{"replace": {"file": "test/t.js", "old": "keep\n", "new": "keep\nmore\n"}}], "B": []},
        }},
    ],
}


def corpus(tasks: dict, root: Path, scratch: Path) -> int:
    """Seeds every task of `tasks` in every arm, seeded and unseeded: `present` names exactly the task's seeded defects
    on the seeded copy and none on the unseeded one. Answers the number of copies made."""
    components = {name: str(scratch / "component.wasm") for name in ("@gsd/core", "@gsd/security")}
    made = 0
    for task in tasks["tasks"]:
        for arm in ARMS:
            for unseeded in (False, True):
                copy = scratch / f"{task['id']}-{arm}-{'unseeded' if unseeded else 'seeded'}"
                seed(tasks, root, task["id"], arm, copy, components, unseeded=unseeded)
                got, want = present(tasks, task["id"], arm, copy), [] if unseeded else task["seeded"]
                assert got == want, f"{copy.name}: present {got}, want {want}"
                made += 1
    return made


def self_test() -> None:
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        fixture = root / "fx"
        for path, text in {
            ".planning/STATE.md": "status: planned\n", "spec/a.spec": "phase 01 {\n  status planned\n}\n",
            "specforge-harness": "# specforge-harness 1\n", "README.md": "the fixture\n",
            "src/x.js": "same\nsame\n", "test/t.js": "keep\ndrop\n",
        }.items():
            (fixture / path).parent.mkdir(parents=True, exist_ok=True)
            (fixture / path).write_text(text)
        git(fixture, "init", "-q", "-b", "main")
        components = {"@gsd/core": str(root / "gsd_core.wasm")}

        def files(copy: Path, under: str) -> int:
            return sum(1 for path in copy.rglob("*") if path.is_file() and path.relative_to(copy).as_posix()
                       .startswith(under))

        a, b, b0 = root / "a", root / "b", root / "b0"
        assert seed(SYNTHETIC, root, "t1", "A", a) == 2, "A applies its two edits"
        assert (files(a, "spec/"), files(a, ".planning/"), (a / "specforge.json").exists()) == (0, 1, False)
        assert seed(SYNTHETIC, root, "t1", "B", b, components) == 3, "B applies its three edits"
        assert (files(b, ".planning/"), files(b, "spec/"), files(b, "README.md")) == (0, 1, 0)
        wrote = json.loads((b / "specforge.json").read_text())["extensions"]
        assert [e.split("=")[0] for e in wrote] == ["@gsd/core", "@code/core"], wrote
        seed(SYNTHETIC, root, "t1", "B0", b0, components)
        assert [e.split("=")[0] for e in json.loads((b0 / "specforge.json").read_text())["extensions"]] == [
            "@gsd/core"]

        refusals = 0
        for task, arm in (("t2", "A"), ("t2", "B"), ("t3", "A")):
            try:
                seed(SYNTHETIC, root, task, arm, root / f"r-{task}-{arm}", components)
            except Refused as refused:
                print(f"refused: {refused}")
                refusals += 1
        assert refusals == 3, "an old found 0 times, one found twice and a write over a file: 3 refusals"
        for bad in (("t9", "A"), ("t1", "C")):
            try:
                seed(SYNTHETIC, root, *bad, root / "unknown")
            except Refused:
                refusals += 1
        assert refusals == 5, "an unknown task and an unknown arm are refused"
        try:
            seed(SYNTHETIC, root, "t1", "B", root / "no-wasm")
        except Refused as refused:
            assert "--gsd-core" in str(refused), refused
            refusals += 1
        assert refusals == 6, "a B copy without the component is refused, naming the flag"

        again = root / "b-again"
        seed(SYNTHETIC, root, "t1", "B", again, components)
        heads = {git(copy, "rev-parse", "HEAD").strip() for copy in (b, again)}
        assert len(heads) == 1, f"two seedings, one HEAD: {heads}"
        assert git(b, "status", "--porcelain") == "", "a seeded copy starts clean: no seed shows in git status"

        assert present(SYNTHETIC, "t1", "B", b) == ["D1", "D2"]
        (b / "src/new.js").unlink()
        assert present(SYNTHETIC, "t1", "B", b) == ["D1", "D2"], "D1 stays while one of its two edits holds"
        (b / "spec/a.spec").write_text("phase 01 {\n  status planned\n}\n")
        assert present(SYNTHETIC, "t1", "B", b) == ["D2"], "both of D1's edits undone: exactly D1 is gone"
        assert present(SYNTHETIC, "t1", "A", a) == ["D1", "D2"]
        git(a, "reset", "-q", "--hard", "HEAD~1")
        assert present(SYNTHETIC, "t1", "A", a) == ["D1"], "the seeded commit unreachable: D2 is gone"
        (a / ".planning/STATE.md").write_text("status: planned\n")
        assert present(SYNTHETIC, "t1", "A", a) == [], "and STATE.md restored: D1 too"

        # A deletion (D5) holds while its text is gone, the file too; an insertion (D6) while its new text is there.
        t4 = root / "t4"
        seed(SYNTHETIC, root, "t4", "A", t4)
        assert present(SYNTHETIC, "t4", "A", t4) == ["D5", "D6"], "a deletion and an insertion, seeded"
        (t4 / "test/t.js").write_text("keep\ndrop\n")
        assert present(SYNTHETIC, "t4", "A", t4) == [], "both restored"
        (t4 / "test/t.js").unlink()
        assert present(SYNTHETIC, "t4", "A", t4) == ["D5"], "the file gone: the deleted text is still gone"

        order = schedule(SYNTHETIC)
        assert len(order) == len(SYNTHETIC["tasks"]) * SEEDS
        assert len({(task, number) for task, number, _ in order}) == len(order), "each (task, seed) once"
        assert all(sorted(arms) == sorted(ARMS) for _, _, arms in order), "every arm in every (task, seed)"
        firsts = [arms[0] for _, _, arms in order]
        assert all(x != y for x, y in zip(firsts, firsts[1:])), f"no arm first twice in a row: {firsts}"

        tasks = json.loads(TASKS.read_text())
        made = corpus(tasks, REPO, root / "corpus")
        assert made == len(tasks["tasks"]) * len(ARMS) * 2
        print(f"corpus: {made} copies, each seeded one naming its seeds and each unseeded one none")
    print("self-test: ok")


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    verbs = parser.add_subparsers(dest="verb", required=True)
    made = verbs.add_parser("seed", help="make an arm's copy of a task's fixture")
    made.add_argument("--task", required=True)
    made.add_argument("--arm", required=True)
    made.add_argument("--out", required=True, type=Path)
    made.add_argument("--gsd-core", help="the @gsd/core component (B, B0)")
    made.add_argument("--gsd-security", help="the @gsd/security component (B, B0 on a fixture that loads it)")
    made.add_argument("--state", choices=["complete"], help="apply the task's completion edits too (tests only)")
    made.add_argument("--unseeded", action="store_true", help="the control copy: no seed applied")
    held = verbs.add_parser("present", help="the seeded defects still in a copy")
    held.add_argument("--task", required=True)
    held.add_argument("--arm", required=True)
    held.add_argument("--dir", required=True, type=Path)
    verbs.add_parser("schedule", help="the run order")
    args = parser.parse_args()
    tasks = json.loads(TASKS.read_text())
    try:
        if args.verb == "seed":
            components = {"@gsd/core": args.gsd_core, "@gsd/security": args.gsd_security}
            applied = seed(tasks, REPO, args.task, args.arm, args.out, components, args.state == "complete",
                           args.unseeded)
            head = git(args.out, "rev-parse", "HEAD").strip()
            print(json.dumps({"task": args.task, "arm": args.arm, "edits": applied, "head": head}))
        elif args.verb == "present":
            print(json.dumps(present(tasks, args.task, args.arm, args.dir)))
        else:
            for task, number, arms in schedule(tasks):
                print(f"{task} {number} {' '.join(arms)}")
    except Refused as refused:
        print(f"seed.py: {refused}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()

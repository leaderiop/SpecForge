#!/usr/bin/env python3
"""Make each arm's copy of a GSD A/B task's fixture (evaluation/gsd-ab/README.md), with its seeded defects.

Usage:
    python3 -I evaluation/gsd-ab/seed.py seed --task <id> --arm A|B|B0 --out <dir>
        [--gsd-core <wasm>] [--gsd-security <wasm>] [--state complete] [--unseeded] [--worktree]
    python3 -I evaluation/gsd-ab/seed.py present --task <id> --arm A|B|B0 --dir <dir>
    python3 -I evaluation/gsd-ab/seed.py rework --arm A|B|B0 --dir <dir>
    python3 -I evaluation/gsd-ab/seed.py prompt --task <id> --arm A|B|B0
    python3 -I evaluation/gsd-ab/seed.py schedule
    python3 -I evaluation/gsd-ab/seed.py --self-test

`seed` copies the fixture as committed (`git archive` of the tree `tasks.json` pins for it, which must be the fixture's
tree at HEAD) without the paths the arm drops; it refuses while `evaluation/gsd-ab/` or the fixture has uncommitted
changes, so a study copy is always the frozen protocol's. `--worktree` copies the working tree instead (the tracked and
untracked files git would add), for the tests only: its output's `commit` ends in `+worktree`. Then it writes the
arm's `specforge.json` (B and B0; the component paths of the `@gsd/*` extensions from the flags), applies each seed's
file edits in `tasks.json` order, makes the copy a git repository with one base commit whose subject names no plan (it
holds the seeded files, so the copy starts clean), then makes each seed's commits. `--state complete` then applies the
task's `completion` (what a correct run leaves; the tests only). `--unseeded` applies no seed: the control copy the
tests compare a seeded one with. Two seedings of one (task, arm) with the same flags have one HEAD.

`present` names the seeded defects still in a copy after a run (the run file's `defects_present`): a defect is present
while any of its edits holds (a text it inserted is still in its file, a text or file it deleted is still gone, a commit
it made is reachable from HEAD) and none of its `resolved_if` does. A defect seeded by several edits (arm A's S3 lies
in ROADMAP.md and STATE.md) is fixed only when every lie is. `rework` counts the run's commits (those after the last
seeded one) that come after its first commit writing verification state: in A a `.planning/phases/*/*-VERIFICATION.md`,
in B and B0 a `verdict:` line of `specforge-harness`; 0 when none does (the run file's `rework_commits`). `prompt`
prints the run's one message: `prompts/<arm>.md` with the task's prompt and its workflow's `RESULT:` values (from
`tasks.json#results`) in place of `{task.prompt}` and `{task.results}`. `schedule` prints the run order: each (task,
seed) once, its arms in an order drawn with a fixed RNG seed, no arm first twice in a row.

Standard library and the `git` program only (arm A's gsd-core needs git too), Python 3.11+.
"""

from __future__ import annotations

import argparse
import fnmatch
import io
import json
import os
import random
import shutil
import subprocess
import sys
import tarfile
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


def git(cwd: Path, *args: str, at: str = BASE_AT, raw: bool = False) -> str | bytes:
    """`git <args>` in `cwd` with a fixed identity and date, never the operator's configuration or hooks; its output as
    text, or as bytes when `raw`."""
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
    done = subprocess.run(["git", *config, *args], cwd=cwd, env=env, capture_output=True, stdin=subprocess.DEVNULL)
    if done.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {done.stderr.decode(errors='replace').strip()}")
    return done.stdout if raw else done.stdout.decode()


def find(items: list[dict], key: str, what: str) -> dict:
    for item in items:
        if item["id"] == key:
            return item
    raise Refused(f"no such {what}: {key}")


def tracked(source: Path) -> dict[str, bytes]:
    """The fixture's files in the working tree that git tracks or would track, by path relative to it."""
    listed = git(source, "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", ".")
    paths = sorted(path for path in listed.split("\0") if path and (source / path).is_file())
    return {path: (source / path).read_bytes() for path in paths}


def committed(root: Path, fixture: dict) -> dict[str, bytes]:
    """The fixture's files as committed: the tree `tasks.json` pins for it, which must be its tree at HEAD, while
    neither it nor `evaluation/gsd-ab/` has uncommitted changes."""
    paths = [fixture["path"]] + ([HERE.relative_to(root).as_posix()] if HERE.is_relative_to(root) else [])
    dirty = git(root, "status", "--porcelain", "--untracked-files=no", "--", *paths)
    if dirty.strip():
        raise Refused(
            f"uncommitted changes under {' and '.join(paths)}: a study copy is made from a commit (commit them; "
            "--worktree is for the tests)"
        )
    tree = git(root, "rev-parse", f"HEAD:{fixture['path']}").strip()
    if fixture.get("tree", tree) != tree:
        raise Refused(
            f"fixture {fixture['id']} is tree {tree} at HEAD, but tasks.json pins {fixture['tree']}: a changed fixture "
            "is a new protocol version"
        )
    with tarfile.open(fileobj=io.BytesIO(git(root, "archive", "--format=tar", tree, raw=True))) as archive:
        return {
            member.name: archive.extractfile(member).read()
            for member in archive.getmembers()
            if member.isfile()
        }


def protocol_commit(root: Path, worktree: bool) -> str:
    """The commit a copy's protocol comes from: HEAD, marked `+worktree` when the copy is the working tree's."""
    return git(root, "rev-parse", "HEAD").strip() + ("+worktree" if worktree else "")


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


def inside(out: Path, file: str, label: str) -> Path:
    """`file` in the copy `out`; refused, naming the edit, when it resolves outside it."""
    path = (out / file).resolve()
    if not path.is_relative_to(out.resolve()):
        raise Refused(f"{label}: {file} is outside the copy")
    return path


def apply(out: Path, edit: dict, label: str) -> None:
    """One seed edit in the copy `out`. Refuses, naming the edit, a path outside the copy, an `old` found other than
    once or a `write` over a file that exists."""
    (kind, body), = edit.items()
    if kind == "replace":
        path = inside(out, body["file"], label)
        text = path.read_text() if path.is_file() else ""
        found = text.count(body["old"])
        if found != 1:
            raise Refused(f"{label}: `{body['old'][:60]}` is in {body['file']} {found} times, not once")
        path.write_text(text.replace(body["old"], body["new"], 1))
    elif kind == "write":
        path = inside(out, body["file"], label)
        if path.exists():
            raise Refused(f"{label}: {body['file']} exists; a write makes a new file")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body["text"])
    elif kind == "delete":
        path = inside(out, body["file"], label)
        if not path.is_file():
            raise Refused(f"{label}: {body['file']} is not there to delete")
        path.unlink()
    elif kind == "commit":
        for file, text in body["files"].items():
            path = inside(out, file, label)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
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
    components: dict | None = None, complete: bool = False, unseeded: bool = False, worktree: bool = False,
) -> int:
    """Makes the copy at `out` (absent or empty) from the fixture as committed, or from the working tree with
    `worktree`. Answers the number of edits applied. Everything that can be refused without the copy is checked
    first, and the copy is built beside `out` and renamed to it only once it is whole: a refusal leaves nothing."""
    task = find(tasks["tasks"], task_id, "task")
    arm = find(tasks["arms"], arm_id, "arm")
    fixture = find(tasks["fixtures"], task["fixture"], "fixture")
    if fixture["status"] != "ready":
        raise Refused(f"fixture {fixture['id']} is not ready: {fixture['status']}")
    if complete and arm_id == "A":
        raise Refused("--state complete is the B edits a correct run leaves; arm A has none")
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise Refused(f"{out} is not empty")
    config = None if arm["config"] is None else manifest(tasks, fixture, arm, components or {})
    files = tracked(root / fixture["path"]) if worktree else committed(root, fixture)
    out.parent.mkdir(parents=True, exist_ok=True)
    building = Path(tempfile.mkdtemp(prefix=f".{out.name}.", dir=out.parent))
    try:
        applied = build(task, arm, fixture, building, files, config, complete, unseeded)
    except BaseException:
        shutil.rmtree(building, ignore_errors=True)
        raise
    if out.exists():
        out.rmdir()
    building.rename(out)
    return applied


def build(
    task: dict, arm: dict, fixture: dict, out: Path, files: dict[str, bytes], config: dict | None,
    complete: bool, unseeded: bool,
) -> int:
    """Writes the copy into the empty directory `out`: the fixture's `files` but the arm's drops, its `config` as
    `specforge.json`, the seeds' edits and commits, and the completion with `complete`. Answers the edits applied."""
    task_id, arm_id = task["id"], arm["id"]
    for path, data in files.items():
        if not dropped(path, arm["drop"]):
            (out / path).parent.mkdir(parents=True, exist_ok=True)
            (out / path).write_bytes(data)
    if config is not None:
        (out / "specforge.json").write_text(json.dumps(config, indent=2) + "\n")
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


def writes_verification(copy: Path, arm_id: str, commit: str) -> bool:
    """Whether `commit` writes verification state: in A a phase's VERIFICATION.md, in B and B0 a verdict line of the
    harness ledger."""
    if arm_id == "A":
        changed = git(copy, "diff-tree", "--root", "--no-commit-id", "--name-only", "-r", commit).splitlines()
        return any(fnmatch.fnmatch(path, ".planning/phases/*/*-VERIFICATION.md") for path in changed)
    diff = git(copy, "show", "--format=", "--unified=0", commit, "--", "specforge-harness")
    return any(line.startswith("+") and " verdict:" in line for line in diff.splitlines())


def rework(arm_id: str, copy: Path) -> int:
    """The run's commits after its first one that writes verification state, or 0 when none does. The run's commits
    are those after the last commit the seeding made (its author is the seeding's identity)."""
    if arm_id not in ARMS:
        raise Refused(f"no such arm: {arm_id}")
    base = git(copy, "log", "-1", "--format=%H", f"--author=<{IDENTITY[1]}>", "HEAD").strip()
    if not base:
        raise Refused(f"{copy}: no commit of the seeding, so no run to count")
    commits = git(copy, "rev-list", "--reverse", f"{base}..HEAD").split()
    for index, commit in enumerate(commits):
        if writes_verification(copy, arm_id, commit):
            return len(commits) - index - 1
    return 0


def prompt(tasks: dict, task_id: str, arm_id: str, prompts: Path = HERE / "prompts") -> str:
    """The run's one message: `prompts/<arm>.md` with the task's prompt and its workflow's RESULT values filled in."""
    task = find(tasks["tasks"], task_id, "task")
    find(tasks["arms"], arm_id, "arm")
    text = (prompts / f"{arm_id}.md").read_text()
    for holder in ("{task.prompt}", "{task.results}"):
        if text.count(holder) != 1:
            raise Refused(f"prompts/{arm_id}.md holds {holder} {text.count(holder)} times, not once")
    results = " or ".join(
        f"`RESULT: {result['value']}` ({result['means']})" for result in tasks["results"][task["workflow"]]
    )
    return text.replace("{task.prompt}", task["prompt"]).replace("{task.results}", f"{results}.")


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
                seed(tasks, root, task["id"], arm, copy, components, unseeded=unseeded, worktree=True)
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
        git(root, "init", "-q", "-b", "main")
        git(root, "add", "fx")
        git(root, "commit", "-q", "-m", "the synthetic fixture")
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
        # A refusal leaves nothing behind: not the copy, not the directory it was built in.
        escapes = {**SYNTHETIC, "tasks": [{"id": "t5", "fixture": "fx", "seeded": ["D7"], "seeds": {"D7": {
            "A": [{"write": {"file": "../escaped.js", "text": "x"}}], "B": []}}}]}
        for args, why in (((SYNTHETIC, root, "t1", "A", root / "a-complete"), "arm A has none"),
                          ((escapes, root, "t5", "A", root / "escape"), "outside the copy")):
            try:
                seed(*args, complete=args[2] == "t1")
            except Refused as refused:
                assert why in str(refused), refused
                refusals += 1
        assert not (root / "escaped.js").exists(), "an edit never writes outside the copy"
        refused_names = (".", "no-wasm", "a-complete", "escape")
        left = sorted(path.name for path in root.iterdir() if path.name.startswith(refused_names))
        assert left == [".git"], f"a refused seeding leaves nothing: {left}"

        # A study copy comes from the commit: a fixture with an uncommitted change is refused, and so is one whose tree
        # is not the one tasks.json pins. `worktree` copies the working tree as it is.
        (fixture / "src/x.js").write_text("same\nsame\nchanged\n")
        try:
            seed(SYNTHETIC, root, "t1", "A", root / "dirty")
        except Refused as refused:
            assert "uncommitted" in str(refused), refused
            refusals += 1
        seed(SYNTHETIC, root, "t1", "A", root / "dirty-worktree", worktree=True)
        assert (root / "dirty-worktree/src/x.js").read_text().endswith("changed\n"), "the working tree, as it is"
        git(root, "checkout", "-q", "--", "fx/src/x.js")
        moved = {**SYNTHETIC, "fixtures": [{**SYNTHETIC["fixtures"][0], "tree": "0" * 40}]}
        try:
            seed(moved, root, "t1", "A", root / "moved")
        except Refused as refused:
            assert "pins" in str(refused), refused
            refusals += 1
        assert refusals == 10, "a dirty fixture and a fixture off its pinned tree are refused"

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

        # The run's commits come after the seeded ones; rework counts those after the first verification write.
        def agent_commit(copy: Path, path: str, text: str) -> None:
            (copy / path).parent.mkdir(parents=True, exist_ok=True)
            (copy / path).write_text(text)
            git(copy, "add", "--", path)
            git(copy, "-c", "user.email=agent@example.com", "commit", "-q", "-m", f"agent: {path}")

        assert (rework("A", a), rework("B", b)) == (0, 0), "no run commit, no rework"
        agent_commit(a, "src/a.js", "1\n")
        agent_commit(a, ".planning/phases/01-core/01-VERIFICATION.md", "status: gaps_found\n")
        agent_commit(a, "src/a.js", "2\n")
        agent_commit(b, "specforge-harness", "# specforge-harness 1\nv1.0/01/01-01/TR1 verdict:gsd/E501 ab\n")
        for n in range(2):
            agent_commit(b, "src/b.js", f"{n}\n")
        assert (rework("A", a), rework("B", b)) == (1, 2), "the commits after the first verification write"

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
        for task in tasks["tasks"]:
            for arm in ARMS:
                message = prompt(tasks, task["id"], arm)
                values = [result["value"] for result in tasks["results"][task["workflow"]]]
                assert "{task." not in message and task["prompt"] in message, (task["id"], arm)
                assert all(f"`RESULT: {value}`" in message.splitlines()[-1] for value in values), (task["id"], arm)
        made = corpus(tasks, REPO, root / "corpus")
        assert made == len(tasks["tasks"]) * len(ARMS) * 2
        print(f"corpus: {made} copies, each seeded one naming its seeds and each unseeded one none")
    print("self-test: ok")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--self-test", action="store_true", help="run the embedded checks, and the corpus pass")
    verbs = parser.add_subparsers(dest="verb")
    made = verbs.add_parser("seed", help="make an arm's copy of a task's fixture")
    made.add_argument("--task", required=True)
    made.add_argument("--arm", required=True)
    made.add_argument("--out", required=True, type=Path)
    made.add_argument("--gsd-core", help="the @gsd/core component (B, B0)")
    made.add_argument("--gsd-security", help="the @gsd/security component (B, B0 on a fixture that loads it)")
    made.add_argument("--state", choices=["complete"], help="apply the task's completion edits too (tests only)")
    made.add_argument("--unseeded", action="store_true", help="the control copy: no seed applied")
    made.add_argument("--worktree", action="store_true", help="copy the working tree, not the commit (tests only)")
    held = verbs.add_parser("present", help="the seeded defects still in a copy")
    held.add_argument("--task", required=True)
    held.add_argument("--arm", required=True)
    held.add_argument("--dir", required=True, type=Path)
    counted = verbs.add_parser("rework", help="the run's rework commits")
    counted.add_argument("--arm", required=True)
    counted.add_argument("--dir", required=True, type=Path)
    said = verbs.add_parser("prompt", help="the run's one message")
    said.add_argument("--task", required=True)
    said.add_argument("--arm", required=True)
    verbs.add_parser("schedule", help="the run order")
    args = parser.parse_args()
    if args.self_test:
        if args.verb:
            parser.error("--self-test takes no verb")
        self_test()
        return
    if not args.verb:
        parser.error("a verb (seed, present, rework, prompt, schedule) or --self-test")
    tasks = json.loads(TASKS.read_text())
    try:
        if args.verb == "seed":
            components = {"@gsd/core": args.gsd_core, "@gsd/security": args.gsd_security}
            applied = seed(tasks, REPO, args.task, args.arm, args.out, components, args.state == "complete",
                           args.unseeded, args.worktree)
            head = git(args.out, "rev-parse", "HEAD").strip()
            commit = protocol_commit(REPO, args.worktree)
            print(json.dumps({"task": args.task, "arm": args.arm, "edits": applied, "head": head, "commit": commit}))
        elif args.verb == "present":
            print(json.dumps(present(tasks, args.task, args.arm, args.dir)))
        elif args.verb == "rework":
            print(rework(args.arm, args.dir))
        elif args.verb == "prompt":
            print(prompt(tasks, args.task, args.arm), end="")
        else:
            for task, number, arms in schedule(tasks):
                print(f"{task} {number} {' '.join(arms)}")
    except Refused as refused:
        print(f"seed.py: {refused}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()

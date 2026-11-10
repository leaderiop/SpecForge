# gsd-git reads the host's git port, and tasks and commits name their code items

**Status:** accepted (2026-11-10, R2 of the Beyond Parity build plan). **Amends**
[ADR 0331](0331-the-gsd-requirement-thread.md) D3 (the trailer is read from a typed log, not an argv), D4 (gsd-git is
detected by `.git`) and D8 (the route gains an `item` step), and
[ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D5 (a history reference is never unheld). Builds on
[ADR 0287](0287-who-writes-a-field-is-declared.md) (harness and immutable fields),
[ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) (nothing secret reaches a guest),
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D4 (the `specforge:host` imports),
[ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md) (`Fact.at`) and
[ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md) (item ids). Protocol 2.0.0 (unreleased)
gains the guest `git` interface's `log` and `diff` and `UnheldReference.history`; `@gsd/core` gains gsd/W522.

## Context

- gsd-git ran `git log --no-renames --numstat --summary` with a hand-written `--format` string, parsed its output with
  its own record parser and C-quote unquoter, and needed a `git` program on `PATH`: without one a collect recorded no
  commit. It was detected by a branch ref, so a linked worktree (`.git` is a file), where GSD's executor agents work,
  never ran it.
- The host already reads the repository in process through gix (`specforge git log|diff`), but a guest could not ask
  for a log or a diff, and a diff had no typed line ranges, could not start from a root commit, and followed renames
  as the repository's configuration said.
- The requirement thread ended at the commit: nothing said which code a task targets, or which code a commit changed.
  `@code/core` mints item facts (`rust:…`, `js:…`) with their place (`Fact.at`), but no gsd field cited them.

## Decisions

- **D1. `commit.items_changed` is the commit's diff met with the stored item facts.** gsd-git asks the commit's
  first-parent diff with typed hunks (new side, no context), carries each line the commit wrote forward to the
  worktree with one more diff (`base` the commit, `head` the worktree), and names the item facts whose `at` span holds
  a carried line, sorted and distinct. A deletion is the gap between two lines: an item holding both neighbours is
  changed by it. `@gsd/core` mints no id and parses no source: the language packs stay the one id authority. In the
  normal flow (a collect right after the executor's commit) the forward diff is empty. Its limit: on a first collect
  of an old history, an item whose every line was rewritten since is not named; the item that holds the commit's
  lines now is.
- **D2. A history reference.** A reference a field declared `writer: harness` and `immutable` holds records what held
  when the harness wrote it. The registry marks such an unheld reference `history` (`UnheldReference.history`), and
  `@code/core`'s `refs` pass reports none of them (no `code/E101`, no `code/W102`). A held history reference is still
  an edge. The kernel knows no kind and no field by name: the flag comes from the field's own descriptor.
- **D3. The `item` step is required.** A route step has no optional mode (ADR 0330 D6): a task that names no held item
  leaves the thread incomplete, which `--partial` shows. Without `@code/core` the step is skipped (I210), as `test`.
- **D4. gsd-git is detected by `.git`**, a directory or a linked worktree's file. The port answers an unborn repository
  with no commit, so gsd-git records nothing there and the collect goes on.
- **D5. gsd-git asks for no renames.** The port's diff gains `renames`: `false` turns rewrite tracking off, so a rename
  is a deletion and an addition as `git log --no-renames` listed it, and `commit.files` (read by gsd/W520) holds both
  paths. A side over the port's 16 MiB blob cap counts no lines.
- **D6. Backfill once.** A commit recorded before any item facts existed gets `items_changed` on the first collect that
  serves item facts (an immutable value unset at the baseline may be set once, ADR 0287); never rewritten afterwards.
- **D7. gsd/W522, an item anchored only by plans that reach no requirement.** For each item a `task.items` or a
  `commit.items_changed` names, the anchors through quick plans dropped: when some remain and none of their plans
  reaches a requirement (a `plan.requirements` entry that resolves, or a truth of the plan whose `requirement`
  resolves), a warning on the first anchoring task (else commit). It names the orphaned item, which gsd/E302 cannot.
- **D8. Commits stay entities.** The ports round deferred `@git/core` and commit facts until gix reads were in place
  and a shallow clone was checked. This decision keeps commits as the authored-file entities gsd-git writes into the
  plan's file (ADR 0289 D4), so gsd/E515 never needs history in CI; on a shallow clone the log ends at the boundary and
  recorded commits stay in their plan files. `@git/core` stays deferred.

### The guest `git` interface

`log(log-query)` and `diff(diff-query)` join `locate`, `commit`, `merge-base`, `tree`, `sizes`, `blobs`, `branch` and
`status`. The host decides the repository (the project's own, a linked worktree's included); a guest names commits by
id and paths from the repository's top, never a place. No remote, URL, credential or environment variable reaches it.
A log with no `max` lists every commit up to the call's answer cap and says `truncated`; a pickaxe over a withheld path
is refused. A diff of a withheld file keeps its status and counts, never its patch or hunks, and says `withheld`; the
patches past the answer cap are dropped and the answer says `truncated`. A signature carries its epoch seconds.

## Rejected

- **Parsing each changed file at the commit and minting ids in `@gsd/core`** (a revision-capable syntax import): a
  second id minter per language in an extension that depends on the base only.
- **The host running the source collectors at a commit** (a `facts::at(revision)` import): the most historical answer,
  but every source pack would accept a revision and a pass would call across extensions.
- **A string list for `items_changed`**: no edge, so anchors could not follow it.
- **Owner state for a commit**: a commit is never pending, so its references would be E101 once the item moves.
- **An optional route step**: a kernel change outside this decision.
- **A positional `diff(a, b, patch)` import**: a root commit needs no base, the forward mapping needs the worktree, and
  gsd-git needs `renames: false` and hunks; one record holds them all.

## Consequences

- `specforge collect` records commits with no `git` program, no consent and no report; `--no-run` records them too.
- `commit.files` and `commit.created` are project-relative (equal to before when the project is the repository's top);
  a path outside the project is left out, its lines still counted.
- `specforge trace <requirement>` on a gsd project loading `@code/core` prints an `item` step and is incomplete until a
  task names a held item.
- The thread of the taskboard twin prints as a tree: a task with two items has two matches.

## What would reopen it

- A history worth naming the items a commit removed (the old side of its diff).
- `@git/core` and commit facts.
- An optional route step.

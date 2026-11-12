<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/complete-milestone/detail/elaboration.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# complete-milestone.md — deferred elaboration

Read in full when `workflow.compact_content` is `false` (the default) — see
`gsd-core/references/compact-content-gate.md` for the check and resolution rule this
spine defers to. Each `§` below is the full text the spine condenses at the point it
names.

## § 1 — pre_close_artifact_audit: the [A] Acknowledge branch

If user chooses [A] (Acknowledge): the open items are `specforge check` findings, and an agent cannot suppress them.
1. Ask the person to accept the latest milestone audit's tech debt: they write its `debt_accepted` (`by`, `at`) by hand or from an interactive `specforge set`. The host refuses that field in every plan and MCP write, so the agent never writes it.
2. Wait for their answer.
3. Re-run `specforge check --format json`: gsd/E106 passes once the milestone's latest audit is `passed`, or `tech_debt` with a `debt_accepted`; that is all [A] clears.
4. Proceed with the close (`specforge gsd close-milestone`) only when none of its gate codes is reported (gsd/E106, gsd/E511, gsd/E515 and gsd/E516); an open UAT or verification item (gsd/E511, gsd/E515, gsd/E516) still blocks: stop, report it and resolve it through [R]. The audit's `tech_debt` entries are the disclosure record: no other file is written.











Accepting is a person's act on one audit: a later audit of the milestone carries no `debt_accepted` until a person writes one again.

If output shows all clear (no open items): proceed with the close.

(The SECURITY note on the check's output and on `debt_accepted` is stated in the spine, not repeated here.)

## § 2 — handle_branches

Check branching strategy and offer merge options.

Use `init milestone-op` for context, or load config directly:

```bash

: "${PHASE_ID:?PHASE_ID is not set: the phase being completed}"; INIT=$(specforge query "$PHASE_ID" --depth 1 --format json) || exit 1

: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; INIT_CM=$(specforge query "$MILESTONE" --depth 1 --format json) || exit 1

```

The strategy is `git.branching_strategy` and each branch the computed `branch` of its phase or milestone. `commit_docs` and `git.create_tag` are settings: `specforge settings --format json --extension @gsd/core`; no init bundle prints a section manifest, so it is null and the `git_tag` step below is read.

`BASE_BRANCH` is already resolved by the spine at this point (via the shared `git.base-branch` resolver) — the branch options below use it as-is.

**If "none":** Skip to git_tag.

**For "phase" strategy:**

```bash
BRANCHING_STRATEGY="{{setting:git.branching_strategy|none}}"   # git.branching_strategy (the settings)
PHASE_BRANCHES=$(specforge gsd milestone-branches "${MILESTONE:?MILESTONE is not set: the milestone being completed}" --pick '.branches[]') || exit 1
```

**For "milestone" strategy:**

```bash
BRANCHING_STRATEGY="{{setting:git.branching_strategy|none}}"   # git.branching_strategy (the settings)
MILESTONE_BRANCH=$(specforge gsd milestone-branches "${MILESTONE:?MILESTONE is not set: the milestone being completed}" --pick '.branches[]') || exit 1
```

**If no branches found:** Skip to git_tag.

**If branches exist:**

```
## Git Branches Detected

Branching strategy: {phase/milestone}
Branches: {list}

Options:
1. **Merge to main** — Merge branch(es) to main
2. **Delete without merging** — Already merged or not needed
3. **Keep branches** — Leave for manual handling
```

AskUserQuestion with options: Squash merge (Recommended), Merge with history, Delete without merging, Keep branches.

**Squash merge:**

```bash
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"
BRANCHES=$(specforge gsd milestone-branches "$MILESTONE" --pick '.branches[]') || exit 1
BASE_BRANCH=$(specforge git config init.defaultBranch --pick .value 2>/dev/null); BASE_BRANCH=${BASE_BRANCH:-main}
COMMIT_DOCS="{{setting:commit_docs|true}}"   # commit_docs (the settings)
CURRENT_BRANCH=$(specforge git head --pick .branch) || exit 1
specforge git switch "$BASE_BRANCH" || exit 1

while IFS= read -r branch; do
  [ -n "$branch" ] || continue
  specforge git merge --squash "$branch" || exit 1
  # Strip spec/ from staging if commit_docs is false
  if [ "$COMMIT_DOCS" = "false" ]; then
    specforge git restore spec/ --staged --pick .restored 2>/dev/null || true
  fi
  specforge git commit -m "feat: $branch for v[X.Y]" || exit 1
done <<<"$BRANCHES"


specforge git switch "$CURRENT_BRANCH"
```

**Merge with history:**

```bash
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"
BRANCHES=$(specforge gsd milestone-branches "$MILESTONE" --pick '.branches[]') || exit 1
BASE_BRANCH=$(specforge git config init.defaultBranch --pick .value 2>/dev/null); BASE_BRANCH=${BASE_BRANCH:-main}
COMMIT_DOCS="{{setting:commit_docs|true}}"   # commit_docs (the settings)
CURRENT_BRANCH=$(specforge git head --pick .branch) || exit 1
specforge git switch "$BASE_BRANCH" || exit 1

while IFS= read -r branch; do
  [ -n "$branch" ] || continue
  specforge git merge --no-ff --no-commit "$branch" || exit 1
  # Strip spec/ from staging if commit_docs is false
  if [ "$COMMIT_DOCS" = "false" ]; then
    specforge git restore spec/ --staged --pick .restored 2>/dev/null || true
  fi
  specforge git commit -m "Merge branch '$branch' for v[X.Y]" || exit 1
done <<<"$BRANCHES"


specforge git switch "$CURRENT_BRANCH"
```

**Delete without merging:**

```bash
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"
BRANCHES=$(specforge gsd milestone-branches "$MILESTONE" --pick '.branches[]') || exit 1
while IFS= read -r branch; do
  [ -n "$branch" ] || continue
  specforge git branch delete "$branch" --pick .name 2>/dev/null || specforge git branch delete "$branch" --force || exit 1
done <<<"$BRANCHES"

```

**Keep branches:** Report "Branches preserved for manual handling"

<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/pr-branch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Create a clean branch for pull requests by filtering spec paths out of the
cherry-picked history. Two modes, selected by the `planning.pr_strict` config key:

- **default** (`planning.pr_strict: false`) — the PR branch contains code changes and
  structural planning state. Reviewers don't see GSD transient artifacts (PLAN.md,
  SUMMARY.md, CONTEXT.md, RESEARCH.md, etc.), but milestone archives, STATE.md,
  ROADMAP.md, and PROJECT.md changes are preserved.
- **strict** (`planning.pr_strict: true`) — *every* spec path is filtered out,
  structural files included. This is what makes `planning.commit_docs: true` safe for a
  project that versions its planning tree locally but publishes none of it: planning state
  keeps real git history (so `/gsd:undo` and revert paths have something to restore) and
  executor worktrees still find their PLAN.md, while the public PR carries nothing from
  the spec.

Uses git cherry-pick with path filtering to rebuild a clean history.
</purpose>

<process>

<step name="detect_state">
The target branch is the `--target` value. If none is supplied, detect the
default branch via the single resolver (#1146).

```bash
# the commands below are specforge commands: there is no launcher to define
CURRENT_BRANCH=$(specforge git head --pick .branch)
TARGET="{{arg:target|}}"; [ -n "$TARGET" ] || TARGET="{{setting:git.base_branch|}}"; [ -n "$TARGET" ] || TARGET=$(specforge git remotes --pick '.remotes[name=origin].head' 2>/dev/null) || true; [ -n "$TARGET" ] || for b in main master; do [ "$(specforge git rev "$b" --pick '.revs[0].exists')" = true ] && TARGET=$b && break; done
```

Check preconditions:
- Must be on a feature branch (not main/master)
- Must have commits ahead of target
- Working tree must be clean

```bash
AHEAD=$(specforge git log "$TARGET".."$CURRENT_BRANCH" --pick .count 2>/dev/null)
if [ "$AHEAD" = "0" ]; then
  echo "No commits ahead of $TARGET — nothing to filter."
  exit 0
fi

# The filter below removes files from the index AND the working tree before each
# commit lands, and this command switches branches underneath the user's own
# checkout. An uncommitted edit to a tracked file would be destroyed by that, and
# git cherry-pick refuses to run against a dirty tree anyway — so fail here, where
# the message is legible, rather than midway through the cherry-pick loop.
DIRTY=$(for k in changed renamed unmerged; do specforge git status --pick ".entries[kind=$k].path"; done)
if [ -n "$DIRTY" ]; then
  echo "Working tree has uncommitted changes — commit or stash them first:" >&2
  echo "$DIRTY" >&2
  exit 1
fi
```

The filter has one mode: the spec files are always filtered out, as upstream's strict mode does. `planning.pr_strict` is
not a setting here.

```bash
PR_MODE="strict"   # one mode: every spec file is filtered (planning.pr_strict is not a setting here)

```

Display:
```
### GSD ► PR BRANCH

Branch: {CURRENT_BRANCH}
Target: {TARGET}
Commits: {AHEAD} ahead
Mode:    {PR_MODE}
```
</step>

<step name="handle_sub_repos">
There are no sub-repositories: a sub-repository is another SpecForge project, and `planning.sub_repos` is not a setting here. Continue to `analyze_commits`.
























































































































































</step>

<step name="analyze_commits">
Classify commits:

```bash
# Get all commits ahead of target
specforge git log "$TARGET".."$CURRENT_BRANCH" --no-merges --format human
```

**Canonical path declarations.** These three lines are the single source of truth for the
whole command. `create_pr_branch` derives *which paths it removes* from them, and `verify`
derives *which paths must not appear* from the same three lines — so the two steps cannot
disagree about what the filter promised. Declare them exactly once; do not restate either
list anywhere else in this file. Their literal values are drift-guarded (`npm run lint:ci`,
`scripts/lint-pr-branch-pattern-drift.cjs`) against the canonical `src/pr-branch-patterns.cts`
constants — change one side and update the other, or the guard fails.

```bash
# The spec root and the harness ledger (a file at the project root) are reviewer noise (U47); `app.spec` of a build is code.
TRANSIENT_DIRS=""   # none: specforge-harness is a file
SPEC_ROOT="{{project:spec_root}}"   # the project's spec root, spelled by SpecForge at render (`.`: the project root)
TRANSIENT_FILES="specforge-harness"

# Structural planning files: none here. Every spec file is reviewer noise, so the filter has one mode.


# The spec files, as a pathspec and an ERE: the spec root's tree, or with the spec root at the project root (`.`) the .spec files at any depth (`./` would name every path)
if [ "$SPEC_ROOT" = "." ]; then SPEC_PATHSPEC=':(glob)**/*.spec'; SPEC_RE='\.spec$'
else SPEC_PATHSPEC="$SPEC_ROOT/"; SPEC_RE="^$(printf '%s' "$SPEC_ROOT" | sed 's/[][\.*^$/]/\\&/g')/"; fi









```

Derive the mode's two projections — `FILTER_PATHS` (what `create_pr_branch` removes from
each cherry-picked commit) and `FORBIDDEN_RE` (what `verify` asserts is absent):

```bash

FILTER_PATHS=$(printf '%s\n' "$SPEC_PATHSPEC"; for f in $TRANSIENT_FILES; do printf '%s\n' "$f"; done)
FORBIDDEN_RE="$SPEC_RE|^($(echo "$TRANSIENT_FILES" | tr ' ' '|'))$"
































```

For each commit, check what it touches:

```bash
# For each commit hash
FILES=$(for k in path old_path; do specforge git diff --base "$HASH~1" --head "$HASH" --pick ".files[].$k"; done)
NON_PLANNING=$(echo "$FILES" | grep -Evc "$FORBIDDEN_RE" || true)

PLANNING_COUNT=$(echo "$FILES" | grep -Ec "$FORBIDDEN_RE" || true)
```

Classify: a commit with `NON_PLANNING > 0` is INCLUDED (its spec paths are filtered by `create_pr_branch`); any other is EXCLUDED.




















Display analysis:
```
Commits to include: {N} (code changes)
Commits to exclude: {N} (planning-only)
Mixed commits: {N} (code + planning — included, planning paths filtered)


```
</step>

<step name="create_pr_branch">
```bash
PR_BRANCH="${CURRENT_BRANCH}-pr"

# Create PR branch from target
specforge git switch "$PR_BRANCH" --create --at "$TARGET"
```

Cherry-pick the included commits, in order, filtering `$FILTER_PATHS` out of each one.

The filter forces every filtered path back to **exactly what the PR branch's HEAD already
has**, in both the index and the working tree. That is stricter than simply un-staging, and
both halves matter:

- `git rm -r -f --ignore-unmatch` clears the index entry (including an unmerged one) and
  removes the file the pick just wrote. It only ever touches paths that are in the index, so
  a genuinely untracked planning file of the user's is never harmed.
- `git checkout HEAD --` then restores whatever the target branch legitimately tracks at
  those paths. **Without this, un-staging a path the target branch already tracks records a
  DELETION** — the generated PR would remove the base branch's planning files. In strict mode
  that would be the base's entire spec tree.

Leaving the filtered file behind in the working tree is not an option either: a later commit
touching the same planning path makes `git cherry-pick` abort with *"untracked working tree
files would be overwritten by merge"*, and every remaining commit is silently dropped.

```bash
# Rewrapped through unquoted command substitution (gsd-core#4109): a bare
# `$VAR` word-splits under bash but not zsh, collapsing every element onto
# one iteration there.
for HASH in $(printf '%s' "$INCLUDED_COMMITS"); do
  # A modify/delete conflict on a filtered path is EXPECTED and is resolved below — the
  # filtered path is absent from HEAD by construction. Do not treat it as a failure here.
  specforge git cherry-pick --no-commit "$HASH" || true

  # One path per LINE, not per space (#4605) — a discovered `<milestone>-phases/`
  # directory can contain a space, so `for P in $(printf ...)` would word-split it
  # into spurious entries. `while IFS= read -r` reads $FILTER_PATHS one line at a
  # time instead, matching the newline-safe pattern used to build $FILTER_PATHS above.
  while IFS= read -r P; do
    [ -n "$P" ] || continue
    specforge git rm -r -f --ignore-unmatch -- "$P" >/dev/null 2>&1 || true
    specforge git restore --source HEAD --staged --worktree -- "$P" >/dev/null 2>&1 || true
  done <<FILTER_PATHS_EOF
$FILTER_PATHS
FILTER_PATHS_EOF

  # every spec path is in FILTER_PATHS, resolved by the loop above: a path still unmerged below is a real conflict





















































  # Anything still unmerged is a REAL conflict, outside the filter. Halt — do not
  # improvise a resolution and do not continue, which would drop the rest of the queue.
  # Unwind first: this loop runs in the user's own checkout, so exiting mid-sequence
  # would strand them on a half-built branch with cherry-pick state still live.
  if [ -n "$(specforge git status --pick '.entries[kind=unmerged].path')" ]; then
    echo "Conflict outside the *.spec filter while picking $HASH:" >&2
    specforge git status --pick '.entries[kind=unmerged].path' >&2
    # Order matters. `--quit` drops the sequencer state but leaves the unmerged index
    # in place, and an unmerged index makes `git checkout` refuse — so reset first.
    # $PR_BRANCH is disposable and every commit on it was cherry-picked, and the
    # clean-tree precondition guarantees the user had nothing uncommitted, so a hard
    # reset here cannot destroy anything of theirs.
    specforge git cherry-pick --quit 2>/dev/null || true
    specforge git reset HEAD --hard --yes --pick .sha >/dev/null
    if specforge git switch "$CURRENT_BRANCH" --pick .branch >/dev/null; then
      specforge git branch delete "$PR_BRANCH" --force >/dev/null 2>&1 || true
      echo "Restored $CURRENT_BRANCH and removed the partial $PR_BRANCH." >&2
    else
      # Never claim a restore that did not happen — say exactly where they are.
      echo "Could not return to $CURRENT_BRANCH; you are still on $PR_BRANCH." >&2
      echo "Run: git checkout $CURRENT_BRANCH && git branch -D $PR_BRANCH" >&2
    fi
    echo "Resolve the conflict against $TARGET, then re-run /gsd:pr-branch." >&2
    exit 1
  fi

  # Nothing left after filtering (possible when a pick's only surviving content was
  # planning state): clear the sequencer rather than failing on an empty commit.
  if [ "$(specforge git diff --staged --pick .count)" = 0 ]; then
    specforge git cherry-pick --quit 2>/dev/null || true
    continue
  fi

  specforge git commit -C "$HASH" >/dev/null
done
```

Return to original branch:
```bash
specforge git switch "$CURRENT_BRANCH"
```
</step>

<step name="verify">
Assert against the filter's contract — `$FORBIDDEN_RE`, the same declaration
`create_pr_branch` filtered on: no spec file and no harness ledger may remain.



```bash
DIFF_PATHS=$(specforge git diff --base "$TARGET" --head "$PR_BRANCH" --pick '.files[].path')
FORBIDDEN=$(echo "$DIFF_PATHS" | grep -Ec "$FORBIDDEN_RE" || true)
PLANNING_TOTAL=$(echo "$DIFF_PATHS" | grep -Ec "$FORBIDDEN_RE" || true)
ALLOWED=$((PLANNING_TOTAL - FORBIDDEN))
TOTAL_FILES=$(specforge git diff --base "$TARGET" --head "$PR_BRANCH" --pick .count)
PR_COMMITS=$(specforge git log "$TARGET".."$PR_BRANCH" --pick .count)

# #3679: a DELETED planning path is never legitimate — this workflow only ever
# excludes content a cherry-picked commit ADDED; pre-existing target-tracked
# planning files must survive byte-identical. Name-only counting cannot see
# status (a deleted structural/allowed path verifies clean there), so gate on
# deletions explicitly, across every planning category.
PLANNING_DELETIONS=$(for p in "[status=deleted].path" "[status=renamed].old_path"; do specforge git diff --base "$TARGET" --head "$PR_BRANCH" --pick ".files$p"; done | grep -Ec "$FORBIDDEN_RE" || true)





```

`$FORBIDDEN` is the pass/fail number — it must be `0`. A non-zero value means the filter
did not do what this mode promised; report it and do not tell the user to push.

`$PLANNING_DELETIONS` is a second hard gate (#3679) — it must also be `0`. A non-zero
value means the PR branch would DELETE planning files the target branch tracks
(`git diff --name-status --no-renames "$TARGET".."$PR_BRANCH" | grep "^D" | grep "\.spec"` lists
them). That is data loss, not filtering — report it, do not tell the user to push, and
rebuild the branch.

Display results:
```
✅ PR branch created: {PR_BRANCH}

Original: {AHEAD} commits, {ORIGINAL_FILES} files
PR branch: {PR_COMMITS} commits, {TOTAL_FILES} files
Mode: {PR_MODE}
Planning paths in diff: {PLANNING_TOTAL} (allowed {ALLOWED}, forbidden {FORBIDDEN} — must be 0)
Planning deletions: {PLANNING_DELETIONS} (must be 0 — #3679)

Next steps:
  git push origin {PR_BRANCH}
  gh pr create --base {TARGET} --head {PR_BRANCH}

Or use /gsd:ship to create the PR automatically.
```







</step>

</process>

<success_criteria>
- [ ] Working tree was clean before the PR branch was created
- [ ] PR branch created from target
- [ ] Planning-only commits excluded
- [ ] Zero paths matching `$FORBIDDEN_RE` in the PR branch diff: no spec path at all

- [ ] No spec path the target branch already tracked was deleted
- [ ] Every included commit landed — none dropped by a failed cherry-pick
- [ ] Commit messages preserved from original
- [ ] User shown next steps
</success_criteria>

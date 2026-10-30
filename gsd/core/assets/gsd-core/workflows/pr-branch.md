<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/pr-branch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
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
Parse `$ARGUMENTS` for target branch. If no argument is supplied, detect the
default branch via the single resolver (#1146).

```bash
# the commands below are specforge commands: there is no launcher to define
CURRENT_BRANCH=$(git branch --show-current)
TARGET=$(echo "$ARGUMENTS" | sed -nE 's/.*--target[[:space:]]+"?([A-Za-z0-9._\/-]+).*/\1/p'); TARGET=${TARGET:-$(printf '%s' "{{setting:git.base_branch|}}" | grep . || git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's|^origin/||' | grep . || for b in main master; do git rev-parse --verify -q "$b" >/dev/null && echo "$b" && break; done)}
```

Check preconditions:
- Must be on a feature branch (not main/master)
- Must have commits ahead of target
- Working tree must be clean

```bash
AHEAD=$(git rev-list --count "$TARGET".."$CURRENT_BRANCH" 2>/dev/null)
if [ "$AHEAD" = "0" ]; then
  echo "No commits ahead of $TARGET — nothing to filter."
  exit 0
fi

# The filter below removes files from the index AND the working tree before each
# commit lands, and this command switches branches underneath the user's own
# checkout. An uncommitted edit to a tracked file would be destroyed by that, and
# git cherry-pick refuses to run against a dirty tree anyway — so fail here, where
# the message is legible, rather than midway through the cherry-pick loop.
DIRTY=$(git status --porcelain --untracked-files=no)
if [ -n "$DIRTY" ]; then
  echo "Working tree has uncommitted changes — commit or stash them first:" >&2
  echo "$DIRTY" >&2
  exit 1
fi
```

The filter has one mode: the spec files are always filtered out, as upstream's strict mode does. `planning.pr_strict` is
not a setting here.

```bash
PR_STRICT="true"   # planning.pr_strict: not a setting here, the filter is always strict
if [ "$PR_STRICT" = "true" ]; then PR_MODE="strict"; else PR_MODE="default"; PR_STRICT="false"; fi
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
Read the sub-repo list from config using the canonical key path — `planning.sub_repos`.
A non-zero exit code means the key is absent; treat that as "no sub-repos configured".

```bash
SUB_REPOS_JSON="[]"   # planning.sub_repos: a sub-repository is another SpecForge project, so there are none to list
if [ $? -ne 0 ] || [ -z "$SUB_REPOS_JSON" ] || [ "$SUB_REPOS_JSON" = "null" ] || [ "$SUB_REPOS_JSON" = "[]" ]; then
  : # Not configured or empty — skip to analyze_commits
fi
```

Scan each sub-repo for uncommitted changes using node (always available — avoids undeclared
jq dependency). Write dirty repo names to a temp file so the list survives across
subsequent command executions:

```bash
ROOT=$(git rev-parse --show-toplevel)
DIRTY_FILE=$(mktemp)

node -e "
  const repos = JSON.parse(process.argv[1]);
  const { execFileSync } = require('child_process');
  const path = require('path');
  const fs = require('fs');
  const root = process.argv[2];
  // realpath parity with the pr-subrepo seam's validatePath: resolve $ROOT through
  // symlinks once so the containment check below compares real paths, not text.
  let realRoot;
  try { realRoot = fs.realpathSync(root); } catch (_) { realRoot = path.resolve(root); }
  const out = [];
  for (const r of repos) {
    // Reject before any git invocation: this scan runs on raw config values,
    // ahead of the pr-subrepo seam's own validatePath guard. A traversal,
    // embedded-newline, or symlink entry here would run git outside the
    // workspace, or inject a spurious record into the dirty-file output.
    if (typeof r !== 'string' || !/^[A-Za-z0-9._\/-]+$/.test(r)) continue;
    // realpathSync follows symlinks — path.resolve only normalizes '..' textually,
    // so an in-tree symlink pointing outside root would otherwise smuggle git out.
    let resolved;
    try { resolved = fs.realpathSync(path.resolve(realRoot, r)); } catch (_) { continue; }
    if (resolved !== realRoot && !resolved.startsWith(realRoot + path.sep)) continue;
    try {
      const res = execFileSync('git', ['-C', resolved, 'status', '--porcelain'],
                               { encoding: 'utf8', timeout: 10_000 });
      // Exclude untracked-only repos: seam filters ?? lines, so detection must match.
      const tracked = res.split('\n').filter(l => l.length > 0 && !l.startsWith('??'));
      if (tracked.length > 0) out.push(r);
    } catch (_) {}
  }
  fs.writeFileSync(process.argv[3], out.join('\n'));
" "$SUB_REPOS_JSON" "$ROOT" "$DIRTY_FILE"

DIRTY_REPOS=$(cat "$DIRTY_FILE")
```

If `$DIRTY_REPOS` is empty, remove the temp file and continue to `analyze_commits`.

Display dirty repos and prompt the user:

```
Sub-repos with uncommitted changes:
  backend
  frontend

How should sub-repo changes be handled?
  1. all    — branch, commit (explicit files only), push -u, open companion PR per repo
  2. select — choose which sub-repos to process
  3. skip   — ignore sub-repos, continue with root repo only
```

If the user chooses **skip**, remove the temp file and continue to `analyze_commits`.

For each selected sub-repo `$REPO_REL`, delegate all git work to the `pr-subrepo` query
seam — it stages explicit changed files (never `git add -A`), creates the branch,
commits, and pushes with `--set-upstream`. Branch names include the repo slug to avoid
colliding with the root `PR_BRANCH` that `create_pr_branch` creates later:

```bash
# Replace path separators to make the name safe as a branch component
REPO_SAFE="${REPO_REL//\//-}"
SUB_BRANCH="${CURRENT_BRANCH}-${REPO_SAFE}-pr"
COMMIT_MSG="fix(${REPO_REL}): sync uncommitted changes for PR"

RESULT=$(false)   # pr-subrepo: not available here, a sub-repository is another SpecForge project


SUBREPO_EXIT=$?
```

If the seam exited non-zero (stage/commit/push failure), report its error and move on to
the next selected sub-repo. **Do not run the companion-PR step below for this repo** —
the seam's stderr already explains the failure, and the "branch pushed" path would
otherwise contradict it:

```bash
if [ "$SUBREPO_EXIT" -ne 0 ]; then
  echo "pr-subrepo failed for $REPO_REL — see error above; skipping companion PR." >&2
fi
```

Only when `$SUBREPO_EXIT` is `0`, parse the structured result with node and open the
companion PR. If `remote_slug` is null (non-GitHub remote), skip `gh pr create` and show
the push URL instead:

```bash
REMOTE_SLUG=$(node -e "
  try { console.log(JSON.parse(process.argv[1]).remote_slug || ''); } catch(_) {}
" "$RESULT")

if [ -n "$REMOTE_SLUG" ]; then
  # Defense-in-depth: $REPO_REL was already validated by the dirty-scan filter and
  # the pr-subrepo seam's validatePath, but these are separate, independent git -C
  # invocations on the same value. Resolve it through symlinks with the SAME realpath
  # containment the seam uses (path.resolve alone would not catch a symlink escape),
  # and run git against the validated absolute path rather than re-concatenating.
  SUB_REPO_DIR=$(node -e "
    const fs = require('fs'), path = require('path');
    try {
      const realRoot = fs.realpathSync(process.argv[1]);
      const resolved = fs.realpathSync(path.resolve(realRoot, process.argv[2]));
      if (resolved !== realRoot && !resolved.startsWith(realRoot + path.sep)) process.exit(1);
      process.stdout.write(resolved);
    } catch (_) { process.exit(1); }
  " "$ROOT" "$REPO_REL" 2>/dev/null)

  if [ -z "$SUB_REPO_DIR" ]; then
    echo "Refusing unsafe sub-repo path: $REPO_REL" >&2
    SUB_TARGET="$TARGET"
  else
    # Resolve base branch: use $TARGET if it exists in sub-repo, else fall back to
    # the sub-repo's own default branch
    if git -C "$SUB_REPO_DIR" ls-remote --exit-code --heads origin "$TARGET" \
         > /dev/null 2>&1; then
      SUB_TARGET="$TARGET"
    else
      SUB_TARGET=$(git -C "$SUB_REPO_DIR" remote show origin 2>/dev/null \
        | awk '/HEAD branch/ {print $NF}')
      SUB_TARGET="${SUB_TARGET:-main}"
    fi
  fi

  gh pr create \
    --repo "$REMOTE_SLUG" \
    --base "$SUB_TARGET" \
    --head "$SUB_BRANCH" \
    --title "$COMMIT_MSG" \
    --body "Companion PR for root repo branch \`$CURRENT_BRANCH\`."
else
  echo "No GitHub remote detected for $REPO_REL — branch pushed, open PR manually."
fi
```

After processing all selected sub-repos, remove the temp file and continue to
`analyze_commits` for the root repo.
</step>

<step name="analyze_commits">
Classify commits:

```bash
# Get all commits ahead of target
git log --oneline "$TARGET".."$CURRENT_BRANCH" --no-merges
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
SPEC_ROOT=$(jq -r '.spec_root // "spec"' specforge.json 2>/dev/null || echo spec)
TRANSIENT_FILES="specforge-harness"

# Structural planning files: none here. Every spec file is reviewer noise, so the filter has one mode.






STRUCTURAL_RE="a^"   # matches nothing
# Milestone-scoped phase-plan directories: none here.





MILESTONE_PHASES_RE="a^"   # matches nothing
```

Derive the mode's two projections — `FILTER_PATHS` (what `create_pr_branch` removes from
each cherry-picked commit) and `FORBIDDEN_RE` (what `verify` asserts is absent):

```bash
if [ "$PR_STRICT" = "true" ]; then
  FILTER_PATHS=$(printf '%s/\n' "$SPEC_ROOT"; for f in $TRANSIENT_FILES; do printf '%s\n' "$f"; done)
  FORBIDDEN_RE="^($(printf '%s' "$SPEC_ROOT" | sed 's/[][\.*^$/]/\\&/g')/|$(echo "$TRANSIENT_FILES" | tr ' ' '|')$)"
else
  : # default mode keeps structural planning state, and there is none here (the filter is always strict)





























fi
```

For each commit, check what it touches:

```bash
# For each commit hash
FILES=$(git diff-tree --no-commit-id --name-only -r $HASH)
NON_PLANNING=$(echo "$FILES" | grep -Evc "$FORBIDDEN_RE" || true)
STRUCTURAL=$(echo "$FILES" | grep -Ec "$STRUCTURAL_RE" || true)
PLANNING_COUNT=$(echo "$FILES" | grep -Ec "$FORBIDDEN_RE" || true)
```

Classify, using `NON_PLANNING`, `STRUCTURAL`, and `PLANNING_COUNT` computed above — every arm's
condition is explicit and computable so no reading of it is ambiguous:
- **Code commits**: `NON_PLANNING > 0` and `PLANNING_COUNT == 0` → INCLUDE (both modes)
- **Mixed code+planning commits**: `NON_PLANNING > 0` and `PLANNING_COUNT > 0` → INCLUDE (both
  modes; the planning paths are filtered out by `create_pr_branch`, not the commit)
- **Structural-only planning commits**: `NON_PLANNING == 0` and `STRUCTURAL == PLANNING_COUNT`
  and `PLANNING_COUNT > 0` (every spec file touched is structural) → INCLUDE in
  **default** mode; **EXCLUDE** in strict mode, which has no structural carve-out
- **Mixed planning commits (#4447)**: `NON_PLANNING == 0` and `STRUCTURAL > 0` and
  `STRUCTURAL < PLANNING_COUNT` (some but not all spec files touched are structural —
  the rest are transient and/or the "other" bucket, e.g. `config.json`/`intel/`) → INCLUDE in
  **default** mode (the transient-dir subset of the non-structural paths is filtered out by
  `create_pr_branch`'s universal per-commit filter exactly as for a mixed code+planning commit;
  any "other" non-structural, non-transient path — `config.json`, `intel/`, etc. — is simply
  preserved, same as default mode already does for such paths on any commit); **EXCLUDE** in
  strict mode
- **Transient-only planning commits**: `NON_PLANNING == 0` and `STRUCTURAL == 0` and
  `PLANNING_COUNT > 0` → EXCLUDE (both modes)

In strict mode this collapses to a single rule: `NON_PLANNING > 0` → INCLUDE, else EXCLUDE.

Display analysis:
```
Commits to include: {N} (code changes{, + structural planning — default mode only})
Commits to exclude: {N} (planning-only)
Mixed commits: {N} (code + planning — included, planning paths filtered)
Structural planning commits: {N} ({included|excluded — strict mode})
Mixed planning commits: {N} ({included — structural + transient/other, planning paths filtered|excluded — strict mode})
```
</step>

<step name="create_pr_branch">
```bash
PR_BRANCH="${CURRENT_BRANCH}-pr"

# Create PR branch from target
git checkout -b "$PR_BRANCH" "$TARGET"
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
  git cherry-pick --no-commit "$HASH" || true

  # One path per LINE, not per space (#4605) — a discovered `<milestone>-phases/`
  # directory can contain a space, so `for P in $(printf ...)` would word-split it
  # into spurious entries. `while IFS= read -r` reads $FILTER_PATHS one line at a
  # time instead, matching the newline-safe pattern used to build $FILTER_PATHS above.
  while IFS= read -r P; do
    [ -n "$P" ] || continue
    git rm -r -f -q --ignore-unmatch -- "$P" 2>/dev/null || true
    git checkout HEAD -- "$P" 2>/dev/null || true
  done <<FILTER_PATHS_EOF
$FILTER_PATHS
FILTER_PATHS_EOF

  # #4606: a conflict on a "third bucket" path (a spec path — not `$FORBIDDEN_RE`
  # transient, not `$STRUCTURAL_RE` structural — the same bucket `verify`'s
  # `$OTHER` reports) is not a real conflict either, and needs its own
  # resolution distinct from the filter loop above. Such a path is never
  # per-commit replayed by classification — a commit touching ONLY a
  # third-bucket path is EXCLUDEd — so when a LATER included commit (structural
  # or code + that same path) reuses it, the diff's context can predate
  # whatever the PR branch actually has, and cherry-pick reports a genuine
  # content conflict on a path this command was never asked to filter.
  # `git checkout --theirs` resolves it correctly by construction: in a
  # cherry-pick's 3-way merge, "theirs" IS $HASH's own content for that path —
  # exactly the chained final value the path is owed, the same guarantee
  # `create_pr_branch` already gives structural files. A path $HASH deletes has
  # no "theirs" blob to check out, so fall back to accepting the deletion (the
  # `verify` step's `$PLANNING_DELETIONS` gate independently catches this if
  # `$TARGET` ever legitimately tracked that path).
  #
  # The deleted-by-$HASH case is checked explicitly with `git cat-file -e`
  # rather than inferred from `checkout --theirs` failing, so an unrelated
  # checkout failure (I/O, permissions, a stale index lock) can't be
  # misread as a deletion and silently `git rm`-ed — it falls through to the
  # unmerged-path halt below instead.
  #
  # The `git add` below restages a path already committed to $CURRENT_BRANCH's
  # own history onto the disposable $PR_BRANCH; not a commit_docs bypass
  # (#1783/#3585), which guards against staging spec/ content that was
  # never committed at all.
  #
  # The unmerged list is snapshotted, then read one path PER LINE. `for P in
  # $(git diff --name-only --diff-filter=U)` would split it on IFS instead: a
  # third-bucket path containing a space (`spec/My Notes.md`) becomes the
  # fragments `spec/My` and `Notes.md`, neither of which names the real
  # conflicted file, so nothing is resolved and the run aborts with exactly the
  # #4606 failure this block exists to prevent. Same word-split class as #4109.
  # Snapshot-then-iterate (rather than piping) both keeps the list stable while
  # the body restages paths and keeps the body in THIS shell, not a subshell.
  UNMERGED_PATHS=$(git diff --name-only --diff-filter=U)
  while IFS= read -r P; do
    [ -n "$P" ] || continue
    case "$P" in
      "$SPEC_ROOT"/*|specforge-harness) ;;
      *) continue ;;
    esac
    if echo "$P" | grep -Eq "$FORBIDDEN_RE"; then continue; fi
    if echo "$P" | grep -Eq "$STRUCTURAL_RE"; then continue; fi
    if git cat-file -e "$HASH:$P" 2>/dev/null; then
      git checkout --theirs -- "$P" && git add -- "$P" # gsd-scan-ignore: #4606 -- see block comment above
    else
      git rm -f -q -- "$P" 2>/dev/null || true
    fi
  done <<UNMERGED_PATHS_EOF
$UNMERGED_PATHS
UNMERGED_PATHS_EOF

  # Anything still unmerged is a REAL conflict, outside the filter. Halt — do not
  # improvise a resolution and do not continue, which would drop the rest of the queue.
  # Unwind first: this loop runs in the user's own checkout, so exiting mid-sequence
  # would strand them on a half-built branch with cherry-pick state still live.
  if [ -n "$(git diff --name-only --diff-filter=U)" ]; then
    echo "Conflict outside the *.spec filter while picking $HASH:" >&2
    git diff --name-only --diff-filter=U >&2
    # Order matters. `--quit` drops the sequencer state but leaves the unmerged index
    # in place, and an unmerged index makes `git checkout` refuse — so reset first.
    # $PR_BRANCH is disposable and every commit on it was cherry-picked, and the
    # clean-tree precondition guarantees the user had nothing uncommitted, so a hard
    # reset here cannot destroy anything of theirs.
    git cherry-pick --quit 2>/dev/null || true
    git reset -q --hard HEAD
    if git checkout -q "$CURRENT_BRANCH"; then
      git branch -q -D "$PR_BRANCH" 2>/dev/null || true
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
  if git diff --cached --quiet; then
    git cherry-pick --quit 2>/dev/null || true
    continue
  fi

  git commit -q -C "$HASH"
done
```

Return to original branch:
```bash
git checkout "$CURRENT_BRANCH"
```
</step>

<step name="verify">
Assert against the **active mode's** contract — `$FORBIDDEN_RE`, the same declaration
`create_pr_branch` filtered on. Counting every spec path unconditionally would
contradict default mode, which is specified to preserve structural files: a correct run
would report itself as failed on every phase that touched STATE.md, which is every phase.

```bash
DIFF_PATHS=$(git diff --name-only "$TARGET".."$PR_BRANCH")
FORBIDDEN=$(echo "$DIFF_PATHS" | grep -Ec "$FORBIDDEN_RE" || true)
PLANNING_TOTAL=$(echo "$DIFF_PATHS" | grep -Ec "$FORBIDDEN_RE" || true)
ALLOWED=$((PLANNING_TOTAL - FORBIDDEN))
TOTAL_FILES=$(echo "$DIFF_PATHS" | grep -c . || true)
PR_COMMITS=$(git rev-list --count "$TARGET".."$PR_BRANCH")

# #3679: a DELETED planning path is never legitimate — this workflow only ever
# excludes content a cherry-picked commit ADDED; pre-existing target-tracked
# planning files must survive byte-identical. Name-only counting cannot see
# status (a deleted structural/allowed path verifies clean there), so gate on
# deletions explicitly, across every planning category.
PLANNING_DELETIONS=$(git diff --name-status --no-renames "$TARGET".."$PR_BRANCH" | grep "^D" | cut -f2- | grep -Ec "$FORBIDDEN_RE" || true)

# Default mode preserves anything of the spec that is neither transient nor
# structural — config.json, intel/, workstreams/. That is deliberate and unchanged, but it
# must not be silent: report it so the user can choose strict mode knowingly.
OTHER=""   # strict mode: every spec path is forbidden, so none is kept
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

When `$OTHER` is non-empty (default mode only — strict forbids all of it), append:
```
ℹ️  These spec/ paths are neither transient nor structural, so default mode keeps them:
{OTHER}
    (the filter is always strict here: no spec path is kept)
```
</step>

</process>

<success_criteria>
- [ ] Working tree was clean before the PR branch was created
- [ ] PR branch created from target
- [ ] Planning-only commits excluded
- [ ] Zero paths matching the active mode's `$FORBIDDEN_RE` in the PR branch diff —
      strict: no spec path at all; default: none from `$TRANSIENT_DIRS`
- [ ] No spec path the target branch already tracked was deleted
- [ ] Every included commit landed — none dropped by a failed cherry-pick
- [ ] Commit messages preserved from original
- [ ] User shown next steps
</success_criteria>

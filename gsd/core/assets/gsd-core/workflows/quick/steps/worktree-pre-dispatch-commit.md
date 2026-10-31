<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick/steps/worktree-pre-dispatch-commit.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 5.6: Pre-dispatch artifact commit (worktree mode only)**

When `USE_WORKTREES !== "false"`, commit the quick plan — the entity `plan ${quick_id}`
in `spec/quick.spec`, plus the context and research entities the discussion/research
phases wrote under `spec/` — to the current branch **before** spawning the executor.
This ensures the worktree inherits them at its branch HEAD so the executor reads them
through worktree-rooted paths — avoiding the main-repo path priming that triggers CC
#36182 path-resolution drift, and avoiding the locked decisions and research findings
being stranded outside the worktree while only the plan reaches it (#4996).

Skip this step entirely if `USE_WORKTREES === "false"` (non-worktree mode: these artifacts
are committed in Step 8 as usual).

```bash
# the commands below are specforge commands: there is no launcher to define
QUICK_PLAN_PARENT=""
QUICK_PLAN_COMMIT=""
if [ "${USE_WORKTREES}" != "false" ]; then
  COMMIT_DOCS="{{setting:commit_docs|true}}"   # commit_docs (the settings)
  if [ "$COMMIT_DOCS" != "false" ]; then
    # pre-dispatch plan commit. SpecForge writes no ${QUICK_DIR}/${quick_id}-PLAN.md,
    # -CONTEXT.md or -RESEARCH.md file: the plan is the entity `plan ${quick_id}` in
    # spec/quick.spec, and what --discuss / --research found is entities under spec/
    # as well. So the one path is `spec`, as Step 8 commits it (`git add spec`), and
    # no mode flag adds or drops one.
    QUICK_PREDISPATCH_PATHS=(spec)
    # The executor's first action checks the same entity at QUICK_PLAN_COMMIT, and
    # the alternate worktree base below is the parent of the commit that wrote it:
    # with no such commit, QUICK_PLAN_PARENT stays empty and the guard fails closed.
    # An array (not a space-joined string) keeps each path a single git argument
    # under `set -u` word-splitting rules (ShellCheck SC2086), as the git commit
    # lines below pass it.
    git add "${QUICK_PREDISPATCH_PATHS[@]}"
    # No-op skip if nothing actually staged (idempotent re-runs).
    if git diff --cached --quiet -- "${QUICK_PREDISPATCH_PATHS[@]}"; then
      echo "ℹ Pre-dispatch artifact commit skipped (no staged changes)"
    else
      # Run hooks normally (#2924). If a project opts out via
      # workflow.worktree_skip_hooks=true, honor that opt-in only.
      SKIP_HOOKS="{{setting:workflow.worktree_skip_hooks|}}"   # workflow.worktree_skip_hooks (the settings)
      if [ "$SKIP_HOOKS" = "true" ]; then
        git commit --no-verify -m "docs(${quick_id}): pre-dispatch artifacts for ${DESCRIPTION}" -- "${QUICK_PREDISPATCH_PATHS[@]}" \
          || { echo "ERROR: pre-dispatch artifact commit failed (--no-verify path). Aborting before executor dispatch." >&2; exit 1; }
      else
        git commit -m "docs(${quick_id}): pre-dispatch artifacts for ${DESCRIPTION}" -- "${QUICK_PREDISPATCH_PATHS[@]}" \
          || { echo "ERROR: pre-dispatch artifact commit failed — likely a pre-commit hook failure. Fix the hook output above (or set workflow.worktree_skip_hooks=true to bypass) and re-run." >&2; exit 1; }
      fi
      QUICK_PLAN_COMMIT=$(git rev-parse HEAD)
    fi
  fi
  if [ -z "$QUICK_PLAN_COMMIT" ]; then
    QUICK_PLAN_COMMIT=$(git rev-parse HEAD)
  fi
  # #5209: the alternate base is the PLAN commit's immediate parent, read AFTER the
  # commit attempt so it holds whether the planner already committed the plan or this
  # step just did. Empty when docs commits are off, for a root commit, and with no plan commit.
  if [ "$COMMIT_DOCS" != "false" ]; then
    _GSD_PLAN_AT=$(git log -1 --format=%H -S "plan ${quick_id}" -- spec 2>/dev/null || true)
    QUICK_PLAN_PARENT=""
    [ -z "$_GSD_PLAN_AT" ] || QUICK_PLAN_PARENT=$(git rev-parse --verify --quiet "${_GSD_PLAN_AT}^" 2>/dev/null || true)
  fi
fi
```

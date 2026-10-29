<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/quick/steps/worktree-pre-dispatch-commit.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 5.6: Pre-dispatch artifact commit (worktree mode only)**

When `USE_WORKTREES !== "false"`, commit PLAN.md — plus CONTEXT.md and/or RESEARCH.md
when the discussion/research phases produced them — to the current branch **before**
spawning the executor. This ensures the worktree inherits these artifacts at its branch
HEAD so the executor can read them via a worktree-rooted path — avoiding the main-repo
path priming that triggers CC #36182 path-resolution drift, and avoiding the locked
decisions (CONTEXT.md) and research findings (RESEARCH.md) being stranded outside the
worktree while only PLAN.md reaches it (#4996).

Skip this step entirely if `USE_WORKTREES === "false"` (non-worktree mode: these artifacts
are committed in Step 8 as usual).

```bash
# the commands below are specforge commands: there is no launcher to define
QUICK_PLAN_PARENT=""
QUICK_PLAN_COMMIT=""
if [ "${USE_WORKTREES}" != "false" ]; then
  QUICK_PLAN_PARENT=$(git rev-parse HEAD)
  COMMIT_DOCS="{{setting:commit_docs|}}"   # commit_docs (the settings)
  if [ "$COMMIT_DOCS" != "false" ]; then
    # pre-dispatch plan commit — PLAN.md is always in scope. CONTEXT.md/RESEARCH.md
    # join it only when their phase actually ran AND actually produced a file — a
    # mode flag alone is not proof the file exists (#4996). An array (not a
    # space-joined string) keeps each path a single git argument under `set -u`
    # word-splitting rules (ShellCheck SC2086).
    QUICK_PREDISPATCH_PATHS=("${QUICK_DIR}/${quick_id}-PLAN.md")
    if [ "${DISCUSS_MODE}" = "true" ] && [ -f "${QUICK_DIR}/${quick_id}-CONTEXT.md" ]; then
      QUICK_PREDISPATCH_PATHS+=("${QUICK_DIR}/${quick_id}-CONTEXT.md")
    fi
    if [ "${RESEARCH_MODE}" = "true" ] && [ -f "${QUICK_DIR}/${quick_id}-RESEARCH.md" ]; then
      QUICK_PREDISPATCH_PATHS+=("${QUICK_DIR}/${quick_id}-RESEARCH.md")
    fi
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
fi
```

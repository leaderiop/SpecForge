<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/steps/protected-branch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Protected-branch warning for `branching_strategy: none` (#3552)

Run this from the `handle_branching` step's `"none"` arm, after deciding to
continue on the current branch. It warns without refusing execution — the
`"none"` strategy still runs on whatever branch it started on.

```bash
# the commands below are specforge commands: there is no launcher to define
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || true)
IS_PROTECTED=$(case "$CURRENT_BRANCH" in main|master|develop|release/*) echo true;; *) echo false;; esac) || IS_PROTECTED=""
if [ "$IS_PROTECTED" = true ]; then
  echo "⚠ Current branch '$CURRENT_BRANCH' is a protected branch; branching_strategy=none will continue here." >&2
elif [ -z "$IS_PROTECTED" ]; then
  echo "⚠ Could not determine whether '$CURRENT_BRANCH' is protected — the query failed. Continuing." >&2
fi
```

The `IS_PROTECTED=""` fallback on the first line, and the second `elif`, exist
so a failed or missing `specforge` invocation degrades **visibly** (an explicit
"could not determine" warning) rather than silently reading as "not
protected" under `set -e` (#3648 review, round 5).

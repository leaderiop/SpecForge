<!-- Ported from gsd-core v1.19.0 gsd-core/references/execute-phase-between-wave-reset.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
7b. **Pre-wave dependency check (waves 2+ only):**
    Before wave N+1, run `specforge check` for each upcoming plan.
    If any PRIOR-wave artifact link fails, present:
    - `## Cross-Plan Wiring Gap` with plan/link/from/pattern rows
    - Options: investigate+fix before continue, or continue with cascade risk
    Skip key-links that reference files in the CURRENT (upcoming) wave.
    Exit status (#5170): `0` all verified; `1` a link failed (read the JSON); `66` the plan declares no
    key_links (nothing to check); `69` the plan is missing/unreadable (could not look — surface it, never
    treat the plan as wired).

7c. **No between-wave worktree reset:** `@gsd/core` runs no worktree isolation (no wave manifest to unset, no fork base to refresh).






































<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Part A — milestone-state write (skip when a workstream is active).** Skip Part A if `WS` (Step 1) is non-empty or a workstream is active. The active workstream's own state, roadmap and requirement entities already carry this milestone's state. Recording the milestone here would clobber the shared project's current milestone, and with parallel milestones across workstreams, whichever workstream runs `new-milestone` last would silently win (#2308). In flat mode (`WS` empty), run Part A: record the milestone, its goal and its target features (one `--target_features` per feature). A refusal names its code: read it with `specforge explain <code>`.

```bash
VERSION="{the milestone version, e.g. v1.1}"
NAME="{the milestone name}"
GOAL="{one sentence describing the milestone focus}"
FEATURE="{a target feature: one --target_features per feature}"
specforge new milestone "$VERSION" "$NAME" --in "spec/$VERSION.spec" --name "$NAME" --goal "$GOAL" --target_features "$FEATURE" || exit 1
```

The project's active requirements are computed: there is no section or footer to update.

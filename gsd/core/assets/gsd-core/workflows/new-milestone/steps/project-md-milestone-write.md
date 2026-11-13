<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Part A — the milestone, in flat mode and in a workstream alike.** Record the milestone, its goal and its target features (one `--target_features` per feature). With a workstream (`WS`, bound in Step 1) the milestone is the workstream's own: `--workstream` scopes its id to `<workstream>/<version>`, so two workstreams can each hold a `v1.1` without touching the other's, and the roadmap's phases name that id (#2308). In flat mode `WS` is empty and the id is the version. A refusal names its code: read it with `specforge explain <code>`.

```bash
WS="{the workstream Step 1 bound, or nothing in flat mode}"
VERSION="{the milestone version, e.g. v1.1}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the milestone name}
TEXT
IFS= read -rd '' GOAL <<'TEXT' || true; GOAL=${GOAL%$'\n'}
{one sentence describing the milestone focus}
TEXT
IFS= read -rd '' FEATURE <<'TEXT' || true; FEATURE=${FEATURE%$'\n'}
{a target feature: one --target_features per feature}
TEXT
specforge new milestone "$VERSION" "$NAME" --in "spec/$VERSION.spec" --name "$NAME" --goal "$GOAL" --target_features "$FEATURE" ${WS:+--workstream "$WS"} || exit 1
```

The project's active requirements are computed: there is no section or footer to update.

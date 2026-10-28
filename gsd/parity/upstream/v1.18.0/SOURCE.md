# gsd-core v1.18.0: the configuration roster

Frozen so the roster check (`gsd/core/tests/settings_roster.rs`) runs offline and fails when the tracked tag moves
without the snapshot.

| File | Taken from |
|---|---|
| `config-schema.manifest.json` | `gsd-core/bin/shared/config-schema.manifest.json`, unchanged |
| `config-defaults.manifest.json` | `gsd-core/bin/shared/config-defaults.manifest.json`, unchanged |
| `capabilities.json` | the `config` block of each `capabilities/*/capability.json`, one entry per key: `{capability, type, default, values?}` |

- Repository: <https://github.com/open-gsd/gsd-core>, tag `v1.18.0` (commit `35f5a452239b897b8d6352c973e1e4efd683370f`).
- Licence: MIT, Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).
- The roster also names keys only `docs/CONFIGURATION.md` lists (`parallelization.*`, `dynamic_routing.*`,
  `worktree.baseRef`, `context_profile`, the tier defaults of `effort` and `fast_mode`); `settings_roster.rs` lists
  them by hand.
- Defaults come from `config-defaults.manifest.json`, then the capability's own default, then
  `docs/CONFIGURATION.md`. Where the docs disagree with the defaults manifest the manifest wins
  (`workflow.inline_plan_threshold`: docs 3, manifest 2).

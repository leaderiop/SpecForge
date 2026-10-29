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

## `gate-cutover/`: deterministic gate verdicts

Verdicts of gsd-core's deterministic `gsd-tools` gates, copied unchanged by `git -C <gsd-core> show
v1.18.0:tests/fixtures/gate-cutover/<file>.json`. The fixtures that produced them are built by
`tests/fixtures/gate-cutover/arms.cjs` (`dc`, `vp`, `gap`); `gsd/core/tests/projects/gate_cutover/` is their twin, and
the tests in `tests/cli/parity_plans.rs` and the passes' native tests reproduce the verdicts.

| File | Read by |
|---|---|
| `gap-analysis-plan-post-enabled.json` | the table and summary of `gsd/W317` (`passes/scope.rs`) |
| `gap-analysis-plan-post-disabled.json` | no `gsd/W317` with `workflow.post_planning_gaps` off (`tests/cli/parity_plans.rs`) |
| `decision-coverage-plan-{uncovered,all-covered,no-trackable,disabled}.json` | `gsd/E310`: the uncovered decision ids, and none for the other three (`tests/cli/parity_plans.rs`) |
| `verify-command-paths-phase-{findings,clean}.json` | `gsd/E320`: the blocker rows (`tests/cli/parity_plans.rs`) |
| `verify-failure-directions-phase-{findings,clean}.json` | `gsd/E507`: the `missing` rows (`tests/cli/parity_plans.rs`) |

## `agents/`: the 15 core agents

The 15 `@gsd/core` agents of `gsd/core/contributions/README.md` (the Agents table), copied unchanged by `git -C
<gsd-core> show v1.18.0:agents/<file>`: `gsd-<name>.md` for each, and `gsd-<name>.compact.md` for the ten that have one
(all but executor, phase-researcher, plan-checker, planner and verifier). `gsd/core/tests/port.rs` compares each with
the port under `gsd/core/assets/agents/` line for line: a line may differ only where upstream's names `.planning/`,
`gsd-tools`, `gsd_run` or `gsd-sdk`. Both copies are MIT, Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).

## `src/complexity-trigger.cts` and `tests/complexity-trigger.test.cjs`: the complexity scorer

The refactor trigger's decision-point scorer and its own tests, copied unchanged by `git -C <gsd-core> show
v1.18.0:src/complexity-trigger.cts` and `…:tests/complexity-trigger.test.cjs`. `gsd/core/src/commands/complexity/`
ports `stripLiterals`, `scanFunctions`, `analyzeSource`, `isAnalyzablePath` and `evaluateCandidates`, and
`complexity/tests.rs` ports the test cases that exercise them, with the same source text and expected scores; it names
the cases it leaves out (the baseline file, the proposal file and the JavaScript module's exports). Both files are MIT,
Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).

## `src/clusters.cts` and `tests/clusters.test.cjs`: the skill clusters

The ten named groups of skill stems `/gsd:surface` enables and disables, and their own tests, copied unchanged by
`git -C <gsd-core> show v1.18.0:src/clusters.cts` and `…:tests/clusters.test.cjs`. `gsd/core/src/settings/surface.rs`
ports `CLUSTERS` (names, order and members) and declares one `specforge.surface.<cluster>` setting for each;
`tests/cli/cmd_surface.rs` pins the port against the vendored file. Both files are MIT, Copyright (c) 2026 Open GSD
(`gsd/core/LICENSE-GSD`).

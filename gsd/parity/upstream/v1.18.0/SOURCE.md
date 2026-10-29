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

<!-- r42 workflows begin -->
## `gsd-core/`, `commands/gsd/`: the lifecycle workflows (r42)

The workflows, references, templates and command files the 16 lifecycle prompts of `@gsd/core` port, copied unchanged
by `git -C <gsd-core> show v1.18.0:<path>`, at the path they have in the tag. `gsd/core/assets/<path>` is the port
(`gsd/core/tests/port_workflows.rs` compares each with its copy here, line for line: a line may differ only where
upstream's names `.planning`, `gsd-tools`, `gsd_run` or `gsd-sdk`, or where a relative link left the tree and became the
absolute URL of the file at the tag). A file whose relative links leave this tree, or reach one that does, is kept as
`<name>.md.txt`, so the repository's link check does not read it. The command files keep their frontmatter here; the
port drops it, because the slash binding carries it. All files are MIT, Copyright (c) 2026 Open GSD
(`gsd/core/LICENSE-GSD`).

`src/runtime-artifact-conversion.cts` is the source of the host adapters `gsd/core/src/prompts/host.rs` ports (Codex's and
Cursor's text rewrites and adapter headers), and `tests/cursor-conversion.test.cjs` and `tests/runtime-converters.test.cjs`
are the tests whose vectors `host/tests.rs` runs through the port.

| File | sha256 |
|---|---|
| `gsd-core/references/agent-skills-bootstrap.md.txt` | `0ad5db82dd8bda51bdb367c3abba2a360846a3f1bcfc3efbc8d546b7b38a879b` |
| `gsd-core/references/doc-conflict-engine.md` | `aab20aa678c51b617d9f3be6b909c26105d95460400c5c8942191e89e7c1f919` |
| `gsd-core/references/failing-direction.md` | `6f91f97e2508f76a44d4633a783f651c9c700ece7cea0e8202b13fdfb39a8cce` |
| `gsd-core/references/few-shot-examples/plan-checker.md` | `aac7d938bb4ed6d4087dd15897f13f38bd588731db4c8358f8991a1557e1a221` |
| `gsd-core/references/few-shot-examples/verifier.md` | `5badee4560b14ae8c88bff81749a7d16b506b73de2b3e4092191cd08d1f14a32` |
| `gsd-core/references/mandatory-initial-read.md` | `fe59abce693717cf4e55c2050d28c9976c5d9e0499ac5a4f73c7979599c3b443` |
| `gsd-core/references/nyquist-compliance.md` | `513417f4095b4e75d1c0b903e6fab20704faa103ca095b32c3f3d4d7b81612c4` |
| `gsd-core/references/plan-checker-examples.md` | `6e20d4d2c7fe88a85825769a597176b2b8ab156361657a0da5acf368e5045fc7` |
| `gsd-core/references/planner-chunked.md` | `5c0a6371048cede46a475ee444f7a47117cd8b7f9209ffdc43a17e702130681f` |
| `gsd-core/references/planner-coupling.md` | `27a8e4553981282a0c60397c5ec94c0d0663c1f11347f5093fe3ef7091135480` |
| `gsd-core/references/planner-interface-context.md` | `b28fa3da6ae739a81de4287e3fe893133e46ec406a8ee73355ec46145a9d9b15` |
| `gsd-core/references/planner-load-graph-context.md` | `716edc9ed70118856f0280a3b78727e6f8a810ace5f0abd42d3497cfe2bb4ae9` |
| `gsd-core/references/planner-quick-batch.md` | `517d562bdc4aa2fbabd831ae363c67a6e58097d302acf2cbb02b198a591a577c` |
| `gsd-core/references/planner-source-audit.md` | `7de5bdb07232ce0b1a9f9217164e1635de30c09cc129f3fc1d10f44a9de7a704` |
| `gsd-core/references/planner-verify-command-grounding.md` | `7a1c7d108ea31d52874d3b9aed62da4e6caaa1e7f54429d1582631f9efdb3544` |
| `gsd-core/references/project-skills-discovery.md` | `c155e03dce8dc3c2606e93bac6497f6a877209e17f8a4b2a1f0e868a8e992d50` |
| `gsd-core/references/research-documentation-lookup.md` | `da9b532d11aeaeb3e8ab4079a50ef92165dd0e98e83e425aa22ddcf29df71e81` |
| `gsd-core/references/research-philosophy.md` | `62930e66cc979c1a0f9870f1f7725365467f0393ce50780aa5d789ece0ad1b07` |
| `gsd-core/references/research-verification-protocol.md` | `9c38c9d9a687e67914c85c9e7e47c8a23366abbee90607065b5398255c2ae572` |
| `gsd-core/references/security-asvs-levels.md` | `4774fac3b94b6ca85dace3995fc6942cd81fd9a4918cfb487c6f3d78b5870434` |
| `gsd-core/references/thinking-models-execution.md` | `dcc650a8b5f3e0495085a2935d2a6420d8c70a6ad8586e1539058316540e2978` |
| `gsd-core/references/thinking-models-research.md` | `5f6bf3f3b889c6e485c88b25cf91494172b9f1f66222372663db2a2c06a505cd` |
| `gsd-core/references/thinking-models-verification.md` | `a71a933d51ca3d8dd2534e27ae93d6148a5ea8b66e37a5e61b879cff25da31ac` |
| `gsd-core/references/verification-overrides.md` | `8213de9bd62283b60065db2e7b715f537d8304e7053dd4a7701f4abfa00f4f55` |
| `gsd-core/references/verification-patterns.md` | `085d3a07d7382a8e94d66ff112276e3adb30d8ee17bb568dc1d9f0a27a56efa7` |
| `gsd-core/references/verifier-evidence-gate.md` | `dcfb6700cdbd667631c1d5c4a480442a0655120cfb5260c5360950136354f27b` |
| `gsd-core/references/verifier-phase-gates.md` | `0e922a8c361b2aadf637ef5e3692b02505eb8b966b80719470e67345ec372a0e` |
| `gsd-core/references/verifier-wiring-patterns.md` | `9f0d2405f4cf6dca66fcd2e28c711bfd7fb3e565a181d9ebbf8bf923e25917de` |
| `gsd-core/references/verify-command-path-resolvability.md` | `0e87f599b538d0ba72d6eff39eafbc1d6702636e771425e05b436d199361318d` |
| `gsd-core/templates/roadmap.md` | `e4e35a9eb5dd4d4f2b4aed28ca6896c5bf4d652ad565f698325a56a7e840694f` |
| `gsd-core/templates/state.md` | `03d21c7d9e571d2684beaf00198d4f371832a4770255d52dab578973cbb4a0a9` |
| `gsd-core/templates/verification-report.md` | `930e2f288db2429375e97e14334545a85ec9bf2fc493147c817da11abc4b2488` |
| `src/runtime-artifact-conversion.cts` | `9b84125146270c5ecfc6f8f530f2ca01883b0239497990e91834a4b4939cad6c` |
| `tests/cursor-conversion.test.cjs` | `c787bcff4ff452f17459caed63706a11ecc3d8cbc2eb728d20dfcdc448da0e6b` |
| `tests/runtime-converters.test.cjs` | `9b481753590f9a0a48d3a76136ae03d27cb16029d4576c7f2711342f2f6f2263` |
<!-- r42 workflows end -->

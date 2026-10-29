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
| `commands/gsd/add-tests.md` | `d62d2e34817c899a92998903b26f427005d93138806f3a98d8112c671fe3edcb` |
| `commands/gsd/audit-milestone.md` | `64fd4df436242de2e7bb064561b31c60418672620876016aaee03be45ce11b79` |
| `commands/gsd/audit-uat.md` | `a7ce7c63bf2db68eb2f87bd76d3d135c1189318e2ed43d395dcee5d95f8661b8` |
| `commands/gsd/complete-milestone.md` | `bde2999b8492c260b646b321e688b6da16d577dec65019f3f5fcd11dfdfc301c` |
| `commands/gsd/discuss-phase.md` | `6302cf0b4afa6d8df67f5322a1d80b56eadb28f9f3d55e3e25b3ae89b83bdf22` |
| `commands/gsd/execute-phase.md` | `ce00d2d18ce57594855bd1b15f9c7854d33e047e2782952cd05bd11354300d4d` |
| `commands/gsd/mvp-phase.md` | `f4403d0841bfa0046ffe311bb70827c97b6b208db4f1583d659558b70f600f2c` |
| `commands/gsd/new-milestone.md` | `5bd5d02468791c6b50c70e0b50a37807e8d362634e480d26920cb1986dbcb0fd` |
| `commands/gsd/new-project.md` | `3ccfcba83d400d996db7f73a045e9138a7322ee372e516ccbc96ec33971407fd` |
| `commands/gsd/plan-phase.md` | `03c2ea26a9f4c480fa587bd647551399156c419803dcdf6ad9de259adcaeacf0` |
| `commands/gsd/plan-review-convergence.md` | `4ca4073a5d2574d1ca155334136760ea109f0945dff5104f8938ee5c072227c4` |
| `commands/gsd/ship.md` | `b63cc58605b7ccaee6cca771fa68c010f14433c5dbf69567aa748cc6e3b7b68e` |
| `commands/gsd/spec-phase.md` | `99120491c57e274edb56027c5a9573cc6fd6da9fa37304816fda69bbaabebeb1` |
| `commands/gsd/ultraplan-phase.md` | `6ce468cd97f99a3d81a7db17baba2ae20858972e1f0090b621e56d58d9a76cb8` |
| `commands/gsd/validate-phase.md` | `47da3f45f5fb517864921413d06e824dda19ba9272e34192d5c7d5ca1eea73d1` |
| `commands/gsd/verify-work.md` | `9c39dbeceff668986060da04e49ca63a66a8b89b1b3276963814b657e887a084` |
| `gsd-core/references/agent-contracts.md` | `b07ecbfe64411229ec0760e1f6cb5b05ad0f2493ca688f87d258c4b0cd7d6597` |
| `gsd-core/references/agent-skills-bootstrap.md.txt` | `0ad5db82dd8bda51bdb367c3abba2a360846a3f1bcfc3efbc8d546b7b38a879b` |
| `gsd-core/references/checkpoints.md` | `dc5f1f03e9fc812538dd257f620c74fadc7213b6fcd871323d797137bf6c5eb2` |
| `gsd-core/references/compact-content-gate.md` | `30b27c97f5aa9d650a3302c484a3c800934a8b32aa9c57a087199d4f5c4a26e0` |
| `gsd-core/references/context-budget.md` | `0376465732822d601aaedf7073c5b8cae1cab110a256c7ef995405b6874d90c4` |
| `gsd-core/references/dispatch-isolation-gate.md` | `8c1ca903dd235dc309ce9b477c4ab957721d0c90fa89d9432f3a23529075d85f` |
| `gsd-core/references/doc-conflict-engine.md` | `aab20aa678c51b617d9f3be6b909c26105d95460400c5c8942191e89e7c1f919` |
| `gsd-core/references/domain-probes.md` | `62d23ed1992c48a93b96585c407c8ba3b0b0b76f5f8b7122a0402df17a154392` |
| `gsd-core/references/edge-probe.md` | `d28899e29723d906af53a1c4e0bcf6594714149769b96084edb862fc401cee5d` |
| `gsd-core/references/execute-mvp-tdd.md` | `a7f7eade8e84313e72bd5b3161d1f3956c88a22c940d6fe3bdfaa13d22d1d211` |
| `gsd-core/references/execute-phase-between-wave-reset.md` | `bca7832658633a2bff35e39eb733b4b96a5dd6b02c40d8328065d5ca1a4084b0` |
| `gsd-core/references/execute-phase-context-guard.md` | `447dbe3d1363579a6640e623d46fddf6a2e394f2d674a8254b20892aa35faba5` |
| `gsd-core/references/execute-phase-quota-recovery.md` | `c4ca0ee3875b6e0270501460f72fa9901c6e6d7ccbc59bf0404a438b6520fc54` |
| `gsd-core/references/execute-phase-requirement-revert.md` | `48ade76866c57ce21ffbce88b38a7ef7bfef7a10d6f9fc21577d3fdbc717cfbe` |
| `gsd-core/references/execute-phase-response-language.md` | `ef2fd7cd7b7cefa24448d262a019bbcdc34b42f06a9582bea3680c7e6443c5cb` |
| `gsd-core/references/execute-phase-wave-guard.md` | `603fc49c18aa76a05a50544d7482e639ee9039872c51a7014b78b4239ec34fa6` |
| `gsd-core/references/executor-examples.md` | `2de8e6a99861e9b12e17d9dd6ffc37b7340274784dbc3b020e3fbc0bf900be45` |
| `gsd-core/references/failing-direction.md` | `6f91f97e2508f76a44d4633a783f651c9c700ece7cea0e8202b13fdfb39a8cce` |
| `gsd-core/references/few-shot-examples/plan-checker.md` | `aac7d938bb4ed6d4087dd15897f13f38bd588731db4c8358f8991a1557e1a221` |
| `gsd-core/references/few-shot-examples/verifier.md` | `5badee4560b14ae8c88bff81749a7d16b506b73de2b3e4092191cd08d1f14a32` |
| `gsd-core/references/gate-prompts.md` | `c49339fde70d7330f1407466a90e50bb9edeb39a540b4564e3e283f2e47aedd9` |
| `gsd-core/references/gates.md` | `bd79c7f90c8cb8d702b4893fb312f82679050824a1fab0fb1f2f4dc22b58ec97` |
| `gsd-core/references/git-integration.md` | `7003ad238365632b6577a8635c3d0037b0e948d31e86b971444f2bc0068e64ea` |
| `gsd-core/references/gsd-run-resolver.md` | `2c7513820693ac7721f1dcab3f10c816165277959d765d73390f3f6b4fdab9be` |
| `gsd-core/references/honest-verifier.md` | `601aab061a9d5a3260b44e62ee6a5fb8918ddc8a665a4b24b0b83c8e578c878b` |
| `gsd-core/references/ios-scaffold.md` | `5ef0cb7e0fac891f092e5faef305b4abdc8197ef1a522579eca5edfaefee6ec0` |
| `gsd-core/references/loop-hook-dispatch.md.txt` | `b23dd942640b4fc52e01cec2af086363073d9917d323116bc09613fa6d270b34` |
| `gsd-core/references/mandatory-initial-read.md` | `fe59abce693717cf4e55c2050d28c9976c5d9e0499ac5a4f73c7979599c3b443` |
| `gsd-core/references/model-profile-resolution.md` | `d503cd3575b8a5becc6799ea7f351af732ff4b196bdacfab2a11a105b7355521` |
| `gsd-core/references/model-profiles.md` | `c34795b6dab1f3290db60f50886a989a85ef450a4e1eb8cbf015141af817ac08` |
| `gsd-core/references/mvp-concepts.md.txt` | `1d6ce95b160353231114f5cc3aecf99262df34973c7e2e923ffb3c26e80ae56e` |
| `gsd-core/references/nyquist-compliance.md` | `513417f4095b4e75d1c0b903e6fab20704faa103ca095b32c3f3d4d7b81612c4` |
| `gsd-core/references/phase-argument-parsing.md` | `7b36cc7b4458634e46ffbf5e0484c03237fbb101b0ce9d3b6fa5f8cf4bdd4bfb` |
| `gsd-core/references/plan-checker-examples.md` | `6e20d4d2c7fe88a85825769a597176b2b8ab156361657a0da5acf368e5045fc7` |
| `gsd-core/references/planner-antipatterns.md` | `23495282315d14a99d849ddbe87744e4a8b1be8d5fe68515b626c2312635fbef` |
| `gsd-core/references/planner-chunked.md` | `5c0a6371048cede46a475ee444f7a47117cd8b7f9209ffdc43a17e702130681f` |
| `gsd-core/references/planner-coupling.md` | `27a8e4553981282a0c60397c5ec94c0d0663c1f11347f5093fe3ef7091135480` |
| `gsd-core/references/planner-failing-direction.md` | `1d1c5075bd7bf9bfd376b5e6ab99bed66b4d9a69467964c5bedc1fdfb7e8b2c9` |
| `gsd-core/references/planner-gap-closure.md` | `6d773b09c373eab5a53bb92f8a108fe394ed96fea1aff7074ee6080900500a9f` |
| `gsd-core/references/planner-guidance.md` | `4d307cecacab29af4ace14708680472a47514fa283e465bff6dabfc67f8290cf` |
| `gsd-core/references/planner-interface-context.md` | `b28fa3da6ae739a81de4287e3fe893133e46ec406a8ee73355ec46145a9d9b15` |
| `gsd-core/references/planner-load-graph-context.md` | `716edc9ed70118856f0280a3b78727e6f8a810ace5f0abd42d3497cfe2bb4ae9` |
| `gsd-core/references/planner-mvp-mode.md` | `969e533bdf7942ea04573340139d00bdd8e03f4ee1fc09e53d348c452688c5a3` |
| `gsd-core/references/planner-preconditions.md` | `54f15f89c7046c9ef96931083343ad62bc00e02be0b85a572bb7dec939104d89` |
| `gsd-core/references/planner-quick-batch.md` | `517d562bdc4aa2fbabd831ae363c67a6e58097d302acf2cbb02b198a591a577c` |
| `gsd-core/references/planner-reversibility.md` | `24b426adf49848135f469e10527cdc9565912a66c5c50d0e6271b2043d8e4042` |
| `gsd-core/references/planner-reviews.md` | `7747edaca4ff22c7872f8dc47be3319c4b052a17128229f3b2c59e56243ec9bb` |
| `gsd-core/references/planner-revision.md` | `2cfbbba57951578dc4f7427faadf816a36e6272b9bfa607c3688e5a25e44c9df` |
| `gsd-core/references/planner-source-audit.md` | `7de5bdb07232ce0b1a9f9217164e1635de30c09cc129f3fc1d10f44a9de7a704` |
| `gsd-core/references/planner-verify-command-grounding.md` | `7a1c7d108ea31d52874d3b9aed62da4e6caaa1e7f54429d1582631f9efdb3544` |
| `gsd-core/references/prohibition-probe-fixtures/01-streak-reminder/expected.json` | `f10df472f2846cc62779f4cb686d7abfc76e5b66571e4d13d84f80aced83408a` |
| `gsd-core/references/prohibition-probe-fixtures/02-clean-utility/expected.json` | `31e8a781eeffe02099947f62a0de445af72b07a2cf444bcfa882a8d234f10018` |
| `gsd-core/references/prohibition-probe-fixtures/03-multi-prohibition/expected.json` | `70a532a7cc1b6ae8b71ac16c959bcaba49261b2df550b7bc92dca669c2759459` |
| `gsd-core/references/prohibition-probe.md` | `97ba2a0372adb127aae901100e37412ab8c3be91427ae494ea4adbf33021dffc` |
| `gsd-core/references/project-skills-discovery.md` | `c155e03dce8dc3c2606e93bac6497f6a877209e17f8a4b2a1f0e868a8e992d50` |
| `gsd-core/references/questioning.md` | `a8c988cab05f4651f9b88b7e6217fc8569ba4748a795a47711ee8cf64b2c70b0` |
| `gsd-core/references/research-documentation-lookup.md` | `da9b532d11aeaeb3e8ab4079a50ef92165dd0e98e83e425aa22ddcf29df71e81` |
| `gsd-core/references/research-philosophy.md` | `62930e66cc979c1a0f9870f1f7725365467f0393ce50780aa5d789ece0ad1b07` |
| `gsd-core/references/research-verification-protocol.md` | `9c38c9d9a687e67914c85c9e7e47c8a23366abbee90607065b5398255c2ae572` |
| `gsd-core/references/response-language-directive.md` | `0beb50ccb19fa3489d3398c5280c745e1af96a9f7a8c60f370f70539cc949d5d` |
| `gsd-core/references/revision-loop.md` | `09cf2d9ac1821cd2f8b8a708aa52af53f1c09cded910cedde84fe8624bbb5967` |
| `gsd-core/references/runtime-aware-dispatch.md` | `125ff8314dc243a59bcae1d95c13939874bd7057d09a956e9aad54944473362c` |
| `gsd-core/references/scout-codebase.md` | `ba266ecc18fbf1720ba7f0caa410c9517c720686b01164149fc64104eeddf62c` |
| `gsd-core/references/security-asvs-levels.md` | `4774fac3b94b6ca85dace3995fc6942cd81fd9a4918cfb487c6f3d78b5870434` |
| `gsd-core/references/skeleton-template.md` | `f11e9cd2948bd33c26b709751ba0fe18ac0892b4f3e1fe209920c5ab1e22e42a` |
| `gsd-core/references/specless-probe-fallback.md` | `7aefa777b068907f579f16edf08ffb3907be3249bd40b36d057f93e50aed32cc` |
| `gsd-core/references/spidr-splitting.md` | `6c495fbfebdb42e75db9805bc97350b7e885ca8e822739c3a72a0075690c11e2` |
| `gsd-core/references/tdd.md` | `304be980cea7bb6bf6a16b1d239ae6e3b59c4ca1221066e7e0735077cbfa907e` |
| `gsd-core/references/thinking-models-execution.md` | `dcc650a8b5f3e0495085a2935d2a6420d8c70a6ad8586e1539058316540e2978` |
| `gsd-core/references/thinking-models-planning.md` | `7cc8aeaa693497d751fe52a20a045382edcd07a23efd296887c378754a42925b` |
| `gsd-core/references/thinking-models-research.md` | `5f6bf3f3b889c6e485c88b25cf91494172b9f1f66222372663db2a2c06a505cd` |
| `gsd-core/references/thinking-models-verification.md` | `a71a933d51ca3d8dd2534e27ae93d6148a5ea8b66e37a5e61b879cff25da31ac` |
| `gsd-core/references/thinking-partner.md` | `41069529ef776e391270c384ec392d61e911d0e793c9e7c923b154e6dd38bd5e` |
| `gsd-core/references/ui-brand.md` | `189bb64babb007a09d1610f0ee8013cdfd4c04b82428a334b68f3dddbc015c83` |
| `gsd-core/references/universal-anti-patterns.md` | `bacdc27e0718d0e26a6a1fffec10013dffbccd3f34501d456f6b5b1f2a04b8e0` |
| `gsd-core/references/untrusted-input-boundary.md` | `d33b80d4d348599a3e34074c295c091886509f6afa594926feeaad415cdfa606` |
| `gsd-core/references/user-story-template.md` | `0cc50e06a144ff8ac09b4fca7252cf2c42b53fe277058f230328aa020d5f71ce` |
| `gsd-core/references/verification-overrides.md` | `8213de9bd62283b60065db2e7b715f537d8304e7053dd4a7701f4abfa00f4f55` |
| `gsd-core/references/verification-patterns.md` | `085d3a07d7382a8e94d66ff112276e3adb30d8ee17bb568dc1d9f0a27a56efa7` |
| `gsd-core/references/verifier-evidence-gate.md` | `dcfb6700cdbd667631c1d5c4a480442a0655120cfb5260c5360950136354f27b` |
| `gsd-core/references/verifier-phase-gates.md` | `0e922a8c361b2aadf637ef5e3692b02505eb8b966b80719470e67345ec372a0e` |
| `gsd-core/references/verifier-wiring-patterns.md` | `9f0d2405f4cf6dca66fcd2e28c711bfd7fb3e565a181d9ebbf8bf923e25917de` |
| `gsd-core/references/verify-command-path-resolvability.md` | `0e87f599b538d0ba72d6eff39eafbc1d6702636e771425e05b436d199361318d` |
| `gsd-core/references/verify-mvp-mode.md` | `abc46246b08e1d929db1c4d69768888e7e2e9f505df337c154d4c4ae09453722` |
| `gsd-core/references/workstream-flag.md` | `8823e5ba7d2f612b42bfc56b22ab7aea095e73b2fa0323f9dde06e410a9e6e72` |
| `gsd-core/references/worktree-branch-check.md` | `4ee765889967c7a144d7871201f9de6e1dd745ebe98876e082699e13880a39d7` |
| `gsd-core/references/worktree-path-safety.md` | `3c96dae01422cbd13a830ea3ab222266802901713b12428c324ca2d902e75971` |
| `gsd-core/templates/UAT.md` | `19823ed0f9499ad5fca73832abdc71c07753eadb770409395175565333ed2597` |
| `gsd-core/templates/VALIDATION.md` | `e2281534271f86b7d3c62e833b3c3533b1251bb062596f859c1c4cb3050e550e` |
| `gsd-core/templates/context.md` | `69b01e7909ea3f661d5b0fcec5470314f74176aa5bd998939d80d74c03fd07d9` |
| `gsd-core/templates/milestone-archive.md` | `591b6decdc0c0e51fba1359ed015ed140b33d50a9dcf9c0dbe149d605e3e5f54` |
| `gsd-core/templates/milestone.md` | `74d2f750ae9f4a9c18feec3708d8f414c5b15148b22eb7da554dc2da87587711` |
| `gsd-core/templates/project.md` | `4f311fb1b05b823bd00c655f737b546b98bb1123e0b20ca673ff5a0a3b6e5cc1` |
| `gsd-core/templates/requirements.md` | `a44de4c2f146e473265777500951b12642553606b613168001ed2577d9e968d4` |
| `gsd-core/templates/research-project/ARCHITECTURE.md` | `746b9ef791d758b0222ca03e03d6da314f54c0d560966b5a3d34766b1553b1ea` |
| `gsd-core/templates/research-project/FEATURES.md` | `f2b800de5df91b0f567dbe85754be2bf40fe56cb62da5cf6748f7a3cfe24fd8f` |
| `gsd-core/templates/research-project/PITFALLS.md` | `3ef75fa768422eeca68f4411d1e058c1f447a23a23a43aaed449905940c0cf52` |
| `gsd-core/templates/research-project/STACK.md` | `82c85799ac4dd344441370e791f09563119f62843034b3a094876a476c2bd4e5` |
| `gsd-core/templates/research-project/SUMMARY.md` | `dceb2f346388839d9fce7c8de9ffff2354b8539880e5dadfd10fccfce0062997` |
| `gsd-core/templates/retrospective.md` | `03981e30dd760103c1ea91d31ad24810feb082a388b4231d3a03a2c8ca386c5d` |
| `gsd-core/templates/roadmap.md` | `e4e35a9eb5dd4d4f2b4aed28ca6896c5bf4d652ad565f698325a56a7e840694f` |
| `gsd-core/templates/spec.md` | `26d55bce940f0288cf1b9796b7b99374bc88f53491d445bfe8f8c88d8551900b` |
| `gsd-core/templates/state.md` | `03d21c7d9e571d2684beaf00198d4f371832a4770255d52dab578973cbb4a0a9` |
| `gsd-core/templates/summary.md` | `e7b240727bb2d9b5547a3de7a862d30c4a374690098d820295e3354698fc6f04` |
| `gsd-core/templates/user-setup.compact.md` | `b60869ee028f4c96651fe27baf6fc85167b4f4dafabcad8cd568e52b89099c79` |
| `gsd-core/templates/user-setup.md` | `07c227403d62a9e0cae43a7ee0f9db7d667383e42fefb799067da6b39db145c0` |
| `gsd-core/templates/verification-report.md` | `930e2f288db2429375e97e14334545a85ec9bf2fc493147c817da11abc4b2488` |
| `gsd-core/workflows/add-tests.md` | `7b4e1f2ec34c7c25158870ca7508344ac6ceb70b7221b089cb8c2e598fa33e45` |
| `gsd-core/workflows/audit-milestone.md` | `b16abf6ae3137c05502a02ed579df7e431459ebfec7224575ec7b28bcc061f7e` |
| `gsd-core/workflows/audit-uat.md` | `22b7973b45449828a99c993270f50e105922eee1fd43365e6a0a4dd0747d0492` |
| `gsd-core/workflows/complete-milestone.md` | `c1b4c7b2c26738fb6140df69b70ffda0f3698d53ffdfe4d5a73d0f17f44bae5f` |
| `gsd-core/workflows/complete-milestone/detail/elaboration.md` | `8298a3a9fe43cba23e9d518cf65b165c57574d83cc73c9c6a8a753ba1aea1261` |
| `gsd-core/workflows/complete-milestone/steps/git-tag.md` | `651ed29ec56dff1a7e0746b06cd9e61120c9d1a00c6feda188e10a12fb96ea8a` |
| `gsd-core/workflows/diagnose-issues.md` | `ccf520c63dda7f743ea83411da1f82141e10f4325e587b04e8237601d4f835f4` |
| `gsd-core/workflows/discuss-phase-assumptions.md` | `1ccc3c8d295c89542a94751215faa71eaffab34b03d20ab85051eb5e712191d4` |
| `gsd-core/workflows/discuss-phase-assumptions/steps/auto-advance-dispatch.md` | `b29510c8bb53cbb7ce057f2b2fc44a04b332026610d385a1a4104d4246da01f8` |
| `gsd-core/workflows/discuss-phase-power.md` | `f26697a5c55c39033f27738f7f500e2789a7d3023e7360ceddc22e9fbf35ed5c` |
| `gsd-core/workflows/discuss-phase.md` | `a9d9936e8f5ce9dbfd3115d9fde59d28abb6d933d78c3c6811ca0a9e24009655` |
| `gsd-core/workflows/discuss-phase/modes/advisor.md` | `cc6638b7b8ffb539afc71cfeec39e639f5902d8bc4138476b17b65c268eac514` |
| `gsd-core/workflows/discuss-phase/modes/all.md` | `c20f95c2c2e9f99f8a37fa292c91f745aa5b18444fe3a44bf6652ab6e323fd64` |
| `gsd-core/workflows/discuss-phase/modes/analyze.md` | `440ef488ba9190d1f3e85729ad1a703614a024effb057e681e76265df04c71eb` |
| `gsd-core/workflows/discuss-phase/modes/auto.md` | `eb1ce55f87047412006bdd40de3a22ea9c48374417d62dea11fdc37a4d5627a1` |
| `gsd-core/workflows/discuss-phase/modes/batch.md` | `9a73b49bc7f435cc4e027e5e7bc9d83fe3ccf1b1ba17dd4b770e20e5282fa8d0` |
| `gsd-core/workflows/discuss-phase/modes/chain.md` | `2545b5572db84f9f5567d5f33246931eebdd195f63597473c572161d2ce10437` |
| `gsd-core/workflows/discuss-phase/modes/default.md` | `d096bdc99b042ed336a7a937650d18cc498e74f19ddc3dd265c38b216a107ad4` |
| `gsd-core/workflows/discuss-phase/modes/power.md` | `004ae5256e47a85f30606b4d512bd8346653b0363e4d9712cc000bf68463e32c` |
| `gsd-core/workflows/discuss-phase/modes/text.md` | `9644af8abce4766d51abda6654251aa91557a78023f90c0ee60875e928dc8909` |
| `gsd-core/workflows/discuss-phase/templates/checkpoint.json` | `e3bc3dca49db59eb02d2461bb98a53c9ecac041aab377c19a6091d1a517ba186` |
| `gsd-core/workflows/discuss-phase/templates/context.md` | `323884311c94fbc280d0ebbfec7b906bc792e0d50926c4021a24c156eef0378d` |
| `gsd-core/workflows/discuss-phase/templates/discussion-log.md` | `e829d02ec03db8bbc4157845e52283074ef57b9491c04f73833f2cb0876456df` |
| `gsd-core/workflows/execute-phase.md` | `000c640ff7b4af7f21fec8de150481519864392ebc3772e344194ad4bb4ec90e` |
| `gsd-core/workflows/execute-phase/detail/elaboration.md` | `6e414bf165124160134f99e2e585898be6252d573248be0d55b645e2d9817e66` |
| `gsd-core/workflows/execute-phase/steps/code-review-disposition.md` | `df4b87fed091479a103f03bb5c6ed1bd96b2e3ab2f56631449127bcacc4fccdb` |
| `gsd-core/workflows/execute-phase/steps/codebase-drift-gate.md` | `20216656225493d695102aec801bea749a16c386e3705aab56067308b65c1608` |
| `gsd-core/workflows/execute-phase/steps/completion-reconciliation.md` | `87ffd2a105aea8254b83e7687146b5b79437665fde3c67d8b4f0beb989115909` |
| `gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md` | `78b6522cb622951fc051ea199468b3e64af4a560335422eca3880c0cf1fe6f93` |
| `gsd-core/workflows/execute-phase/steps/executor-progress-policy.md` | `cbddeca12096b81f6a0be38022564ef4cd2b6676bc38b48ed21c1e9de3320abf` |
| `gsd-core/workflows/execute-phase/steps/gap-closure-artifacts.md` | `360387bd34e45b125eab452e011793c4b0b703c7d1960cc1f7c2cf865b53b490` |
| `gsd-core/workflows/execute-phase/steps/partial-wave.md` | `e4f2c7fabcce21ef1ae880d220c62202ce9506afeb7993f44da267448b2054ad` |
| `gsd-core/workflows/execute-phase/steps/per-plan-executor-routing.md` | `ceb4a7a8dd8af6591531152618b4ecc1dd50911ba58890d67764f4f03f035444` |
| `gsd-core/workflows/execute-phase/steps/per-plan-worktree-gate.md` | `b371f5d62d54a9946e45e7bc8dc807b6e194fca38866dd04a6636d916d424356` |
| `gsd-core/workflows/execute-phase/steps/post-merge-gate.md` | `665840aea60d8d86be9ae820874c4d6fd63b609ad77e646a25648046bba3a2fc` |
| `gsd-core/workflows/execute-phase/steps/protected-branch.md` | `19289f31d53b876c24a989aec2e1a20b0d01be566274dd535cf56b4ce7a6e862` |
| `gsd-core/workflows/execute-phase/steps/ready-wave-gate.md` | `2e02627868bbe696c09ff46c3b09f91d5bfa4ef143bf8f31caa83d772ef7d6e4` |
| `gsd-core/workflows/execute-phase/steps/regression-gate-run.md` | `5fe18424069df11b0b0eb362bcac7b765ea253ef43b196510d4cbdde0b55c376` |
| `gsd-core/workflows/execute-phase/steps/regression-gate.md` | `a788acd6b87c6643684498c6edfdf60c43386915c38434669a4450725bc6928d` |
| `gsd-core/workflows/execute-phase/steps/sequential-root-pin.md` | `7e8eab41db616d0386a18eb96f1e8934d95b58f58a5952d55f1d003021c5bc2b` |
| `gsd-core/workflows/execute-phase/steps/tdd-applicability-resolution.md` | `d7db442ecc1b15d13fe910b1655d35552b13cb912a573c1f52a513e1040bec83` |
| `gsd-core/workflows/execute-phase/steps/threat-id-gate.md` | `7fcba1e23e636eb668933d7f5e09ce6bcc7c15b2cb6749471cbc53a0f99921cf` |
| `gsd-core/workflows/execute-phase/steps/verify-phase-goal.md` | `85f963e8dfec8a7054b9b34a50ea7046cf3d20f766768b1a4e25d547455f813e` |
| `gsd-core/workflows/execute-phase/steps/wave-post-gate-hooks.md` | `fe38ced810fbedabfa8034abd997437abc6ed40555888ba775f3cd4dbf3d0a72` |
| `gsd-core/workflows/execute-phase/steps/worktree-base-check.md` | `3df47249c0c69982ed4b9328b3ebff3e90a9eb17c25fe5868162f35c9a9d8dbe` |
| `gsd-core/workflows/execute-phase/steps/worktree-recovery-policy.md` | `0a909cd705c221d892a42e1c21ac89ec6e6247ea14ea0706e4db73c3b5aee179` |
| `gsd-core/workflows/execute-plan.md` | `158bbcf874f32b41afc1b8f87f7bf7e541520263dd07a8ec097631dee315a2ee` |
| `gsd-core/workflows/graduation.md` | `cefba34d6b17d2bafbaabd3dacaeac9e1e2fd90585f21f7fcb50888b6e06d3b0` |
| `gsd-core/workflows/list-phase-assumptions.md` | `db6946da4d25256a4bfb016b1465101f553aa0a8f7918f9078d76492e6ee20ee` |
| `gsd-core/workflows/mvp-phase.md` | `b2d7e923b9ede748efb3a36e06603aa780557819e4ffa8f8dfa97dfa68526442` |
| `gsd-core/workflows/new-milestone.md` | `d2300de5139fe176a4a7172cab5d1f65384caa267e46de1afb7a0a8009bbca68` |
| `gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md` | `7fbacae20957818a907edffb07f7552de2a9a023add2463f237a9173e3db4414` |
| `gsd-core/workflows/new-milestone/steps/reset-phase-safety.md` | `0a8e18540587a14cb9981d35f62a9f4df6c726538bd2d3786a1f5cd3f2ef2f28` |
| `gsd-core/workflows/new-project.md` | `7d104af278f9bbcb6c0b9ee0ee91e1056346b770ddf406a89bb5b652df3ecc9c` |
| `gsd-core/workflows/new-project/detail/elaboration.md` | `31d2072b17423737395afcba4794d79f2077b52f2b8917470ee57b5cd53bd25f` |
| `gsd-core/workflows/new-project/steps/auto-mode-config.md` | `a4d8ed52fd6a1d5ebfaedc41030e98443efc58367d97eb9ea4577dc155527bad` |
| `gsd-core/workflows/new-project/steps/auto-mode-detection.md` | `55a6deec1f4e30433e27d976db5dca04c7d5e771c6a960a73148f1f86db1b862` |
| `gsd-core/workflows/new-project/steps/codebase-map-offer.md` | `21b0e14926c29422891eb996c69d81d9bfa28291e6f5b3a59cbf991e25b74442` |
| `gsd-core/workflows/plan-phase.md.txt` | `1d22890f04337ecd6562a2e2346e93be305736445c3bad13a0b9ad900c5d2457` |
| `gsd-core/workflows/plan-phase/detail/elaboration.md` | `1185427996718e6b91eb3fb19ff16a6761d8960a45eee3a8dd5c2063acfa981f` |
| `gsd-core/workflows/plan-phase/steps/adr-ingest-express-path.md` | `b188047653fdb7ceccbbd110a1cb4406c2f899b29d12f0effc2e61b062be0493` |
| `gsd-core/workflows/plan-phase/steps/auto-advance-check.md` | `54a7c8e641617da91877553e37a337120027e4b66675ef948d5d610b30f0a998` |
| `gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md` | `4ebf3b9fb5a7b1d7b57a2f07f0b96a1246547c2e909cace7bbf575d9bf60192e` |
| `gsd-core/workflows/plan-phase/steps/closed-phase-gate.md` | `3cbe49ebeb202bfa596977264cffb912930d0073fd684b8a66c329a4318c5009` |
| `gsd-core/workflows/plan-phase/steps/prd-express-gate.md` | `110a18f8d4f5a9041e9732dee674a6f6c961e96f5151d74d23e8f0dcefb7d23f` |
| `gsd-core/workflows/plan-phase/steps/prd-express-path.md` | `9b08c5995db64a3160a54c688e1f8eb1b594d648023ae8ef80741012e92afa92` |
| `gsd-core/workflows/plan-phase/steps/research-only-early-exit.md` | `0a8e2fef954af151f6a49492d5e0bae9cd619bb41e9593c93eaa4383fa346873` |
| `gsd-core/workflows/plan-phase/steps/research-only-modifiers.md` | `eff31aab435712600f536cedbe74bd565b5c590909bb9bd97697d27ce9a46817` |
| `gsd-core/workflows/plan-phase/steps/reviews-prerequisite.md` | `d0d084ad6a1fea566643a476c3dc6e46d85c49075b4a10cf991553c9ee539f02` |
| `gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md` | `9b9d040fa38e35ce147ba712268fede1f4df3367d414d5380bb66d61dae1915e` |
| `gsd-core/workflows/plan-phase/steps/windows-troubleshooting.md` | `49f58c3f75be3eb5e607b572fd28b1969faa0b47e837c3d9255df36ffbafe80b` |
| `gsd-core/workflows/plan-review-convergence.md` | `0ffef2e4ae27966f485326f017312a42ed153d8da0f29cf49abb81ac32393798` |
| `gsd-core/workflows/ship.md` | `20b492e7404ad6e5eb52e9f63e8a6aa1a43a57e8b89ba04660e6408be04722f3` |
| `gsd-core/workflows/spec-phase.md` | `9a05d48ae07f40f373c2bf02321094b9d64e66329d63310851accebc7e16dd00` |
| `gsd-core/workflows/transition.md` | `9b8a135f54ddbc9de3e48a69686e2e46ed5a8e49517b33251dcdb00c90119041` |
| `gsd-core/workflows/transition/steps/workstream-collision-check.md` | `ddf53acd9512bd2b6caab2c45bff4da585f8a2976bba3d4fe7e6092265e7824c` |
| `gsd-core/workflows/ultraplan-phase.md` | `58512036c7dc6a70f1612854e5a9929cfa22c33039d5242133147daf744c2521` |
| `gsd-core/workflows/validate-phase.md` | `113db27abc3a6b8db0026a35429da15982092280b838a1bc602a08e1c69751a7` |
| `gsd-core/workflows/verify-work.md` | `8d93cbf259516fd7aa00cfea76e67c97057f14805e82696933b5cecc33cae204` |
| `gsd-core/workflows/verify-work/detail/elaboration.md` | `43cf5c123ca24a4394e5c3624d96ccf093438aba8dc4cb090d8bfc7206baa267` |
| `gsd-core/workflows/verify-work/steps/automated-ui-verification.md` | `6a2313603645606042261bbfc2af597eebb79ae6c638c10978e806e0e5921045` |
| `gsd-core/workflows/verify-work/steps/mvp-uat-framing.md` | `e540ff57945535fa64b5267ab32f09c1fda59442752514b18476a66da7a34558` |
| `src/runtime-artifact-conversion.cts` | `9b84125146270c5ecfc6f8f530f2ca01883b0239497990e91834a4b4939cad6c` |
| `tests/cursor-conversion.test.cjs` | `c787bcff4ff452f17459caed63706a11ecc3d8cbc2eb728d20dfcdc448da0e6b` |
| `tests/runtime-converters.test.cjs` | `9b481753590f9a0a48d3a76136ae03d27cb16029d4576c7f2711342f2f6f2263` |
<!-- r42 workflows end -->

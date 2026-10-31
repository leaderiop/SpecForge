# gsd-core v1.19.0: the configuration roster

Frozen so the roster check (`gsd/core/tests/settings_roster.rs`) runs offline and fails when the tracked tag moves
without the snapshot.

| File | Taken from |
|---|---|
| `config-schema.manifest.json` | `gsd-core/bin/shared/config-schema.manifest.json`, unchanged |
| `config-defaults.manifest.json` | `gsd-core/bin/shared/config-defaults.manifest.json`, unchanged |
| `capabilities.json` | the `config` block of each `capabilities/*/capability.json`, one entry per key: `{capability, type, default, values?}` |

- Repository: <https://github.com/open-gsd/gsd-core>, tag `v1.19.0` (commit `9dc57a772195de423ab788e4d0d8734b162db35d`).
- Licence: MIT, Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).
- The roster also names keys only `docs/CONFIGURATION.md` lists (`parallelization.*`, `dynamic_routing.*`,
  `worktree.baseRef`, `context_profile`, the tier defaults of `effort` and `fast_mode`); `settings_roster.rs` lists
  them by hand.
- Defaults come from `config-defaults.manifest.json`, then the capability's own default, then
  `docs/CONFIGURATION.md`. Where the docs disagree with the defaults manifest the manifest wins
  (`workflow.inline_plan_threshold`: docs 3, manifest 2).

## `gate-cutover/`: deterministic gate verdicts

Verdicts of gsd-core's deterministic `gsd-tools` gates, copied unchanged by `git -C <gsd-core> show
v1.19.0:tests/fixtures/gate-cutover/<file>.json`. The fixtures that produced them are built by
`tests/fixtures/gate-cutover/arms.cjs` (`dc`, `vp`, `gap`); `gsd/core/tests/projects/gate_cutover/` is their twin, and
the tests in `tests/cli/parity_plans.rs` and the passes' native tests reproduce the verdicts. r44 T10 vendors the rest
(171 in all; see "r44 T10" below).

| File | Read by |
|---|---|
| `gap-analysis-plan-post-enabled.json` | the table and summary of `gsd/W317` (`passes/scope.rs`) |
| `gap-analysis-plan-post-disabled.json` | no `gsd/W317` with `workflow.post_planning_gaps` off (`tests/cli/parity_plans.rs`) |
| `decision-coverage-plan-{uncovered,all-covered,no-trackable,disabled}.json` | `gsd/E310`: the uncovered decision ids, and none for the other three (`tests/cli/parity_plans.rs`) |
| `verify-command-paths-phase-{findings,clean}.json` | `gsd/E320`: the blocker rows (`tests/cli/parity_plans.rs`) |
| `verify-failure-directions-phase-{findings,clean}.json` | `gsd/E507`: the `missing` rows (`tests/cli/parity_plans.rs`) |

## `agents/`: the 15 core agents

The 15 `@gsd/core` agents of `gsd/core/contributions/README.md` (the Agents table), copied unchanged by `git -C
<gsd-core> show v1.19.0:agents/<file>`: `gsd-<name>.md` for each, and `gsd-<name>.compact.md` for the ten that have one
(all but executor, phase-researcher, plan-checker, planner and verifier). `gsd/core/tests/port.rs` compares each with
the port under `gsd/core/assets/agents/` line for line: a line may differ only where upstream's names `.planning/`,
`gsd-tools`, `gsd_run` or `gsd-sdk`. Both copies are MIT, Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).

## `src/complexity-trigger.cts` and `tests/complexity-trigger.test.cjs`: the complexity scorer

The refactor trigger's decision-point scorer and its own tests, copied unchanged by `git -C <gsd-core> show
v1.19.0:src/complexity-trigger.cts` and `…:tests/complexity-trigger.test.cjs`. `gsd/core/src/commands/complexity/`
ports `stripLiterals`, `scanFunctions`, `analyzeSource`, `isAnalyzablePath` and `evaluateCandidates`, and
`complexity/tests.rs` ports the test cases that exercise them, with the same source text and expected scores; it names
the cases it leaves out (the baseline file, the proposal file and the JavaScript module's exports). Both files are MIT,
Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).

## `src/clusters.cts` and `tests/clusters.test.cjs`: the skill clusters

The ten named groups of skill stems `/gsd:surface` enables and disables, and their own tests, copied unchanged by
`git -C <gsd-core> show v1.19.0:src/clusters.cts` and `…:tests/clusters.test.cjs`. `gsd/core/src/settings/surface.rs`
ports `CLUSTERS` (names, order and members) and declares one `specforge.surface.<cluster>` setting for each;
`tests/cli/cmd_surface.rs` pins the port against the vendored file. Both files are MIT, Copyright (c) 2026 Open GSD
(`gsd/core/LICENSE-GSD`).

<!-- r42 workflows begin -->
## `gsd-core/`, `commands/gsd/`: the lifecycle workflows (r42)

The workflows, references, templates and command files the 16 lifecycle prompts of `@gsd/core` port, copied unchanged
by `git -C <gsd-core> show v1.19.0:<path>`, at the path they have in the tag. `gsd/core/assets/<path>` is the port
(`gsd/core/tests/port_workflows.rs` compares each with its copy here, line for line: a line may differ only where
upstream's names `.planning`, `gsd-tools`, `gsd_run` or `gsd-sdk`, or where a relative link left the tree and became the
absolute URL of the file at the tag). A file whose relative links leave this tree, or reach one that does, is kept as
`<name>.md.txt`, so the repository's link check does not read it. The command files keep their frontmatter here; the
port drops it, because the slash binding carries it. All files are MIT, Copyright (c) 2026 Open GSD
(`gsd/core/LICENSE-GSD`).

`src/runtime-artifact-conversion.cts` is the source of the host adapters `gsd/kit/src/prompt/host.rs` ports (Codex's and
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
| `gsd-core/references/git-integration.md.txt` | `b1ea613171c3262ee7cb4b948f086998a051484592a5c069c423cff635725470` |
| `gsd-core/references/gsd-run-resolver.md` | `2c7513820693ac7721f1dcab3f10c816165277959d765d73390f3f6b4fdab9be` |
| `gsd-core/references/honest-verifier.md` | `601aab061a9d5a3260b44e62ee6a5fb8918ddc8a665a4b24b0b83c8e578c878b` |
| `gsd-core/references/ios-scaffold.md` | `5ef0cb7e0fac891f092e5faef305b4abdc8197ef1a522579eca5edfaefee6ec0` |
| `gsd-core/references/loop-hook-dispatch.md.txt` | `5e2e840337db4b5d083f8ff9e60ad50a99d049977747c0a1c54cffb690385dff` |
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
| `gsd-core/references/tdd.md` | `506ee9acbf7cebbcb8c23279906e81a85f967115946e52bce838197951c5ffe4` |
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
| `gsd-core/templates/verification-report.md` | `49d7842d10aa378a71491720bd1fbda593e8f9aa47b8d38876094901ab33bc76` |
| `gsd-core/workflows/add-tests.md` | `7b4e1f2ec34c7c25158870ca7508344ac6ceb70b7221b089cb8c2e598fa33e45` |
| `gsd-core/workflows/audit-milestone.md` | `b16abf6ae3137c05502a02ed579df7e431459ebfec7224575ec7b28bcc061f7e` |
| `gsd-core/workflows/audit-uat.md` | `22b7973b45449828a99c993270f50e105922eee1fd43365e6a0a4dd0747d0492` |
| `gsd-core/workflows/complete-milestone.md` | `c1aaf01cbd3aaf87019f170d52f3393e5e8a330991478dc93333ddb934665244` |
| `gsd-core/workflows/complete-milestone/detail/elaboration.md` | `8298a3a9fe43cba23e9d518cf65b165c57574d83cc73c9c6a8a753ba1aea1261` |
| `gsd-core/workflows/complete-milestone/steps/git-tag.md` | `651ed29ec56dff1a7e0746b06cd9e61120c9d1a00c6feda188e10a12fb96ea8a` |
| `gsd-core/workflows/complete-milestone/steps/project-md-evolution-review.md` | `6b4e900d717536b6d9ab9e8c034de74d02b140c69fd06042513ac8405879fd86` |
| `gsd-core/workflows/diagnose-issues.md` | `71510c99a61ea36f0eee899c9b2d30263ca8b9f7c9fa9f9ea6b3b2c353afe427` |
| `gsd-core/workflows/discuss-phase-assumptions.md` | `28184311a201291a02a6684a6b57d2e8347500ecffadee53859f01b3feb572b9` |
| `gsd-core/workflows/discuss-phase-assumptions/steps/auto-advance-dispatch.md` | `b29510c8bb53cbb7ce057f2b2fc44a04b332026610d385a1a4104d4246da01f8` |
| `gsd-core/workflows/discuss-phase-power.md` | `60157b753a368d7715d9ab6167703b89120d7e79d02014270a96cdba01876312` |
| `gsd-core/workflows/discuss-phase.md` | `1b3e5faf07c4f63e7b5c579b77f5edf311d1aa0b2eb5cd14d6b837a21975bc13` |
| `gsd-core/workflows/discuss-phase/modes/advisor.md` | `cc6638b7b8ffb539afc71cfeec39e639f5902d8bc4138476b17b65c268eac514` |
| `gsd-core/workflows/discuss-phase/modes/all.md` | `c20f95c2c2e9f99f8a37fa292c91f745aa5b18444fe3a44bf6652ab6e323fd64` |
| `gsd-core/workflows/discuss-phase/modes/analyze.md` | `440ef488ba9190d1f3e85729ad1a703614a024effb057e681e76265df04c71eb` |
| `gsd-core/workflows/discuss-phase/modes/auto.md` | `eb1ce55f87047412006bdd40de3a22ea9c48374417d62dea11fdc37a4d5627a1` |
| `gsd-core/workflows/discuss-phase/modes/batch.md` | `9a73b49bc7f435cc4e027e5e7bc9d83fe3ccf1b1ba17dd4b770e20e5282fa8d0` |
| `gsd-core/workflows/discuss-phase/modes/chain.md` | `2545b5572db84f9f5567d5f33246931eebdd195f63597473c572161d2ce10437` |
| `gsd-core/workflows/discuss-phase/modes/default.md` | `d096bdc99b042ed336a7a937650d18cc498e74f19ddc3dd265c38b216a107ad4` |
| `gsd-core/workflows/discuss-phase/modes/power.md` | `ee79bfb72dd35a2b361e8563a488332973553a82e1961105a5f9612b4b5fd216` |
| `gsd-core/workflows/discuss-phase/modes/text.md` | `9644af8abce4766d51abda6654251aa91557a78023f90c0ee60875e928dc8909` |
| `gsd-core/workflows/discuss-phase/templates/checkpoint.json` | `e3bc3dca49db59eb02d2461bb98a53c9ecac041aab377c19a6091d1a517ba186` |
| `gsd-core/workflows/discuss-phase/templates/context.md` | `323884311c94fbc280d0ebbfec7b906bc792e0d50926c4021a24c156eef0378d` |
| `gsd-core/workflows/discuss-phase/templates/discussion-log.md` | `e829d02ec03db8bbc4157845e52283074ef57b9491c04f73833f2cb0876456df` |
| `gsd-core/workflows/execute-phase.md` | `ef687106b5d1981d4cf07a56ad25618937f72f6f93a8536c3293d775163db6d4` |
| `gsd-core/workflows/execute-phase/detail/elaboration.md` | `6e414bf165124160134f99e2e585898be6252d573248be0d55b645e2d9817e66` |
| `gsd-core/workflows/execute-phase/steps/code-review-disposition.md` | `df4b87fed091479a103f03bb5c6ed1bd96b2e3ab2f56631449127bcacc4fccdb` |
| `gsd-core/workflows/execute-phase/steps/codebase-drift-gate.md` | `20216656225493d695102aec801bea749a16c386e3705aab56067308b65c1608` |
| `gsd-core/workflows/execute-phase/steps/completion-reconciliation.md` | `87ffd2a105aea8254b83e7687146b5b79437665fde3c67d8b4f0beb989115909` |
| `gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md` | `78b6522cb622951fc051ea199468b3e64af4a560335422eca3880c0cf1fe6f93` |
| `gsd-core/workflows/execute-phase/steps/executor-progress-policy.md` | `cbddeca12096b81f6a0be38022564ef4cd2b6676bc38b48ed21c1e9de3320abf` |
| `gsd-core/workflows/execute-phase/steps/gap-closure-artifacts.md` | `360387bd34e45b125eab452e011793c4b0b703c7d1960cc1f7c2cf865b53b490` |
| `gsd-core/workflows/execute-phase/steps/partial-wave.md` | `e4f2c7fabcce21ef1ae880d220c62202ce9506afeb7993f44da267448b2054ad` |
| `gsd-core/workflows/execute-phase/steps/per-plan-executor-routing.md` | `ceb4a7a8dd8af6591531152618b4ecc1dd50911ba58890d67764f4f03f035444` |
| `gsd-core/workflows/execute-phase/steps/per-plan-worktree-gate.md` | `099d72a3ea69839c31a62854dffdfb597f288816dae0dc4dbb190933b7829781` |
| `gsd-core/workflows/execute-phase/steps/post-merge-gate.md` | `f182ae0f4138dceedff536512248726cb2a23be1ee89b8a19fa0418f0cfe6a3d` |
| `gsd-core/workflows/execute-phase/steps/protected-branch.md` | `19289f31d53b876c24a989aec2e1a20b0d01be566274dd535cf56b4ce7a6e862` |
| `gsd-core/workflows/execute-phase/steps/ready-wave-gate.md` | `2e02627868bbe696c09ff46c3b09f91d5bfa4ef143bf8f31caa83d772ef7d6e4` |
| `gsd-core/workflows/execute-phase/steps/regression-gate-run.md` | `68cbb6a6c8bc7054cc39be89c9b59ee0f695619a0a08ab39d6bf3e437d4eae78` |
| `gsd-core/workflows/execute-phase/steps/regression-gate.md` | `a788acd6b87c6643684498c6edfdf60c43386915c38434669a4450725bc6928d` |
| `gsd-core/workflows/execute-phase/steps/sequential-root-pin.md` | `7e8eab41db616d0386a18eb96f1e8934d95b58f58a5952d55f1d003021c5bc2b` |
| `gsd-core/workflows/execute-phase/steps/tdd-applicability-resolution.md` | `d7db442ecc1b15d13fe910b1655d35552b13cb912a573c1f52a513e1040bec83` |
| `gsd-core/workflows/execute-phase/steps/threat-id-gate.md` | `7fcba1e23e636eb668933d7f5e09ce6bcc7c15b2cb6749471cbc53a0f99921cf` |
| `gsd-core/workflows/execute-phase/steps/verify-phase-goal.md` | `c2a1f656c3a0dad4dba51bd8f110b165fac6863601771041bf4bfb0b52126527` |
| `gsd-core/workflows/execute-phase/steps/wave-post-gate-hooks.md` | `8ae1ce68ae95ae1fa5dbcb9aff48d89cb9d094e91768923191e7b4446491ba21` |
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
| `gsd-core/workflows/plan-phase.md.txt` | `99180abb8198c1a70fbef4fb09e3bf729701769424e9bdd43aac265591c22542` |
| `gsd-core/workflows/plan-phase/detail/elaboration.md` | `1185427996718e6b91eb3fb19ff16a6761d8960a45eee3a8dd5c2063acfa981f` |
| `gsd-core/workflows/plan-phase/steps/adr-ingest-express-path.md` | `b188047653fdb7ceccbbd110a1cb4406c2f899b29d12f0effc2e61b062be0493` |
| `gsd-core/workflows/plan-phase/steps/auto-advance-check.md` | `54a7c8e641617da91877553e37a337120027e4b66675ef948d5d610b30f0a998` |
| `gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md` | `3f4b4deed4cc8c6ca245571a0f7ab7009baa0049b9297655d405f539c192210e` |
| `gsd-core/workflows/plan-phase/steps/closed-phase-gate.md` | `3cbe49ebeb202bfa596977264cffb912930d0073fd684b8a66c329a4318c5009` |
| `gsd-core/workflows/plan-phase/steps/prd-express-gate.md` | `110a18f8d4f5a9041e9732dee674a6f6c961e96f5151d74d23e8f0dcefb7d23f` |
| `gsd-core/workflows/plan-phase/steps/prd-express-path.md` | `9b08c5995db64a3160a54c688e1f8eb1b594d648023ae8ef80741012e92afa92` |
| `gsd-core/workflows/plan-phase/steps/research-only-early-exit.md` | `0a8e2fef954af151f6a49492d5e0bae9cd619bb41e9593c93eaa4383fa346873` |
| `gsd-core/workflows/plan-phase/steps/research-only-modifiers.md` | `eff31aab435712600f536cedbe74bd565b5c590909bb9bd97697d27ce9a46817` |
| `gsd-core/workflows/plan-phase/steps/reviews-prerequisite.md` | `d0d084ad6a1fea566643a476c3dc6e46d85c49075b4a10cf991553c9ee539f02` |
| `gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md` | `9b9d040fa38e35ce147ba712268fede1f4df3367d414d5380bb66d61dae1915e` |
| `gsd-core/workflows/plan-phase/steps/windows-troubleshooting.md` | `49f58c3f75be3eb5e607b572fd28b1969faa0b47e837c3d9255df36ffbafe80b` |
| `gsd-core/workflows/plan-review-convergence.md` | `0ffef2e4ae27966f485326f017312a42ed153d8da0f29cf49abb81ac32393798` |
| `gsd-core/workflows/ship.md` | `9c0a0acb22b1d8cc446d0bcfd5eed8b075931c343e39d5223c9b8df300303248` |
| `gsd-core/workflows/spec-phase.md` | `cd6f348f7d5e1648fac0051e83bc682ab3920258ab570f6198c2c025eaa38447` |
| `gsd-core/workflows/transition.md` | `9b8a135f54ddbc9de3e48a69686e2e46ed5a8e49517b33251dcdb00c90119041` |
| `gsd-core/workflows/transition/steps/workstream-collision-check.md` | `ddf53acd9512bd2b6caab2c45bff4da585f8a2976bba3d4fe7e6092265e7824c` |
| `gsd-core/workflows/ultraplan-phase.md` | `58512036c7dc6a70f1612854e5a9929cfa22c33039d5242133147daf744c2521` |
| `gsd-core/workflows/validate-phase.md` | `113db27abc3a6b8db0026a35429da15982092280b838a1bc602a08e1c69751a7` |
| `gsd-core/workflows/verify-work.md` | `c979c67666355752c7a594ecdc2652a12dbd7f31fe973253c4d03c37ca49aefc` |
| `gsd-core/workflows/verify-work/detail/elaboration.md` | `43cf5c123ca24a4394e5c3624d96ccf093438aba8dc4cb090d8bfc7206baa267` |
| `gsd-core/workflows/verify-work/steps/automated-ui-verification.md` | `6a2313603645606042261bbfc2af597eebb79ae6c638c10978e806e0e5921045` |
| `gsd-core/workflows/verify-work/steps/mvp-uat-framing.md` | `e540ff57945535fa64b5267ab32f09c1fda59442752514b18476a66da7a34558` |
| `src/runtime-artifact-conversion.cts` | `cda19ea8e6e753a9e2ee92d3c7dc1eb0706051b6d46a7426f5f8aefdc4b0a615` |
| `tests/cursor-conversion.test.cjs` | `c787bcff4ff452f17459caed63706a11ecc3d8cbc2eb728d20dfcdc448da0e6b` |
| `tests/runtime-converters.test.cjs` | `2f39d134874f1746941d4cbeb20bfd8d9b8c77e30a2f1a2524b022ff468b06a5` |
<!-- r42 workflows end -->

## `commands.json`: the 72 commands (r43)

The roster of upstream's commands: for each file of `commands/gsd/` (sorted by stem), its `name` (the stem), its
`description` and its `argument_hint` (`argument-hint`, or null), read from the first YAML block of the file at the tag
`v1.19.0` (a quoted value loses its quotes). It is a snapshot, not a copy: `gsd/core/src/prompts/roster.rs` is checked
against it (`gsd/core/tests/roster.rs`), and so is the README's Commands table.

| File | sha256 |
|---|---|
| `commands.json` | `e203d386a5df1f6f747d210c819dc62b115852fd3c6f5de5732bfb3b05b70ec4` |

## `src/smart-entry.cts`, `src/runtime-slash.cts` and `tests/smart-entry.unit.test.cjs`: the router (r43)

`/gsd:next`'s state-aware classifier and the slash-name formatter it uses, and the classifier's own unit tests, copied
unchanged by `git -C <gsd-core> show v1.19.0:<path>`. `gsd/core/src/commands/smart_entry.rs` ports `classify`,
`isComplete`, `isIdleStranded`, `actionsFor`, `phaseSummary`, `buildSummary`, `progressLine` and `SITUATIONS`, and
`formatGsdSlash`, over signals read from the graph a command receives; `smart_entry/tests.rs` ports the unit cases that
call them on a signals object (or on a fixture whose STATE.md and ROADMAP.md reduce to one) and names the cases it
leaves out. All three are MIT, Copyright (c) 2026 Open GSD (`gsd/core/LICENSE-GSD`).

| File | sha256 |
|---|---|
| `src/smart-entry.cts` | `8503ef8bf36afb7e20b49fadeefc538a81171f69a595a29d7ae8b0d35eaeb021` |
| `src/runtime-slash.cts` | `0ba8838ff084a5182b12ca0e9c346b9c964bf18cdeb57a18591035523cb0ab4a` |
| `tests/smart-entry.unit.test.cjs` | `bd8c7a27c5317110236221c073b6f77e2ac3628877a10f3f0f1533576efdfd80` |

## r43 T1: next, progress and manager

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-next`, `/gsd-progress` and `/gsd-manager`.

| File | sha256 |
|---|---|
| `commands/gsd/manager.md` | `02734b44af8e771e62aec6d2e690ed94c399798260a1b7512726825ef4ee7623` |
| `commands/gsd/next.md` | `784d99db8f903ff0d67582711300ef1421d6f8b9518e536260325ee46c3bf952` |
| `commands/gsd/progress.md` | `0bd26bb1608c9d17e3e471f6e9ca4e144900eff14613414321729a49d6828fad` |
| `gsd-core/workflows/analyze-dependencies.md` | `26af3b5ebdd423206db01491ef09fee429ce24fb96ad7b209a8062d3ab6ae2f9` |
| `gsd-core/workflows/do.md` | `62af2738082a58d336022e3775fbf6b9e5dc6f099bd9e18fd0adb3cea8ac21da` |
| `gsd-core/workflows/manager.md` | `a615e6c5b7eb2e395bc21a3fb427864d4a59b256b0b842d1ec229bb394c0e26e` |
| `gsd-core/workflows/next.md` | `a7194150cb75d0a85e458c2412b85b1b24e595dd2e604cd341a70f3c2964662e` |
| `gsd-core/workflows/progress.md` | `cc444c146336dd9eb2512710258f4b0b173069576207a83aee9db682a3919d5a` |
| `gsd-core/workflows/progress/steps/forensic-audit.md` | `18a8259b34cf9ac88d11915af7163bff3a5ee3ac7eb50bde7bc5ea13668dcfb1` |
| `gsd-core/workflows/progress/steps/mvp-display.md` | `5a8cfbca688a9021e05a8c011a4427c59f50ca151332063b7bb43d62f1a25f7c` |
| `gsd-core/workflows/smart-entry.md` | `7450c3fdd945c5b4a3c0fcf5fbccf0e681913b66c8bee636386f6c51f8682243` |

## r43 T2: quick, quick-batch and fast

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-quick`, `/gsd-quick-batch` and `/gsd-fast`.

| File | sha256 |
|---|---|
| `commands/gsd/fast.md` | `9d26279e3e038f83a7df29b6a2b65c2f55dfc86cef7249756dcb02b805b50c21` |
| `commands/gsd/quick-batch.md` | `502642bf0a71944f779211d9204f49fecfd9ae6f5a41066bac6cc8c4b9d1bc27` |
| `commands/gsd/quick.md` | `5392a78ad85dde4afc1a0ca45a4fb96ea0c1fe9304edc6ef09171a86a4a7f4e8` |
| `gsd-core/workflows/fast.md` | `0f3d8981eeaee0d0b9eb5e290d63e95707635ada2a90607186609b0531acdb4c` |
| `gsd-core/workflows/quick-batch.md` | `fc523a1008727bc5ba17229a019a4a0379b816c40110041d579c105d7de451ae` |
| `gsd-core/workflows/quick-batch/steps/batch-init.md` | `ae78b383ccf94c96a8c3e0f11fa4c7a6ab0f181becd98520c8818f2c02588f02` |
| `gsd-core/workflows/quick-batch/steps/completion.md` | `4792226e1de8878be684c0ee5e6c3e3e2eaefe14412a610c5dc0aefbd200cba5` |
| `gsd-core/workflows/quick-batch/steps/merge-wave.md` | `a680c1f1bb16fb225814e3bfe4910b2b108faf32750efa7b2e6367b87ad0827b` |
| `gsd-core/workflows/quick-batch/steps/plan-checker-loop.md` | `5dd0c0b8f2e1eef85c2997b6726f483d45400880f25c5021cf0d6bfaf16bb62c` |
| `gsd-core/workflows/quick-batch/steps/planner-wave.md` | `d985460359821d90a193f2613eb9060ff8e1e6fef008c2b9fa3257c017088678` |
| `gsd-core/workflows/quick-batch/steps/research-phase.md` | `c9ebeb0e6c19c30417a0f06fdbde6e374e94e0e3faeeb6ec98d9a4df89967b0b` |
| `gsd-core/workflows/quick-batch/steps/resume-mode.md` | `2e0993f30697c8f01277d7a8b05e59a159335d40916b0df2e400586fd99312aa` |
| `gsd-core/workflows/quick-batch/steps/verification-wave.md` | `dfa2ba337fa28b781315e9a5ea9f85f3397b760634d70aff1e596c829e3017bd` |
| `gsd-core/workflows/quick-batch/steps/worktree-dispatch.md` | `fbf40017f6819c84d6855c1b6f582f59e1ea74bbe5125b92e1d82d9c9a2455f1` |
| `gsd-core/workflows/quick.md` | `e38335251722c65bcb760075518f44b69c17d54420fa023ddcb295910575e17f` |
| `gsd-core/workflows/quick/steps/discussion-phase.md` | `ff7a0c851f0b4a3c5ceb26d0aa125897e9a572522c9761a0962279ef46744d7f` |
| `gsd-core/workflows/quick/steps/plan-checker-loop.md` | `e6375ff05a25c70f8a10be8a2aa90fd4c46220d451702d915fa4877b472652fa` |
| `gsd-core/workflows/quick/steps/quick-verification.md` | `84cd6b07a1378f3d27a1d5dc2a168c8260339d2062e433b6a4dd5e48906f6f04` |
| `gsd-core/workflows/quick/steps/research-phase.md` | `9dbacbf97906704f30b3cedba24a2bd655d189e05274e870ba1c60ab4459dc80` |
| `gsd-core/workflows/quick/steps/worktree-pre-dispatch-commit.md` | `e2387a582bb3f74d4622cbcf31b65eb7e78630c6c1f770c070021d4131115e2f` |

## r43 T3: autonomous

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-autonomous`.

| File | sha256 |
|---|---|
| `commands/gsd/autonomous.md` | `92de7711b8f9aa52bf3e7f0004023429fe483c1a4f3aa0fa65c8eb71c6f2d539` |
| `gsd-core/references/autonomous-smart-discuss.md` | `b7e449daf980339418e523689465c32b10bfe315b20ddf8af8a99ff8c6caccce` |
| `gsd-core/references/autonomous-ui-design-contract.md` | `be49254b96992c7ef72c34ccc9c85cd6e65fac5773065ec28259638843c174a3` |
| `gsd-core/workflows/autonomous.md` | `60404df809dc593868c053e622c7d508c43c7ff78fcf445c2b416b7fbf31c91a` |
| `gsd-core/workflows/autonomous/steps/converge-banner.md` | `623a3dc7f4ee79b03550dd6c7e5ee6e2e2d53322465e030dff31fd3304cf119d` |
| `gsd-core/workflows/autonomous/steps/converge-dispatch-bg.md` | `a9e80df7f1629639890d87b6227ddeacf2dd7973adb10cce471e2e009ed68691` |
| `gsd-core/workflows/autonomous/steps/converge-dispatch-inline.md` | `9c28a7ff77bdd61ba17fcbf951c69d376a6679018e02e01345ab9502c7dabc4e` |
| `gsd-core/workflows/autonomous/steps/converge-fail-fast.md` | `724b870339135c19f53b9a8e32e552c8659755ebfb9c2631f2e71162b0670908` |
| `gsd-core/workflows/autonomous/steps/converge-loop.md` | `2e7236809b8323b00432aed14c6efc2eb4be0f418cd81f309e0fcef9d8d42cfc` |
| `gsd-core/workflows/autonomous/templates/minimal-context.md` | `5b5a283dbfca92f4f36859f869bfa103442baa107eaa9198f6d42fcb6a5cf891` |

## r43 T4: pause-work and resume-work

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-pause-work` and `/gsd-resume-work`.

| File | sha256 |
|---|---|
| `commands/gsd/pause-work.md` | `4d9b31eb7d8657d2c770d9db28daa4d10a8da463c580ca77215dd7163d2f7aae` |
| `commands/gsd/resume-work.md` | `b7d2324a5c96462a725fcc8ce67ef87b1d28a8ac623e2cd801adb8681b725538` |
| `gsd-core/references/continuation-format.md` | `ce73e57d053ab8a7949ebeadbf5d0d56563603375e23c9d59a7787746fa0e3f4` |
| `gsd-core/workflows/pause-work.md` | `47195b230b0915f7629578a500795026e06079a232c5239dad2378e8290c68e6` |
| `gsd-core/workflows/resume-project.md` | `bfaa76506389f8b10efdde14b08c27eff0d98cf7433b6a4fd2de822555aafcda` |
| `gsd-core/workflows/session-report.md` | `0a8ed74c950a4d5b34b4d1847f059f5a7d95a5bf9584f86289a3c3995f39067e` |

## r43 T5: review and review-backlog

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-review` and `/gsd-review-backlog`. `reviewer-instances.md` is kept as `.md.txt`: its link to an ADR leaves the tree.

| File | sha256 |
|---|---|
| `commands/gsd/review-backlog.md` | `ec0cdae53b5edf72d6f8ed55de2ad0291707a7d9b2bf7ae4ba61ea1079b1df34` |
| `commands/gsd/review.md` | `cea64f93eb906b4ea7f33223da0049f288de287006d36d37e75c14989de75ff9` |
| `gsd-core/references/reviewer-instances.md.txt` | `c2cac14d435400215398feeace69fdc95d6c4a81bd792f67887bd444219c0f35` |
| `gsd-core/workflows/review.md` | `d31eae9b5bb017a0216349788b942eac96abb95466933102368752bda996add5` |
| `gsd-core/workflows/review/steps/reviewer-instances-note-1.md` | `8e99eab70c786614de4cee08bea6ab3522361dcacb11a54cf3561caee52b524f` |
| `gsd-core/workflows/review/steps/reviewer-instances-note-2.md` | `25672f5511d739d9a7396f85313b899b9e9de53c43797bfae4d3f017424f1cbc` |

## r43 T6: settings and config

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-settings` and `/gsd-config`. `planning-config.md` is kept as `.md.txt`: its link to a doc leaves the tree.

| File | sha256 |
|---|---|
| `commands/gsd/config.md` | `cbb67008f8701f4611d1452f4e9be4ad7427bf21ccd09b101a0ed6679a642cec` |
| `commands/gsd/settings.md` | `ad4cd4b4a5a3bab5f5fe2df1db1932d7629a0445fe42e1fbf571c135f79597f1` |
| `gsd-core/references/planning-config.md.txt` | `5f0b3d9606ad0195ea3d94c7db0b04e4d376ecef4a6f1e367f5fd6f59c2df9e5` |
| `gsd-core/workflows/settings-advanced.md` | `c9cc02bf5f7c7c1c1155a839e7408aa54eb3749d441a435cbb0360773e54fcac` |
| `gsd-core/workflows/settings-integrations.md` | `eb4634671256c7e567bb278ba96e8fcb41cb541c0f9c7c3f23e9ad8b586b6fb1` |
| `gsd-core/workflows/settings.md` | `6b1525bb801e323cb4786cb1def7ceec38480ab55cfc0843d1499ae0fe54ea1f` |

## r43 T7: undo and pr-branch

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-undo` and `/gsd-pr-branch`.

| File | sha256 |
|---|---|
| `commands/gsd/pr-branch.md` | `b7eccd9e24f97f3612ce93f2199edab58133d0706b27745fbb442d0eb37ee2c4` |
| `commands/gsd/undo.md` | `d0fbc584d2d21ad565d4d4be4ede33feff5d631324927e9abecf64dd8cae5b73` |
| `gsd-core/workflows/pr-branch.md` | `63c1482ecea847da620e8310ca2a7854a229f31a74caa8b4709a22b46bb1193e` |
| `gsd-core/workflows/undo.md` | `0812391ee95fb221d360dd22c7b97721200be536dc5b8c6a9c4740970be7ac73` |

## r43 T8: milestone-summary and extract-learnings

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-milestone-summary` and `/gsd-extract-learnings`.

| File | sha256 |
|---|---|
| `commands/gsd/extract-learnings.md` | `16e09bc1cee90e60c0cae14e273c1ed1e569b66f10f81d4c72cdbbfd01bbfb94` |
| `commands/gsd/milestone-summary.md` | `60710830b35b2a1c00be093e696e520b6936e1d2a9f1927d82e59e38c306d0b5` |
| `gsd-core/workflows/extract-learnings.md` | `a9281400d064bab6e3e4013c02f9d5e26cb509bb83ed6e9d7429f5fb7faa057d` |
| `gsd-core/workflows/milestone-summary.md` | `3feb033825b2f3df58c61e6962da15470e1d5abfd5ce77a83b7d83d7710bcd2a` |

## r43 T9: ingest-docs and onboard

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-ingest-docs` and `/gsd-onboard`.

| File | sha256 |
|---|---|
| `commands/gsd/ingest-docs.md` | `b5fc2db95efedc11d550ba1ee3cbb34e62b2a56787285f59faca0bf8edfc92cc` |
| `commands/gsd/onboard.md` | `97eebe6830989d96f71eb48be3aa2b588a6d17efa70826dea1e73d3247beac29` |
| `gsd-core/workflows/ingest-docs.md` | `5a5fb661c572c3ce8f57d13c69ce57f2581fa5767aed2ac15a0eac68176455e4` |
| `gsd-core/workflows/onboard.md` | `6e8ecfe10f680e73b2d19ec5abf7095360525f11799ee8c3c0a4a1de276cc97a` |

## r43 T10: workspace and update

Vendored byte for byte from gsd-core v1.19.0: the three workspace workflows and `/gsd-workspace` are ported; `commands/gsd/update.md` is vendored for the slash metadata only (the update text is ours, D-r43-16).

| File | sha256 |
|---|---|
| `commands/gsd/update.md` | `c009de60568956c4a89d9bcb83527e1d12edc9f486356ca869a4d71fae043af4` |
| `commands/gsd/workspace.md` | `cc8cd6dad484ae91feb48457aef4e3a96b76874897cb32eb9b8c784100aec05e` |
| `gsd-core/workflows/list-workspaces.md` | `da446be2980632cc3b765f430a41cfbe00a8bbfcedc7656c0cf82646178680ba` |
| `gsd-core/workflows/new-workspace.md` | `55ccc4ee96b522eea2932d8ee977cd9617e32127650f77820e6c9e7fdd206e54` |
| `gsd-core/workflows/remove-workspace.md` | `d9cc19a118a68c81d436a698d9f803fed4152b439afc7c2aa27024d4db7da109` |

## r43 T11: the six namespace routers

Vendored byte for byte from gsd-core v1.19.0 for `/gsd-ns-context`, `/gsd-ns-ideate`, `/gsd-ns-manage`, `/gsd-ns-project`, `/gsd-ns-review` and `/gsd-ns-workflow`.

| File | sha256 |
|---|---|
| `commands/gsd/ns-context.md` | `9671e855bdb8339a40f6ab73735254977470072d1eb4080154b55568913ef78f` |
| `commands/gsd/ns-ideate.md` | `53a446e2c39be49c863f7c9253fd4328201ce68e2becd24db95a18f79db5f3d5` |
| `commands/gsd/ns-manage.md` | `83dd7c64de6618bd1f7ba6c43877a9eda0e4ffaade47e9455cd3757239d724dc` |
| `commands/gsd/ns-project.md` | `2965014b60f6951e8b10526dd16c2dfa7e1e737c414384efff6f5bb495fa718f` |
| `commands/gsd/ns-review.md` | `5af65e411b1cc5baa6a2fbf3cfdf9452f671c11a72c21d5b88e89ee78dfbcfd5` |
| `commands/gsd/ns-workflow.md` | `9492a9cdcb4ea148f598688a252b45ee9b695e83b997bfe56834d701ec369c2d` |

## r44 T1: the read, write and secret-read guards

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the three guard scripts, the filename classifier the secret guard requires, their test suites (read by `gsd/core/src/hooks/{guards,secrets}/tests.rs` for the accounting of every case), and the inventory manifest the hook roster is compared with. `hook-cases/` holds the calls these suites make, recorded by `gsd/parity/record-hook-cases.cjs`.

| File | sha256 |
|---|---|
| `hooks/gsd-read-guard.js` | `87dcd4118921ed7de1a9db420f558fa86e713178fc242e83f5898c1ab0564d14` |
| `hooks/gsd-write-guard.js` | `1ec4bb014724ee3e7c636658decb96ec2e271867e1356e38e4a6b87fc8cddbae` |
| `hooks/gsd-secret-read-guard.js` | `390fecfa8422400e55e1a6a04ba9bd25581a04dabe68bf5d5985838e1e1195d2` |
| `hooks/lib/filename-classification.js` | `7888ff56ec697c4e4988f14d34eb54cd2616546ab9f13484fec6339df22c3527` |
| `tests/read-guard.test.cjs` | `1200523c46e2362f9f81bec83363a3e6e15b2b3bc012b24083d5e016dae2fcd4` |
| `tests/gsd-write-guard.test.cjs` | `04848f2efa144efef768f0ff0cdf63b4a95700ebf83c520e15e8ebc8289a9520` |
| `tests/gsd-write-guard.property.test.cjs` | `be3f162aacfd6cf26e8a36d73b17fff4bbba925d1154b1851aa6850fa130dcab` |
| `tests/gsd-secret-read-guard.test.cjs` | `98ceb26e0bb01f5308bee70f8afcbea1682c36cdeaa29611874cda3412bfa5cc` |
| `INVENTORY-MANIFEST.json` (taken from `docs/INVENTORY-MANIFEST.json`) | `12cbaa217822a13225ad524479ea73fe1f9d5dd054da44f586909736e6de30c3` |
| `hook-cases/gsd-secret-read-guard.json` (recorded from v1.18.0 by `gsd/parity/record-hook-cases.cjs`, not upstream bytes) | `2cc44c73230f8ee3811aeb392cb2ec5fcd23a03682c595cd7f6bf7cddeaeb26c` |

## r44 T2: the injection scanner and the prompt guard

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the two hook scripts and their shared pattern list, and the five suites that test them (read by `gsd/core/src/hooks/injection/tests.rs`; `prompt-injection-scan.security.test.cjs` and `injection-blocking-config.test.cjs` call no hook and are skipped case by case). `hook-cases/` holds the calls the suites make to a hook script and the answers upstream gave, recorded by `gsd/parity/record-hook-cases.cjs` with `REC_KEEP=100000 REC_INPUT_MAX=100000`.

| File | sha256 |
|---|---|
| `hooks/gsd-read-injection-scanner.js` | `89d8b1a2aa73f80c85385398c40ce4a56fb6bb20cb5c17ec3090e8e31b3d987a` |
| `hooks/gsd-prompt-guard.js` | `f7a5da2130d76cec3d08eb30156f17721ac954e90d18685b620abbf2ab701cab` |
| `hooks/lib/injection-patterns.js` | `f99dc1db23291adf7e3f46f9fb20ad475d7747e8293b2322d1e2d0c94ef1dd70` |
| `tests/read-injection-scanner.security.test.cjs` | `0e68499e2538f476fc6828feea8ca041963564b370f1053f37c4c141d3477ca3` |
| `tests/read-injection-scanner.property.test.cjs` | `eefb42758f39ca42d607ae5445a1a853bc4802f91198bebbbea66710dc22d063` |
| `tests/injection-patterns-parity.security.test.cjs` | `0e6665ecd027b0af65ad97abe72a32f6eb589d0fa9c35a543a885e6f3b5e8d51` |
| `tests/prompt-injection-scan.security.test.cjs` | `d55c81bcba829b2b5bb2cb5727e8cfb1f19646613f291c78accc69d060f085ff` |
| `tests/injection-blocking-config.test.cjs` | `8a4b6136de66b43419e401dd13a9549179073d2ba3e0d8b5924da8f329b151b4` |
| `hook-cases/read-injection-scanner.security.json` (recorded from v1.18.0 by `gsd/parity/record-hook-cases.cjs`, not upstream bytes) | `c936059a4a3b6745f83216993e9f995de55f83d5016935e16f949e32767b2e6f` |
| `hook-cases/read-injection-scanner.property.json` (recorded from v1.18.0, not upstream bytes) | `e7084608eea38bee907e00e81532fcee0955a907a70fde5222493892f5cd6080` |
| `hook-cases/injection-patterns-parity.security.json` (recorded from v1.18.0, not upstream bytes) | `05fd1d62197f9201985aa783493fe57b48044f6242c823de2cc9bb2bae07320e` |

## r44 T3: the agent isolation and worktree path guards

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the two hook scripts, the reason codes the isolation guard requires, and their suites (read by `gsd/core/src/hooks/isolation/tests.rs`). `worktree-safety.test.cjs` is vendored whole (389 cases); only its `bug #260: gsd-worktree-path-guard.js` and `#1342` blocks are in scope, and every other top-level block is one skipped prefix.

| File | sha256 |
|---|---|
| `hooks/gsd-agent-isolation-guard.js` | `a61b2817e75c87a014e8bf9215a72eba8a9a6c49910836b6513253ef367c7d8c` |
| `hooks/gsd-worktree-path-guard.js` | `6019c3f2cd9c537e60c7da2616cb061ace3ceca225be88b252d1947ac2bb1b38` |
| `hooks/lib/isolation-deny-reason.js` | `4cbff1576f05e4aae4978b2acd5bdff875f462fe73587a15dda96713d0e657f4` |
| `tests/gsd-agent-isolation-guard.test.cjs` | `3ce007cb340b5b0888e4a50f7f9df4441c492f16ab4b5322d70a857daaec6c6a` |
| `tests/worktree-safety.test.cjs` | `3aef69377df585672d96ec2047a1cb6e7f8e49be055c10060fc9eb1bc17892ac` |

## r44 T4: the workflow marker and guard

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the guard hook and its two suites (read by `gsd/core/src/hooks/workflow/tests.rs`). The hook's `git add -f` block uses `lib/git-cmd.js`'s `skipToSubcommand` (vendored here, K20c T15; ported in `workflow.rs`); its other requires (`git-probe.js`, `hook-exit.js`) belong to the node hook's process and are not vendored.

| File | sha256 |
|---|---|
| `hooks/gsd-workflow-guard.js` | `7a623e9730a2149d28043f569fe72fc9c180bd3d3b90111fd28e87a7a67f72fa` |
| `hooks/lib/git-cmd.js` | `c46c0aa506b033d9aa7f3160897cf0b2eeef8501b567307c206e094a9d31af4a` |
| `tests/workflow-guard.test.cjs` | `069e9baf3f610f353d2edd322f02ea214403b1eabb058e16d1e30c64afe68e9b` |
| `tests/workflow-guard-registration.test.cjs` | `256e8ac2fb230ca75ee6186204bcffb04d003a27f62d88c9120be44b6c877458` |

## r44 T5: the context monitor

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the hook and its two suites (read by `gsd/core/src/hooks/context/tests.rs`). `perf-317-context-monitor-fs.test.cjs` is vendored whole (the home of every monitor case); its fs-hardening, #1974, #4905 and event-name blocks are skipped by prefix.

| File | sha256 |
|---|---|
| `hooks/gsd-context-monitor.js` | `5bb3e7b98c705fe5542019c130bc76eef34f183f5fad851f5804fc3c161d5a66` |
| `tests/context-monitor-host-contract.test.cjs` | `7cc3595646c897d270b2643278ec969b7bca5af8e68e5cc6b632894699d189ea` |
| `tests/perf-317-context-monitor-fs.test.cjs` | `b29a632fb2a4888fb89f102a65b9a3254b4d78124cb1268bc554b5bd32a47669` |

## r44 T6: the statusline

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the hook and its three suites (read by `gsd/core/src/hooks/statusline/tests.rs`). `gsd-statusline.test.cjs` is vendored whole; its STATE.md reading, git, todo, update-banner, environment and other-runtime blocks are skipped by prefix with their reasons, and `gsd-statusline-git.property.test.cjs` is wholly skipped (git).

| File | sha256 |
|---|---|
| `hooks/gsd-statusline.js` | `1cf90222b8bd505f92801dc0a2cd00a457284c6a265e8df65cd6429a43b05ba8` |
| `tests/gsd-statusline.test.cjs` | `b59c83f69e91c848bb90132f0b084eadd8108997c571aa31d10581de74da2956` |
| `tests/gsd-statusline-state.property.test.cjs` | `53b9ab7bb6b293de821b75d4488c9c31c530efeaeda6840c4e80b80d6dee43e6` |
| `tests/gsd-statusline-git.property.test.cjs` | `51b79c73a60510cc905027f56d1d8407462eed110a5b1d50e0ad966920f64c54` |

## r44 T7: validate-commit, phase-boundary, session-state

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the three opt-in community scripts and the three suites that spawn them: the two suites of the commit validator and `tests/hooks-opt-in.test.cjs` (80 cases, about 60 of them validate-commit verdicts; read by `gsd/core/src/hooks/community/tests.rs` and `tests/opt_in.rs`, every case ported or skipped with a reason). Upstream's own test file for phase-boundary and session-state is that same `hooks-opt-in` suite; the module's own cases follow it. The validator's `lib/git-cmd.js` is vendored under r44 T4 and not read here: the host classifies the command (`pre_commit`) and hands the hook the message.

| File | sha256 |
|---|---|
| `hooks/gsd-validate-commit.sh` | `75ab1d087f17f7e57cf7cf7e8a54ebe2b11503868ae6aaa1b5c6aacbfa24c55b` |
| `hooks/gsd-phase-boundary.sh` | `ff5a1f242b31f689dc8c856d508b5a9bc4a9c4713c096745f429aa7bab22237d` |
| `hooks/gsd-session-state.sh` | `e7c8f2a8a973e95463804234c87c563bb86678209f5275494fcbe3095f4033cb` |
| `tests/gsd-validate-commit-crash-policy.test.cjs` | `ff75f854974e02751cdff9a432a96c2c488bfbf6f96d58fd13d90b68939dfc92` |
| `tests/gsd-validate-commit-sigpipe.test.cjs` | `715248bb17b0ed93c4ec04a9acdc47e60a5809695b776171c96a346716d6f78b` |
| `tests/hooks-opt-in.test.cjs` | `345fc9f0b4b5dc6c824fb62d001b05dfdaa93d712dadde65403b337dbf921b75` |

## r44 T9: capability ids

`capability-ids.json` is the sorted list of the 45 directory names of `capabilities/` at the tag (`git ls-tree --name-only v1.19.0 capabilities/`, the `capabilities/` prefix removed). The vendored `capabilities.json` holds only the 32 capabilities that have a `config` block, so it cannot say which capabilities exist; `gsd/core/tests/parity_roster.rs` reads this file against the README's capability table.

| File | sha256 |
|---|---|
| `capability-ids.json` | `15994a8e3bb35fcaf05b513b811ee8a809dbdd64ee3316c69b8565c5ba5d178d` |

## r44 T10: all of `gate-cutover/`

All 171 verdicts of `tests/fixtures/gate-cutover/` at the tag, copied byte for byte (`git show v1.19.0:tests/fixtures/gate-cutover/<file>.json`, each compared with `cmp`); the 161 new ones are the `ui-*`, `api-coverage-*`, `verify-*-drift*`, `tdd-*`, `prohibition-*`, `predicate-*`, `auto-mode-*`, `r2-*`, `r4-*` and remaining `decision-coverage-*` files. `arms.cjs`, which builds their fixture projects, is not vendored: the twins are `gsd/core/tests/projects/`. `gsd/core/tests/parity_rules/cutover.rs` classifies every file once, and `parity_rules.rs` requires the table and this directory to name the same 171.

| Set | sha256 |
|---|---|
| `sha256sum gate-cutover/*.json \| sha256sum` (sorted by name) | `e1451d4f04220d79f8d5d2a03823bbca8fda3f132e87ad6029139100803aeb0d` |

## K20c T18: the config reload

Vendored byte for byte from gsd-core v1.19.0 with `git show`: the hook (read by `gsd/core/src/hooks/config.rs`'s header only; upstream has no suite for it). Its `lib/hook-exit.js` is not vendored: a hook here does not exit.

| File | sha256 |
|---|---|
| `hooks/gsd-config-reload.js` | `6a2c579e2709c76bd753e1de94ccc0d7bb16cefa47b0ef482f847add24315b6f` |

## Plan 16 T19: the security capability

Vendored byte for byte from gsd-core v1.19.0 with `git show v1.19.0:<path>`, except `src/security.exports.txt`, which is **not upstream bytes**: the `export` lines of `src/security.cts` (`rg '^export'`) under a one-line header, the roster of the hardening library that `@gsd/security` does not port (plan 16). `gsd/security/tests/port.rs` compares the two agent bodies with the ports under `gsd/security/assets/agents/`.

| File | sha256 |
|---|---|
| `agents/gsd-security-auditor.md` | `3399f81791d4579138dd77ccc232cb267e5e6f1419aaebbb8f374627019e3872` |
| `agents/gsd-security-auditor.compact.md` | `7b9d83288d6ec1014a87469347480da3304f9917f5757766e0e4213e9ee64582` |
| `commands/gsd/secure-phase.md` | `1bc5b11bf6222fba66685d0f5e5632ba14707e17e0f750df316c02ccf7bb97ca` |
| `gsd-core/workflows/secure-phase.md` | `8f2b8fb4a56b0c85e42c94f5b88fb82159708f039bf8de5febfdabf4443129a8` |
| `gsd-core/templates/SECURITY.md` | `4a60dd6efdc4667584afca619d360de22ee7cb7986abe3d6e268cd09b19eeb45` |
| `capabilities/security/capability.json` | `64a65244086e884c79d6be0e97a99d0b91ad6812c10f8fa219c6e518108ed272` |
| `tests/secure-phase.test.cjs` | `f9b8493d3ded82e23d1922eaaa2cfba593b1bf00c05c942f46dc8b7a32e6f62f` |
| `tests/secure-phase-single-writer.test.cjs` | `bfa5f71fc15bf56a826a9c90f7c31ac5c2d9c4d08c0cc9897de973b0e5cda389` |
| `tests/injection-blocking-config.test.cjs` | `8a4b6136de66b43419e401dd13a9549179073d2ba3e0d8b5924da8f329b151b4` |
| `src/security.exports.txt` | `58f448a1b28dcf30e21e683b548e0032ec8151448da78369bc14b9f258480fe0` |

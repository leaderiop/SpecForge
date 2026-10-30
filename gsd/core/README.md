# `@gsd/core`

**Status: `@gsd/core` 1.0.0 tracks gsd-core 1.18.0: every upstream item is accounted in [ACCEPTANCE](../parity/ACCEPTANCE.md), which lists the named gaps (`gsd/W514` and `gsd/E515` are not built, hooks are 15 of 28 upstream files, the `specforged` harness is later, Codex and Cursor are unrun) (r44). All 50 core host names are served (r43): 44 prompts, 30 commands and 50 slash commands, that is the 16 lifecycle prompts (`/gsd-new-project` to `/gsd-ship`, r42) with their gates and the command `reference`, then the utility, session, configuration, review, git, learning, onboarding and workspace prompts, `/gsd-update`, `/gsd-next` over the read-only command `smart_entry`, and the six `/gsd-ns-*` routers; the foundation and the discuss, specify, research, plan and execute kinds are built, with the collectors that bring commits and tests into the project; the verify kinds (verification, UAT, validation, audit, the broken-window ledger) are built, with their computed values and rules; the 14 hooks (r44); the 15 core agents, the command `record_verdict` and the prompt `spec_phase` are built (r40), and so are 29 commands (21 writing) and 7 slash commands (r41, ADR 0299), and nine judged rules take an agent's verdict (`gsd/W311`, `W313`, `W315`, `W316`, `E501`, `W205`, `E518`, and `E202` and `W203` by the prompt); `gsd/W514` and `gsd/E515` wait for the kernel (`gsd/E502` is built).** The unified GSD meta-model: project,
requirements, milestones, phases, plans, tasks, decisions, research, results, verification, UAT and validation, with
GSD's checks as rules and its commands, agents, prompts and hooks as contributions. Tracks gsd-core 1.18.0 (the crate's
`[package.metadata.gsd] tracks` and the handshake keyword `tracks:gsd-core@1.18.0`).

It declares no dependency on any `@specforge/*` extension and on no other `@gsd/*` extension; the capability
extensions and the profiles declare it as their peer.

## What it declares

| | Declared now | Where |
|---|---|---|
| Kinds | `project` (one per repository: the kernel keeps `singleton` as metadata, `gsd/E108` enforces it), `requirement` (named by its GSD id), `milestone` (scoped by workstream), `phase` (scoped by milestone, named `03`, `03.1` or `999.1`), `workstream`, `retrospective`, `lesson`, `methodology`; `context` (named `02-CONTEXT`, scoped by its phase), `decision` (`D-01`, scoped by its phase or the project, a project decision is `/D-01` from inside a scope; from outside, a phase's is `v1.0/02/D-01`), `deferred_idea`, `phase_spec`, `locked_req`, `interview` (append-only), `discussion` (its entries append-only), `research` and its eight item kinds (`finding`, `stack_choice`, `pattern`, `pitfall`, `dont_hand_roll`, `open_question`, `source`, `package_audit`), each item scoped by its research; `plan` (named `03-02`, scoped by its phase; a quick plan by its quick id), `task` and `checkpoint` (scoped by their plan, `T1`, `C1`), the must-haves `truth`, `artifact`, `key_link` and `prohibition`, `disposition` (append-only) and `user_setup`; `summary` (scoped by its plan), `deliverable` (scoped by its summary, with computed check statuses), `deviation` (append-only), `commit` (written only by the git collector), `pause`, `handoff` and `blocker`; `verification`, `verify_gap`, `override` (human-only), `human_item`, `uat_session`, `uat_test`, `uat_gap`, `validation`, `validation_row`, `wave0_item`, `manual_check`, `milestone_audit` (its `debt_accepted` only a human sets), `broken_window` and `refactor_proposal`: 54 kinds in all | `src/kinds/`, model in [spec/project.md](spec/project.md), [spec/discuss.md](spec/discuss.md), [spec/plan.md](spec/plan.md) and [spec/execute.md](spec/execute.md) |
| Agents | the 15 core agents of gsd-core 1.18.0, bodies ported under MIT (`assets/agents/`), their tools mapped to host tool classes, ten with a compact variant (`specforge.agent_variant`) and each with a model line from `model_overrides.<agent>`. `specforge install <host>` writes them; a judge's file also lists the rules it judges: `gsd-plan-checker` (`gsd/W311`, `W313`, `W315`, `W316`), `gsd-verifier` (`gsd/E501`), `gsd-phase-researcher` (`gsd/W205`), `gsd-integration-checker` (`gsd/E518`); the other eleven judge nothing. A judge's body ends with a section that names `record_verdict`. **The bodies are runnable:** every `specforge gsd <cmd>` a ported line names is a declared command (`tests/port_tools.rs`), and a judge's body ends with a section that names `record_verdict` and the refusals of K17. The attribution notice always names the full file `agents/<id>.md`, also when the compact body renders | `src/agents/`, `src/judged.rs` |
| Commands, prompts | 30 commands (21 write; `reference` prints an embedded workflow, reference or template, and `smart_entry` is read-only: it answers what `/gsd-next` routes by): `record_verdict` records a judge's verdict through `set_verdict`; the plan, summary, checkpoint, UAT, validation, pause, blocker, broken-window, refactor and milestone commands write the `H` fields (23 of the 33 have a command writer and 10 are the `gsd-git` collector's; `contributions/README.md`); `health`, `stats` and `progress_report` read the computed values of the graph a command receives (ADR 0299); `phase`, `workstreams`, `cleanup`, `help`, `health`, `stats` and `surface` are also slash commands. 44 prompts and 50 slash commands (43 prompt-backed and 7 command-backed; `execute_plan` has no slash command): r42's `new_project`, `new_milestone`, `discuss_phase`, `spec_phase`, `mvp_phase`, `plan_phase`, `plan_review_convergence`, `ultraplan_phase`, `execute_phase`, `execute_plan`, `verify_work`, `validate_phase`, `add_tests`, `audit_uat`, `audit_milestone`, `complete_milestone`, `ship`, and r43's `next`, `progress`, `manager`, `quick`, `quick_batch`, `fast`, `autonomous`, `pause_work`, `resume_work`, `review`, `review_backlog`, `settings`, `config`, `undo`, `pr_branch`, `milestone_summary`, `extract_learnings`, `ingest_docs`, `onboard`, `workspace`, `update` and the routers `ns_context`, `ns_ideate`, `ns_manage`, `ns_project`, `ns_review`, `ns_workflow`. Each renders upstream's workflow (286 ported assets under `assets/`, byte-equal to the vendored v1.18.0 text except the lines that named `.planning` or `gsd-tools`; `update` renders a text of ours, because upstream's installs upstream GSD over this one, and a router adds the route table of this install) for claude-code, codex and cursor. A prompt with a subject is gated: `plan_phase` and `ultraplan_phase` by `gsd/E201`, `W204`, `E206`, `W406`, `E516`; `execute_phase` by `W311`, `W313`, `W315`, `W318`, `E516`; `execute_plan` by `W318`, `E516`; `complete_milestone` by `E106`, `E511`, `E516`; `ship` by `E511`, `E512`, `E516`; the others by `E516`. `gsd/W506` (a decision of a phase's context that no summary or commit mentions) gates `workflow.context_coverage_gate` | `src/commands/`, `src/prompts/` |
| Collectors | `gsd-git` records each commit that names a plan or task as a `commit` entity in its plan's file: `specforge collect --runner gsd-git --yes` (consented `git log`). `gsd-junit` reads a JUnit report and links a test to its entity by a `[gsd:<id>]` tag, and to a deliverable's check by GSD's own `path#test name` reference: `specforge collect --runner gsd-junit --no-run` (or `--report <file>`). Until the first `collect`, `specforge check` says I205 (information: no test report) | `src/collectors/` |
| Passes | `derive` (the computed fields: each context's `digest`; verification status, score and re-verification, UAT totals and status, validation coverage, audit status, the phase `completed` ladder and the window counts; plan, task, summary and deliverable-check status from the commits and the test report; requirement, phase, milestone, project, workstream and context values), `execute` (`gsd/E401`, `gsd/W403`, `gsd/W407`, `gsd/W408`), `schema_drift` (`gsd/E405`), `roadmap` (`gsd/E101`, `gsd/E102`, `gsd/W103`, `gsd/W104`, `gsd/E108`, `gsd/E206`), `research` (`gsd/W204`), `drift` (`gsd/W406`), `plan_shape` (`gsd/W323`, `gsd/E302`, `gsd/E303`, `gsd/W304`, `gsd/E307`, `gsd/W308`, `gsd/W309`, `gsd/E322`), `waves` (the computed `plan.wave` and `plan.autonomous`), `file_scope` (`gsd/W305`, `gsd/W306`), `scope` (`gsd/E312`, `gsd/W317`), `cross_plan` (`gsd/E301`, `gsd/E310`, `gsd/I321`), `scan` (computes the file facts: anti-patterns, debt markers, artifact and key-link states) with `scan_check` (`gsd/E505`), `nyquist` (`gsd/E314`, `gsd/W510`), `verify_rules` (`gsd/E503`, `gsd/W504`, `gsd/E509`), `windows` (`gsd/E513`), `gates` (the guards `gsd/E106`, `gsd/E511`, `gsd/E512`, `gsd/W318`, `gsd/W404`, `gsd/E516`, which answer at a gate trigger and are silent on a write; `gsd/W514` is not built) and `verify_paths` (`gsd/E320`, `gsd/W324`; computes `task.verify_paths`, which it reads through the project's files) and `fails_when` (`gsd/E507`) | `src/passes/` |
| Hooks | 14 hooks (r44): `read_guard`, `write_guard`, `secret_read_guard`, `injection_scanner`, `prompt_guard`, `worktree_path_guard`, `workflow_marker`, `workflow_guard`, `context_monitor`, `context_compacted`, `statusline` and, under `hooks.community`, `validate_commit`, `phase_boundary`, `session_state`; what each does and does not do is in `contributions/README.md` | `src/hooks/` |
| Rules | `gsd/E105` (an `out_of_scope` requirement has a reason), `gsd/E201` (a context has canonical refs: two declarative rules under one code), `gsd/E107` (generated for each authored timestamp field and block member: first `blocker.resolved.at`), `gsd/E402` (a halted summary says why), the seven declarative `gsd/E307` rules (what each checkpoint kind requires) and `gsd/E319` (a test-tier prohibition names a test) | `src/rules.rs`, `src/kinds/dsl.rs` |
| Code book | all 63 `gsd/` codes of [spec/rules.md](spec/rules.md), each with its `gsd-compat` severity and what `gsd-strict` raises it to; the rules of the others land with their kinds | `src/codes.rs` |
| Settings | the 181 GSD configuration keys `@gsd/core` declares (a count of declarations; ACCEPTANCE counts the inventory's 211 keys, 161 built here, on that basis), typed with GSD's default, and 12 families (`models.*`, `granularities.*`, …), `specforge.agent_variant`, the ten `specforge.surface.<cluster>` settings that show or hide skills; the 15 `model_overrides.<agent>` keys belong to the `model_overrides.*` family and are set by a preset | `src/settings/keys.rs`, model in [spec/settings.md](spec/settings.md) |
| Profiles | `profile` (`gsd-compat` by default, `gsd-strict`, `gsd-plus`), presetting five settings and raising rule severities; `model_profile` presets the 15 `model_overrides.<agent>` and `workflow.compact_content` presets `specforge.agent_variant` | `src/settings/profiles.rs` |
| Gates | the four `specforge.*` settings and the `workflow.*` toggles switch off the rules [spec/rules.md](spec/rules.md#setting-gated-rules) lists | `src/settings/gates.rs` |
| Secrets | the key half of the seven search flags (`brave_search_key` … `jina_key`), read from `BRAVE_API_KEY` … `JINA_API_KEY` | `src/settings/profiles.rs` |

A setting is declared before the round that reads it, so most are inert today; `specforge settings` lists what a
project resolves them to. A rule whose code is in the book is silent until its round.

## Build, test, load

The family is one Cargo workspace under `gsd/` ([ADR gsd/0001](../adr/0001-one-cargo-workspace-built-on-demand.md)).
Cargo reads the SDK patch from `gsd/.cargo/config.toml`, so run it **in `gsd/`**:

```sh
cd gsd
cargo build -p gsd-core --release --target wasm32-wasip2      # target/wasm32-wasip2/release/gsd_core.wasm
(cd .. && cargo build --profile test -p specforge-cli)        # the binary the black-box tests drive (optimised deps)
SPECFORGE_BIN=<repo target>/debug/specforge \
GSD_CORE_WASM=$PWD/target/wasm32-wasip2/release/gsd_core.wasm \
  cargo test                                                   # declaration, roster, profile and CLI tests
```

Both variables are **absolute** paths: cargo runs the tests in `gsd/core`, and each test project resolves the component
from its own temp directory, so the harness refuses a relative one. From the repository root, `cargo gate full --only gsd`
runs all of it (clippy for native and wasm, the release build, the CLI build, the tests); `cargo gate quick` runs it when
`gsd/` or any crate changed. Plain `cargo gate --only gsd` is `quick`: on a clean tree nothing changed, so it selects
nothing and runs nothing. The component is built on demand and never committed.

To use it in a project, add the built file:

```sh
specforge init --name my-project
specforge add gsd/target/wasm32-wasip2/release/gsd_core.wasm   # lists @gsd/core in specforge.json
specforge check
specforge settings --extension @gsd/core
```

## Folders

| Folder | What |
|---|---|
| [spec/](spec/) | the model: kinds, fields, identity, settings, rules |
| [contributions/](contributions/) | the command, agent and hook rosters |
| `src/` | the extension crate, built to `wasm32-wasip2`: `kinds/`, `passes/`, `rules.rs`, `codes.rs`, `settings/` |
| `assets/agents/` | the 15 agent bodies ported under MIT from gsd-core v1.18.0 (`gsd-<name>.md`, and `.compact.md` for ten), each starting with its notice; `tests/port.rs` compares them with [the vendored upstream files](../parity/upstream/v1.18.0/SOURCE.md) |
| `tests/` | `declaration.rs` (the declaration through the SDK's `MockHost`), `settings_roster.rs` and `profiles.rs` (against [the frozen upstream roster](../parity/upstream/v1.18.0/SOURCE.md) and the spec), `cli/` (the `specforge` binary on the projects in `tests/projects/`, including `discuss/`, and on the [taskboard](../parity/fixtures/taskboard/)), `declaration_plan.rs` (the plan kinds), `cli/plan*.rs` and `cli/parity_plans.rs` on `projects/plan/` and `projects/gate_cutover/`, `declaration_execute.rs` and `cli/execute*.rs`, `cli/collect.rs`, `cli/progress.rs` on `projects/execute/` (git runs with a pinned identity and dates), and `cli/parity_state.rs` on the taskboard's summaries and recorded tests |
| [../parity/](../parity/) | the upstream roster, the taskboard fixture and the `gsd-tools` goldens |

`LICENSE-GSD` is the upstream licence, kept for the text ported from it.

### Where `health`, `stats` and `progress_report` differ from gsd-tools

The three reports answer from the computed values of the graph a command receives (ADR 0299), not from `.planning/`, so
they differ from `gsd-tools` v1.18.0 in these named ways. Backlog (`999.x`) phases are left out, as upstream and
`milestone.progress` do, and `stats` counts the v1 requirements (upstream's checkbox lines) and says `Not Started`.

| Divergence | `gsd-tools` v1.18.0 | `@gsd/core` |
|---|---|---|
| `health.status`, `errors`, `warnings` | `healthy`, `degraded` or `broken` from its rule table | always `healthy` with both lists empty; only the `I001` info is reproduced, `specforge check` is the health check |
| `/gsd-health --repair`, `--context` | supported | refused: the command takes no argument |
| `milestone_version`, `milestone_name` | `STATE.md` | the milestone's id and name; a workstream-scoped milestone reads `ws/v1.0`, not `v1.0` |
| the current milestone | `STATE.md`, the active workstream | the first `in_progress` milestone by node order, else the last: ambiguous with two workstreams in progress |
| `stats.last_activity` | `STATE.md`'s `last_activity` text | the newest summary `completed` date; `null` with none |
| `stats.git_commits`, `git_first_commit_date` | asks git (`rev-list`) | counts the recorded `commit` entities (`gsd-git`): `0` and `null` with none |
| `stats.requirements_complete` | the `- [x]` checkbox lines | requirements of `status` `complete` (a `gaps_found` one is not counted) |
| `percent`, `plan_percent` | `null` unless the phase scope is complete; capped at 100 | always a number, not capped (a summary never outnumbers its plans here) |
| `progress_report` status of a phase nothing has begun | `Pending` | `Pending` (as `progress json`) |

### Waiting on the kernel

ADR 0299 put the computed values and judgements in the graph a call receives, so a command reads a phase's `status`
and a context's `digest`, and `record_research`, `record_spec`, `record_validation`, `/gsd-health`, `/gsd-stats` and the progress
report are built. K17 closed the E080 treatment of a judged rule's finding, a collector's plan judged before its own report, and the
human-only guarantee on existing entities and deletes (ADR 0297, ADR 0298), and `gsd/E502` is built on top of them.
ADR 0300 serves a commit's source files to the baseline, so `re_verification` reports `previous_status` and
`previous_score` and compares scan states (r39 T13 is built). What still waits for kernel work (K20):

- **A person's answer is recorded by the caller (K20).** `answer_checkpoint`, `confirm_setup`, `set_uat_result`,
  `approve_validation`, `waive_window`, `dispose_refactor` and `pause` write the `H` fields of a person's reply, as
  upstream's agent transcribes the conversation. Each requires a non-empty `--by` (an empty one is refused), but `--by`
  is the caller's **attestation**, not a proof of a person: an agent can record any name (`set_uat_result` echoes it and
  stores it nowhere, a `uat_test` has no `by`). So a gate that such a record satisfies (`gsd/W404`'s answered
  `tdd_review` checkpoint, and the sign-offs r42's gates read) is an honest-agent gate. `override` and `debt_accepted`
  stay human-only because no command writes them. A human-attested command kind (the host refuses an agent-facing call
  and records the caller) is a K20 item; the ignored tests `cmd_attest` carry the desired behaviour.
- **`gsd/E516` is subject-scoped at a command gate (K20).** The host keeps only the diagnostics that name an entity
  inside the gate's subject, and E516 names the stale entity itself. So `start_plan <plan>` is refused by a stale
  verdict inside that plan only, and `close_milestone` by one under the milestone only, not project-wide as the rule
  reads. The fix is for the guard to name the subject (K20); an ignored test carries it.
- **A command or prompt cannot read its extension's state (K20).** The host sends `state` to a `reads_state` command or
  prompt, but the SDK's `CommandCall` and `PromptCall` have no accessor (a hook has `HookCall::facts()`). So
  `/gsd-workstreams status` cannot name the active workstream and `--ws` is text. The fix is an SDK change, so a re-vendor.
- **A rendered step has no agent field (K20)**: the step's title carries the agent. **There is no free-text tail for a
  prompt's arguments (K20)**: `--name` and `--task` are quoted. **An agent cannot create an artifact through the CLI
  (K20)**: `--path` is the CLI's own option, MCP's `specforge.edit` creates it. Each is a wire or CLI change.
- **The hooks wait for five host items (K20)**: git facts in `HookInput`, a `file_changed` event, a hook that runs without
  a full compile, Cursor's `beforeSubmitPrompt`, and a hook reading another extension's settings.
- **The refactor trigger differs from upstream's.** `propose_refactor` records one proposal per function while one is
  untriaged (it answers `nothing new` and writes nothing, so a hook may call it on every phase event), as upstream
  freezes the anchor. It does not port upstream's anchor on first observation: the baseline is the score of the latest
  *disposed* proposal, so a function that creeps below the threshold in small steps never fires `jump` until it crosses.
- **`specforge <ext> help <cmd>` is gone for every extension.** T17 turned off clap's `help` subcommand on an
  extension's command line (the extension's own `help` command shadowed it). Use `specforge <ext> <cmd> --help`;
  `specforge gsd help` lists the commands.
- **A slash command's `when` takes one setting (K20).** A skill in two clusters follows its first, so disabling
  `utility` leaves `/gsd-help` (also `core_loop`) shown.
- **`gsd/W514`** is not built: a pass is sent no settings, and a setting gates a code only while it is on. It needs
  `SettingDescriptor.gates_when_on`.
- **A `file()` input names a file-reference field only.** `gsd/E501` and the plan checks read no file as an input,
  because a plan, an artifact and a key link name files that do not exist yet and the host checks every file-reference
  field for existence (E016). The verifier cites the files it read (`cited`); a file it did not cite can change
  without staling its verdict. Likewise `gsd/E518`'s `requirements` input is every requirement: a selector cannot go
  on past a reference (`phase.milestone.requirements`).
- **A `number` setting type**, so the settings that accept decimals (`hooks.context_*_threshold`) are whole numbers.
- **A JSON number for a top-level integer plan op**: gsd writes the decimal text (`collectors/git.rs`).
- **`collect` re-keys short ids**: `gsd-junit` resolves `[gsd:<id>]` itself.
- **The SDK exports `TriggerKind`**: `gates.rs` reads the kind by name.
- **A collector's plan judged for host errors only** (REVIEW-r38 M1), for `gsd/E515` (`gsd-plus` only).
- **What stays after K17.** `cited` defaults to `[]` and code files are not E501 inputs, so a verdict that cites no file
  never goes stale on a code change; `check` is red from planning on, since gsd/E501 is an Error from the first planned
  truth until it is verified; an agent cannot create an artifact through the CLI (`--path` is the CLI's own option); and
  the kernel reports `(0 findings)` in every judged message (it counts a list of values, not of blocks). `gsd/E502`
  counts the tests recorded for a truth (its `[gsd:<id>]` link), not the tests a verdict cites: a pass is not sent
  `cited_tests`.
- **What E502 guarantees today, and what it does not.** `gsd/E502` is enforced only through E080: a plan that records
  `verified` over tests that fail the truth is refused, and the verifier records `failed`. It does not hold when (a) one
  plan holds two `set_verdict` ops that swap an E502 (one clears an existing one, the other adds one on another truth:
  E080 matches diagnostics by message shape with the digits collapsed, so the multiset is unchanged), which a verdict
  recorded before K17 or a hand edit can make reachable; (b) `derive` reads a fresh `verified` as `Verified` whether or
  not E502 stands, so a verification can read `passed` while the Error is reported (gates still fail on the Error, but
  anything reading `status` does not); (c) `verification` (`explicit` or `backstop`) is agent-writable, so an agent can
  turn the guard off for a truth before it records `verified` (the diff shows it, and the verdict goes stale when the
  field flips). K20: make `derive` treat a truth whose fresh verdict the recorded tests contradict as not `Verified`,
  with the predicate E502 uses; key E080's pairing by subject for pass codes; make `verification` human-only or
  harness-only.
- **A judged code reused by a pass (K20).** The registry refuses a declarative rule that shares a judged rule's code, not
  a pass that `reports` it, nor a guard's fail-closed error for it: E080 exempts those findings (ADR 0297 D1b). No gsd
  pass reports a judged code.

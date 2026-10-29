# `@gsd/core`

**Status: the foundation and the discuss, specify, research, plan and execute kinds are built, with the collectors that bring commits and tests into the project; the verify kinds (verification, UAT, validation, audit, the broken-window ledger) are built, with their computed values and rules; the 15 core agents, the command `record_verdict` and the prompt `spec_phase` are built (r40), and nine judged rules take an agent's verdict (`gsd/W311`, `W313`, `W315`, `W316`, `E501`, `W205`, `E518`, and `E202` and `W203` by the prompt); `gsd/E502` and `gsd/E515` wait for the kernel.** The unified GSD meta-model: project,
requirements, milestones, phases, plans, tasks, decisions, research, results, verification, UAT and validation, with
GSD's checks as rules and its commands, agents, prompts and hooks as contributions. Tracks gsd-core 1.18.0 (the crate's
`[package.metadata.gsd] tracks` and the handshake keyword `tracks:gsd-core@1.18.0`).

It declares no dependency on any `@specforge/*` extension and on no other `@gsd/*` extension; the capability
extensions and the profiles declare it as their peer.

## What it declares

| | Declared now | Where |
|---|---|---|
| Kinds | `project` (one per repository: the kernel keeps `singleton` as metadata, `gsd/E108` enforces it), `requirement` (named by its GSD id), `milestone` (scoped by workstream), `phase` (scoped by milestone, named `03`, `03.1` or `999.1`), `workstream`, `retrospective`, `lesson`, `methodology`; `context` (named `02-CONTEXT`, scoped by its phase), `decision` (`D-01`, scoped by its phase or the project, a project decision is `/D-01` from inside a scope; from outside, a phase's is `v1.0/02/D-01`), `deferred_idea`, `phase_spec`, `locked_req`, `interview` (append-only), `discussion` (its entries append-only), `research` and its eight item kinds (`finding`, `stack_choice`, `pattern`, `pitfall`, `dont_hand_roll`, `open_question`, `source`, `package_audit`), each item scoped by its research; `plan` (named `03-02`, scoped by its phase; a quick plan by its quick id), `task` and `checkpoint` (scoped by their plan, `T1`, `C1`), the must-haves `truth`, `artifact`, `key_link` and `prohibition`, `disposition` (append-only) and `user_setup`; `summary` (scoped by its plan), `deliverable` (scoped by its summary, with computed check statuses), `deviation` (append-only), `commit` (written only by the git collector), `pause`, `handoff` and `blocker`; `verification`, `verify_gap`, `override` (human-only), `human_item`, `uat_session`, `uat_test`, `uat_gap`, `validation`, `validation_row`, `wave0_item`, `manual_check`, `milestone_audit` (its `debt_accepted` only a human sets), `broken_window` and `refactor_proposal`: 54 kinds in all | `src/kinds/`, model in [spec/project.md](spec/project.md), [spec/discuss.md](spec/discuss.md), [spec/plan.md](spec/plan.md) and [spec/execute.md](spec/execute.md) |
| Agents | the 15 core agents of gsd-core 1.18.0, bodies ported under MIT (`assets/agents/`), their tools mapped to host tool classes, ten with a compact variant (`specforge.agent_variant`) and each with a model line from `model_overrides.<agent>`. `specforge install <host>` writes them; a judge's file also lists the rules it judges: `gsd-plan-checker` (`gsd/W311`, `W313`, `W315`, `W316`), `gsd-verifier` (`gsd/E501`), `gsd-phase-researcher` (`gsd/W205`), `gsd-integration-checker` (`gsd/E518`); the other eleven judge nothing. A judge's body ends with a section that names `record_verdict`. **The bodies are installable, not yet runnable:** their ported lines still name upstream's tools (73 `specforge gsd query`, 13 `specforge gsd.cjs`, 133 `spec/gsd/` paths that do not exist, and a verifier told to write VERIFICATION.md), pinned by `tests/port.rs` so the counts can only fall; r41 (commands) and r42 (workflows) rewrite them. The attribution notice always names the full file `agents/<id>.md`, also when the compact body renders | `src/agents/`, `src/judged.rs` |
| Command, prompt | `record_verdict` (`specforge gsd record-verdict`) records a judge's verdict through `set_verdict`; the prompt `spec_phase` judges `gsd/E202` and `gsd/W203` | `src/commands/`, `src/prompts.rs` |
| Collectors | `gsd-git` records each commit that names a plan or task as a `commit` entity in its plan's file: `specforge collect --runner gsd-git --yes` (consented `git log`). `gsd-junit` reads a JUnit report and links a test to its entity by a `[gsd:<id>]` tag, and to a deliverable's check by GSD's own `path#test name` reference: `specforge collect --runner gsd-junit --no-run` (or `--report <file>`). Until the first `collect`, `specforge check` says I205 (information: no test report) | `src/collectors/` |
| Passes | `derive` (the computed fields: each context's `digest`; verification status, score and re-verification, UAT totals and status, validation coverage, audit status, the phase `completed` ladder and the window counts; plan, task, summary and deliverable-check status from the commits and the test report; requirement, phase, milestone, project, workstream and context values), `execute` (`gsd/E401`, `gsd/W403`, `gsd/W407`, `gsd/W408`), `schema_drift` (`gsd/E405`), `roadmap` (`gsd/E101`, `gsd/E102`, `gsd/W103`, `gsd/W104`, `gsd/E108`, `gsd/E206`), `research` (`gsd/W204`), `drift` (`gsd/W406`), `plan_shape` (`gsd/W323`, `gsd/E302`, `gsd/E303`, `gsd/W304`, `gsd/E307`, `gsd/W308`, `gsd/W309`, `gsd/E322`), `waves` (the computed `plan.wave` and `plan.autonomous`), `file_scope` (`gsd/W305`, `gsd/W306`), `scope` (`gsd/E312`, `gsd/W317`), `cross_plan` (`gsd/E301`, `gsd/E310`, `gsd/I321`), `scan` (computes the file facts: anti-patterns, debt markers, artifact and key-link states) with `scan_check` (`gsd/E505`), `nyquist` (`gsd/E314`, `gsd/W510`), `verify_rules` (`gsd/E503`, `gsd/W504`, `gsd/E509`), `windows` (`gsd/E513`), `gates` (the guards `gsd/E106`, `gsd/E511`, `gsd/E512`, `gsd/W318`, `gsd/W404`, `gsd/E516`, which answer at a gate trigger and are silent on a write; `gsd/W514` is not built) and `verify_paths` (`gsd/E320`, `gsd/W324`; computes `task.verify_paths`, which it reads through the project's files) and `fails_when` (`gsd/E507`) | `src/passes/` |
| Rules | `gsd/E105` (an `out_of_scope` requirement has a reason), `gsd/E201` (a context has canonical refs: two declarative rules under one code), `gsd/E107` (generated for each authored timestamp field and block member: first `blocker.resolved.at`), `gsd/E402` (a halted summary says why), the seven declarative `gsd/E307` rules (what each checkpoint kind requires) and `gsd/E319` (a test-tier prohibition names a test) | `src/rules.rs`, `src/kinds/dsl.rs` |
| Code book | all 63 `gsd/` codes of [spec/rules.md](spec/rules.md), each with its `gsd-compat` severity and what `gsd-strict` raises it to; the rules of the others land with their kinds | `src/codes.rs` |
| Settings | the 181 GSD configuration keys `@gsd/core` owns, typed with GSD's default, and 12 families (`models.*`, `granularities.*`, …), `specforge.agent_variant`; the 15 `model_overrides.<agent>` keys belong to the `model_overrides.*` family and are set by a preset | `src/settings/keys.rs`, model in [spec/settings.md](spec/settings.md) |
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
(cd .. && cargo build -p specforge-cli)                        # the binary the black-box tests drive
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

### Waiting on the kernel

Six things r39 and r40 could not build, or could not guarantee, wait for kernel work (K17):

- **Baseline re-verification (T13).** `re_verification.previous_status` and `previous_score` are not built, and the
  baseline view of an artifact or key link is not used. The git adapter loads only `*.spec` and `specforge-harness` at a
  commit, so at the baseline every source file reads as absent (`exists: false`): the scan says `missing`, never
  `unchecked`, and the baseline's status and score are wrong. `re_verification` therefore compares fresh verdicts and
  verify gaps only. The baseline compile must serve source files from the commit through the on-demand blob fetch
  (with `baseline-format` 3). Its `regressions`, a verdict-based `gaps_closed` and a reopened gap are tested on
  synthetic baselines, not black-box (the judgement list is empty on a baseline).
- **`gsd/E502`** is not built (deferred, not blocked). Its first reason, "recording a `verified` while a failing test is
  on record would be refused by E080", is wrong: that refusal is what E502 is for. The probe the plan asked for
  (`tests/cli/e502_probe.rs`) shows the other order cannot trip it either: a failing report collected after a pass
  changes the truth's `tests` input, the verdict goes stale, and E502 needs a *fresh* passing judgement. So E502 can be
  built as a `reads_tests` check. Until it is, `record_verdict` takes a `verified` on an explicit truth whose recorded
  tests include a failing one, and the verification can pass: the fail-open E502 exists to close
  (`a_verified_truth_with_a_failing_test_records_because_e502_is_not_built`, and the ignored
  `e502_refuses_a_verified_truth_with_a_failing_test`). `covered_digest` is built, and the commit titled for E502
  (`gsd/E502 an explicit truth is verified only with a named passing test, and the verification digests what it covered`)
  built only that.
- **`gsd/W514`** is not built: a pass is sent no settings, and a setting gates a code only while it is on. It needs
  `SettingDescriptor.gates_when_on`.
- **A verdict's own state counts as a new error (E080).** The host re-checks the project an edit would leave and keys
  a diagnostic by its code, severity, file and message shape (`crates/app/ops/src/authoring/edit/validate.rs`). A
  judged rule's message changes with its verdict (`...: no verdict`, `...: the verdict of <date> is '<outcome>' (<n>
  findings)`, `...: is stale: ...`), and a **recorded `passed` emits no diagnostic at all**. Everything below follows,
  and each case is pinned by a live test with an `#[ignore = "K17"]` twin (`tests/cli/judged_refusals.rs`,
  `judged.rs`, `verify_verdicts.rs`, `discuss.rs`):
  - **A failing verdict is refused whole** when the rule's effective severity is an Error: gsd/E501 `failed`,
    `uncertain` and `present_behavior_unverified`, gsd/E518 `issues`, gsd/E202 `failed`, and under `gsd-strict` gsd/W311,
    W313, W315 `issues`, gsd/W203 `failed` and gsd/W205 `suspicious` or `unknown`:
    `error[E080]: the edit would leave the project with 1 error it does not have; nothing was written`.
  - **A passing verdict cannot be withdrawn through `record_verdict`.** A `failed` (or `issues`) that would replace a
    fresh `verified` (or `passed`) is the same refusal, so a judge that finds a regression cannot say so: the old pass
    stays fresh, the verification stays `passed` and the phase `complete`, and a `checked` plan stays `checked`. This is
    **fail-open on revocation**. Code files are not E501 inputs, so a pass stops counting only when something it read
    changed: its truth, its plan, a file it **cited**, or a test result. Cite the files you judged. Today only a person
    editing the `.spec` by hand removes a pass.
  - **Creating a judged target is refused** through any typed agent-facing write (MCP `specforge.new`, a piped CLI, an
    extension command): a new truth, artifact, key link, `phase_spec` or `milestone_audit` adds a `no verdict` Error of
    a shape already present, and under `gsd-strict` so does a new plan (W311, W313, W315). A gap-closure plan's truths
    cannot be created by an agent at all. An agent that writes `.spec` files directly bypasses E080.
  - **An edit that stales a passing Error verdict is refused** (a truth's text, its plan's fields, a checked plan
    under strict). The worst case is gsd/E518: its `requirements` input is every requirement, so once a milestone
    audit's integration verdict passes, every requirement edit and creation is refused.
  - `cited` defaults to `[]` and code files are not E501 inputs, so a verdict that cites no file never goes stale on a code
    change (`record_verdict` does not require a citation: a truth verified by tests alone has no file to cite).
  - `check` is red from planning on: gsd/E501 is an Error from the first planned truth until it is verified.
  - The kernel also reports `(0 findings)` in every judged message (it counts a list of values, not of blocks).
  `@gsd/core` therefore ships fail-closed only when no verdict exists yet; once a pass exists it cannot be revoked. The
  smallest kernel change (K17): in `validate.rs::check`, a diagnostic whose code is a judged rule's goes to the
  reported-not-refused remainder, because it records a verdict that is owed, not a defect of the edit. Status
  derivations stay fail-closed, since they read `Judgement.passed`, never the diagnostic.
- **A `file()` input names a file-reference field only.** `gsd/E501` and the plan checks read no file as an input,
  because a plan, an artifact and a key link name files that do not exist yet and the host checks every file-reference
  field for existence (E016). The verifier cites the files it read (`cited`); a file it did not cite can change
  without staling its verdict. Likewise `gsd/E518`'s `requirements` input is every requirement: a selector cannot go
  on past a reference (`phase.milestone.requirements`).
- **Human-only on existing entities.** Closed for `override` in r40: the host's `edit_flags` checks the kind's writer
  on create, delete and retitle, and a *field's* writer on `set`, `unset` and `link`, so every field of `override` is
  marked human and an agent can no longer change or re-point one
  (`an_agent_cannot_edit_an_existing_override`). What stays: an agent can delete a milestone audit that carries a
  person's `debt_accepted` (gsd/E106 then refuses the gate), because the kind is not human; the general fix is K17.


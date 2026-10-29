# `@gsd/core`

**Status: the foundation and the discuss, specify, research, plan and execute kinds are built, with the collectors that bring commits and tests into the project; verification is not.** The unified GSD meta-model: project,
requirements, milestones, phases, plans, tasks, decisions, research, results, verification, UAT and validation, with
GSD's checks as rules and its commands, agents, prompts and hooks as contributions. Tracks gsd-core 1.18.0 (the crate's
`[package.metadata.gsd] tracks` and the handshake keyword `tracks:gsd-core@1.18.0`).

It declares no dependency on any `@specforge/*` extension and on no other `@gsd/*` extension; the capability
extensions and the profiles declare it as their peer.

## What it declares

| | Declared now | Where |
|---|---|---|
| Kinds | `project` (one per repository: the kernel keeps `singleton` as metadata, `gsd/E108` enforces it), `requirement` (named by its GSD id), `milestone` (scoped by workstream), `phase` (scoped by milestone, named `03`, `03.1` or `999.1`), `workstream`, `retrospective`, `lesson`, `methodology`; `context` (named `02-CONTEXT`, scoped by its phase), `decision` (`D-01`, scoped by its phase or the project, a project decision is `/D-01` from inside a scope; from outside, a phase's is `v1.0/02/D-01`), `deferred_idea`, `phase_spec`, `locked_req`, `interview` (append-only), `discussion` (its entries append-only), `research` and its eight item kinds (`finding`, `stack_choice`, `pattern`, `pitfall`, `dont_hand_roll`, `open_question`, `source`, `package_audit`), each item scoped by its research; `plan` (named `03-02`, scoped by its phase; a quick plan by its quick id), `task` and `checkpoint` (scoped by their plan, `T1`, `C1`), the must-haves `truth`, `artifact`, `key_link` and `prohibition`, `disposition` (append-only) and `user_setup`; `summary` (scoped by its plan), `deliverable` (scoped by its summary, with computed check statuses), `deviation` (append-only), `commit` (written only by the git collector), `pause`, `handoff` and `blocker`: 40 kinds in all | `src/kinds/`, model in [spec/project.md](spec/project.md), [spec/discuss.md](spec/discuss.md), [spec/plan.md](spec/plan.md) and [spec/execute.md](spec/execute.md) |
| Collectors | `gsd-git` records each commit that names a plan or task as a `commit` entity in its plan's file: `specforge collect --runner gsd-git --yes` (consented `git log`). `gsd-junit` reads a JUnit report and links a test to its entity by a `[gsd:<id>]` tag, and to a deliverable's check by GSD's own `path#test name` reference: `specforge collect --runner gsd-junit --no-run` (or `--report <file>`). Until the first `collect`, `specforge check` says I205 (information: no test report) | `src/collectors/` |
| Passes | `derive` (the computed fields: each context's `digest`; plan, task, summary and deliverable-check status from the commits and the test report; requirement, phase, milestone, project, workstream and context values), `execute` (`gsd/E401`, `gsd/E403`, `gsd/W407`, `gsd/W408`), `schema_drift` (`gsd/E405`), `roadmap` (`gsd/E101`, `gsd/E102`, `gsd/W103`, `gsd/W104`, `gsd/E108`, `gsd/E206`), `research` (`gsd/W204`), `drift` (`gsd/W406`), `plan_shape` (`gsd/W323`, `gsd/E302`, `gsd/E303`, `gsd/W304`, `gsd/E307`, `gsd/W308`, `gsd/W309`, `gsd/E322`), `waves` (the computed `plan.wave` and `plan.autonomous`), `file_scope` (`gsd/W305`, `gsd/W306`), `scope` (`gsd/E312`, `gsd/W317`), `cross_plan` (`gsd/E301`, `gsd/E310`, `gsd/I321`), `verify_paths` (`gsd/E320`, `gsd/W324`; computes `task.verify_paths`, which it reads through the project's files) and `fails_when` (`gsd/E507`) | `src/passes/` |
| Rules | `gsd/E105` (an `out_of_scope` requirement has a reason), `gsd/E201` (a context has canonical refs: two declarative rules under one code), `gsd/E107` (generated for each authored timestamp field and block member: first `blocker.resolved.at`), `gsd/E402` (a halted summary says why), the seven declarative `gsd/E307` rules (what each checkpoint kind requires) and `gsd/E319` (a test-tier prohibition names a test) | `src/rules.rs`, `src/kinds/dsl.rs` |
| Code book | all 63 `gsd/` codes of [spec/rules.md](spec/rules.md), each with its `gsd-compat` severity and what `gsd-strict` raises it to; the rules of the others land with their kinds | `src/codes.rs` |
| Settings | the 181 GSD configuration keys `@gsd/core` owns, typed with GSD's default, and 12 families (`models.*`, `granularities.*`, …) | `src/settings/keys.rs`, model in [spec/settings.md](spec/settings.md) |
| Profiles | `profile` (`gsd-compat` by default, `gsd-strict`, `gsd-plus`), presetting five settings and raising rule severities | `src/settings/profiles.rs` |
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
| `tests/` | `declaration.rs` (the declaration through the SDK's `MockHost`), `settings_roster.rs` and `profiles.rs` (against [the frozen upstream roster](../parity/upstream/v1.18.0/SOURCE.md) and the spec), `cli/` (the `specforge` binary on the projects in `tests/projects/`, including `discuss/`, and on the [taskboard](../parity/fixtures/taskboard/)), `declaration_plan.rs` (the plan kinds), `cli/plan*.rs` and `cli/parity_plans.rs` on `projects/plan/` and `projects/gate_cutover/`, `declaration_execute.rs` and `cli/execute*.rs`, `cli/collect.rs`, `cli/progress.rs` on `projects/execute/` (git runs with a pinned identity and dates), and `cli/parity_state.rs` on the taskboard's summaries and recorded tests |
| [../parity/](../parity/) | the upstream roster, the taskboard fixture and the `gsd-tools` goldens |

`LICENSE-GSD` is the upstream licence, kept for the text ported from it.

# `@gsd/core`

**Status: the foundation is built; plans, execution and verification are not.** The unified GSD meta-model: project,
requirements, milestones, phases, plans, tasks, decisions, research, results, verification, UAT and validation, with
GSD's checks as rules and its commands, agents, prompts and hooks as contributions. Tracks gsd-core 1.18.0 (the crate's
`[package.metadata.gsd] tracks` and the handshake keyword `tracks:gsd-core@1.18.0`).

It declares no dependency on any `@specforge/*` extension and on no other `@gsd/*` extension; the capability
extensions and the profiles declare it as their peer.

## What the foundation declares

| | Declared now | Where |
|---|---|---|
| Kinds | `project` (one per repository: the kernel keeps `singleton` as metadata, `gsd/E108` enforces it), `requirement` (named by its GSD id), `milestone` (scoped by workstream), `phase` (scoped by milestone, named `03`, `03.1` or `999.1`), `workstream`, `retrospective`, `lesson`, `methodology` | `src/kinds/`, model in [spec/project.md](spec/project.md) |
| Passes | `derive` (the computed fields), `roadmap` (`gsd/E101`, `gsd/E102`, `gsd/W104`, `gsd/E108`) | `src/passes/` |
| Rules | `gsd/E105` (an `out_of_scope` requirement has a reason), `gsd/E107` (generated for each authored timestamp field; no r34 kind has one, `project.updated` is computed, so it first fires in the round that adds one) | `src/rules.rs`, `src/kinds/dsl.rs` |
| Code book | all 60 `gsd/` codes of [spec/rules.md](spec/rules.md), each with its `gsd-compat` severity and what `gsd-strict` raises it to; the rules of the others land with their kinds | `src/codes.rs` |
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
GSD_CORE_WASM=target/wasm32-wasip2/release/gsd_core.wasm \
  cargo test                                                   # declaration, roster, profile and CLI tests
```

From the repository root, `cargo gate --only gsd` runs all of it (clippy for native and wasm, the release build, the
CLI build, the tests) and `cargo gate quick` runs it when `gsd/` changed. The component is built on demand and never
committed.

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
| `tests/` | `declaration.rs` (the declaration through the SDK's `MockHost`), `settings_roster.rs` and `profiles.rs` (against [the frozen upstream roster](../parity/upstream/v1.18.0/SOURCE.md) and the spec), `cli/` (the `specforge` binary on the projects in `tests/projects/` and on the [taskboard](../parity/fixtures/taskboard/)) |
| [../parity/](../parity/) | the upstream roster, the taskboard fixture and the `gsd-tools` goldens |

`LICENSE-GSD` is the upstream licence, kept for the text ported from it.

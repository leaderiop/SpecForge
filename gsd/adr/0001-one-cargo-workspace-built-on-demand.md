# The @gsd family is one Cargo workspace under gsd/, built on demand and tested through the specforge binary

**Status:** accepted (2026-10-28, `@gsd/core` round r34). Extended by
[ADR gsd/0002](0002-a-capability-extension-plugs-into-core-at-its-hook-points.md) (the capability extensions). Extends
[ADR 0277](../../docs/adr/0277-a-scaffold-builds-where-it-is-written.md) (an extension names the SDK as crates.io would
and builds against an unreleased one with `[patch.crates-io]`) and
[ADR 0292](../../docs/adr/0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (a prompt or a body has no
size limit). Kernel ADRs stay in `docs/adr/`; the family's own live here and move with the folder.

## Context

`gsd/` holds the model, the rosters and the parity plan of the GSD framework in SpecForge, and is to move to its own
repository whole (`gsd/README.md`): nothing in it may name a path outside `gsd/` in a manifest. The first extension,
`@gsd/core`, is built now. It embeds about 2.8 MB of ported upstream text, changes on every prompt edit, and its tests
need the `specforge` binary because the in-process test runtime is not published.

## D1. `gsd/` is one Cargo workspace

`gsd/Cargo.toml` (members `core` now, the other `@gsd/*` later; one `Cargo.lock`, one `target/`, one `[workspace.lints]`
copied from the root, edition 2024). The root workspace already excludes `gsd`. The family shares one lock and one target
directory, and the folder moves whole.

## D2. `@gsd/core` is the crate `gsd/core`

Package `gsd-core`, `crate-type = ["cdylib", "rlib"]` (the `rlib` lets native tests link), artifact `gsd_core.wasm`,
declared `name = "@gsd/core"`, `short = "gsd"`, version `0.1.0`.

## D3. The SDK is named as crates.io would, and `gsd/.cargo/config.toml` patches it

`specforge-extension-sdk = "0.3.0"` (with `features = ["testing"]` in dev), and a `[patch.crates-io]` to
`../crates/runtime/extension-sdk`. This is the documented way to build against an unreleased SDK. The manifest never
names a path outside `gsd/`, and the split deletes one config line. Cargo reads `.cargo/config.toml` from its working
directory, not from `--manifest-path`, so every cargo command of the family runs in `gsd/` (from the repository root,
`--manifest-path gsd/Cargo.toml` would not see the patch and fail to select a version, as crates.io has no 0.3.0 yet).

## D4. The blob is not committed; it is built on demand

`cd gsd && cargo build -p gsd-core --release --target wasm32-wasip2`, with the scaffold's release
profile (opt-level z, fat LTO, strip). The kernel's committed-blob machinery (`COMMITTED`, revendor, the 1 MB budget) is
for its builtins and fixtures; listing `@gsd/core` there would tie the kernel's revendor to a domain extension.

## D5. Tests at two depths, both inside `gsd/`

Native unit tests and `MockHost` declaration goldens (`cargo test`, the SDK's published `testing` feature), and black-box
tests that run the `specforge` binary (`SPECFORGE_BIN`) on small projects that load the built blob (`GSD_CORE_WASM`). A
missing variable fails the test naming the command that sets it.

## D6. CI is one `cargo gate` check, `gsd`

Reach `quick`, so `cargo gate quick`, `full` and `ci` run it; it is triggered by `gsd/**`, every crate under `crates/**`
(the tests run through the `specforge` binary, which is built from all of them, the SDK included) and the root manifest and
lockfile. `cargo gate ci` ignores triggers. `cargo gate --only gsd` is `quick`, and `quick` selects by the change set, so on
a clean tree it runs nothing: use `cargo gate full --only gsd`. Steps: clippy native and wasm (`-D warnings`), the wasm
build, the `specforge` CLI build (the test profile: optimised dependencies), `cargo nextest run` with both variables set
(absolute paths); every cargo step is `--locked`. CI runs the check on a runner of its own (the `gsd` shard).
`gsd/Cargo.toml` joins `OUTSIDE_WORKSPACE`, so the existing `rustfmt` check covers it, and the fixture `.spec` directories
join `spec-format`.

## D7. A test project loads the blob by path

The harness copies a project from `gsd/core/tests/projects/<name>/` into a temp dir and writes
`"extensions": ["@gsd/core=<GSD_CORE_WASM>"]`. A user runs `specforge add <path to gsd_core.wasm>` (the built component;
`specforge add gsd/core` names a crate directory and fails with E040).

## D8. The parity fixture is `gsd/parity/fixtures/taskboard/`

A small Node CLI (`node:test`, whose JUnit reporter needs no dependency), its `.planning/` frozen, the deterministic
`gsd-tools` answers over it recorded as goldens by `gsd/parity/record.sh` (run by hand with `GSD_CORE_DIR` and node, never
in CI), and a hand-written `.spec` twin until `@gsd/import` exists. The `.planning/` is a faithful hand-built tree, not the
output of an agent run: the narrative files (`PROJECT.md`, `REQUIREMENTS.md`, plans, summaries, verification, UAT) were
written by hand to GSD's templates and only the CLI verbs that write files were run by real gsd-core 1.18.0
(`gsd/parity/README.md`, "How the taskboard was made"). What the oracle proves is gsd-core 1.18.0's answers over that tree.

## D9. Ported text lives in `gsd/core/assets/`

Mirroring the upstream path, each file headed with its upstream path, version and MIT notice, `include_str!`-embedded;
`gsd/core/LICENSE-GSD` holds upstream's licence; `gsd/parity/upstream/v1.18.0/` keeps a snapshot of the upstream data the
roster checks read.

## D10. The family's ADRs are `gsd/adr/NNNN-*.md`, from 0001

## Rejected

- **The SDK by git revision**: it cannot see a kernel follow-up made in the same round, which the rounds after r35 need.
- **A committed blob listed in `COMMITTED`**: see D4.
- **Tests on `InProcessRuntime` and `specforge-test`**: they are unpublished, so such a harness would not survive the
  split; the binary is also what users run, so the black-box tests are the parity suite's engine.
- **The family's ADRs in `docs/adr/`**: they would not move with the folder.

## Consequences

A kernel SDK or protocol change that breaks `@gsd/core` fails in the kernel pull request that made it. A workspace
version bump must bump `gsd/core/Cargo.toml` in the same change, or `[patch.crates-io]` no longer applies and the `gsd`
check fails, by design. The check builds the CLI and a release wasm (about 2 to 3 minutes cold), so it is reached by
`full` and `ci` and by `quick` only when a trigger path changed.

## What would reopen it

A second `@gsd/*` crate whose build time outgrows one check, or the SDK's testing runtime being published (then the
black-box layer shrinks to the parity suite).

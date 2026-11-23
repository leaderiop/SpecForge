# Re-vendoring the builtins

The builtin extensions and the test fixtures ship as wasm blobs committed under `<crate>/wasm/`, with the hash of everything
they compile from in `inputs.json`. A change to those inputs makes the check list's `builtins` check (`build-blobs --check`) fail until
the tree is re-vendored.

## When

A change to any file an `inputs.json` records: the guests' sources, the SDK and its macros, protocol-types, coverage, cycles,
the root manifest's inherited tables. `cargo gate` runs the `builtins` check exactly then, and `build-blobs --check`
names each changed input.

## How

```text
cargo run -p xtask --bin revendor
```

Run it from the main checkout, or from a worktree outside the repository directory, with `CARGO_TARGET_DIR` unset
(it refuses otherwise; the blobs build in `target/blob-stage/`). It takes about five minutes (5 min 17 s when it first
vendored `@gsd/core`) and needs the pinned toolchain with its target, not only the target:

```text
rustup toolchain install 1.99.1 --profile minimal --target wasm32-wasip2
```

Every committed blob is built on that one rustc, `BLOB_RUSTC` in `gate/src/blob_inputs.rs` (ADR 0373 D1): the stager
runs every cargo and rustc of a blob build on it whatever the checkout's own toolchain is, `inputs.json` records it as
`built_with`, and `build-blobs --verify` fails on a blob another rustc built. A change of the pin re-vendors every blob
in its commit.

## Build on linux/amd64, not macOS

The bootstrap job rebuilds every blob on the Linux runner and compares the bytes (`build-blobs --verify`). A macOS
build of the same sources is not byte-identical: on a clean `main` it reports `[differs]` for every blob, and a
`gsd_core.wasm` built there failed bootstrap (PR #185). Re-vendor on linux/amd64 and keep only the blobs whose
inputs changed. From a plain clone (a worktree's `.git` points at a host path the container cannot see):

```text
docker run --rm --platform linux/amd64 -v "$PWD":/work -v sf-target:/work/target -v sf-cargo:/usr/local/cargo/registry \
  -w /work -e CARGO_HOME=/usr/local/cargo -e CARGO_INCREMENTAL=0 rust:1.99.1 bash -c \
  'git config --global --add safe.directory "*" && rustup toolchain install 1.99.1 --profile minimal --target wasm32-wasip2 \
   && cargo run -p xtask --bin revendor'
```

Emulated amd64 takes about 15 minutes. Every blob whose inputs did not change comes out identical, so `git status`
shows only the blob you meant to change.

It refreshes the lockfile of every committed blob's crate, rebuilds every committed blob
(`specforge_component::committed::COMMITTED`: every builtin, the four fixtures and the marketing example's extension) into `<crate>/wasm/` with its
`inputs.json`, rewrites the declaration snapshots of every SDK blob in
`crates/runtime/component/tests/declarations/` and the builtin vocabulary
(`crates/runtime/component/data/builtin-vocabulary.json`, ADR 0122), runs `build-blobs --check` and the declarations test (with its
ignored tests), and prints the `git add` line that stages exactly those paths.

A new committed blob is listed in `crates/runtime/component/src/committed.rs` first; a new builtin is a row of
`BUILTINS` (`crates/runtime/component/src/builtins.rs`), which `COMMITTED` lists first. A builtin's crate is
`extensions/<dir>` (`@specforge/<dir>`) or, in a family, `extensions/<family>/<pack>` (`@<family>/<pack>`), with one
exception: `@gsd/core`'s crate is `gsd/core`, a named row of `BUILTINS` (ADR 0366 D1). Its lock is the gsd workspace's
`gsd/Cargo.lock`, shared with `gsd-kit` and `gsd-security` and hashed whole, so a dependency change that only touches
`gsd-security` still re-vendors it; its SDK patch comes from `gsd/.cargo/config.toml`, which the stager replays
re-rooted at the repository (ADR 0366 D2, `gate/src/blob_inputs.rs`). The printed `git add` line then holds
`gsd/core/wasm` and `gsd/Cargo.lock`. A crate under `extensions/`
without `wasm/` is an extension library (`extensions/code/junit`, `extensions/code/facts`, `extensions/rust/manifest`,
`extensions/javascript/workspace`),
an rlib no blob of its own. A library is an input of every builtin that links it, directly or through another library,
so editing one re-vendors those builtins: code-facts, and code-junit, which it links, feed `@code/core`,
`@javascript/node-test`, `@javascript/source`, `@rust/cargo`, `@rust/source`, `@rust/test` and the twelve report packs
(`@rust/{coverage,lint,deps,api,proof}`, `@javascript/{coverage,lint,deps,api,vitest,jest,playwright}`);
`specforge-rust-manifest` feeds `@rust/cargo`; `specforge-javascript-workspace` feeds `@javascript/node-test`,
`@javascript/source`, `@javascript/vitest`, `@javascript/jest` and `@javascript/playwright`.
The report readers `extensions/code/{lcov,api,sarif,cyclonedx,osv}` and `extensions/code/kani` each feed the packs that
link them (`a_change_to_a_report_reader_feeds_the_packs_that_link_it`, ADR 0389).
`every_committed_wasm_is_listed` (xtask) fails for a `.wasm` file git tracks that the list does not name.

## Only the vocabulary

```text
cargo run -p xtask --bin revendor -- --vocabulary
```

When the vocabulary's rule or its written form changed and no blob input did (the `builtins` check passes), this
rewrites `crates/runtime/component/data/builtin-vocabulary.json` from the pinned declarations and builds nothing:
no lockfile, no blob and no snapshot changes, and it runs from any checkout. A kind keyword two builtins declare is
listed with both, sorted (ADR 0369). A field two builtins add to one kind stops it, with an error that names both.

## Profile

The blobs are built with `--profile blob` (root `Cargo.toml`: opt-level `z`, fat LTO, one codegen unit, strip), not with the
workspace's `release`: the nine builtins of the time went from 7.0 MB to 3.1 MB, which also shrinks the CLI that embeds them. The table is
an input of every blob (`Cargo.toml#workspace-inherited`), so editing it re-vendors them. Trap frames lose function names;
to debug a guest, rebuild its crate with a plain `cargo build --release`.

## Review

Declaration snapshots should change only when the change meant to change a declaration; a snapshot diff nobody
intended is a bug. The vocabulary changes only when a builtin's kinds or enhancement fields change. The doctor and MCP snapshots (accept with `INSTA_UPDATE=always`, see `briefs/_shared.md`) follow the declarations.

Run it once per round or change set, after the last input changed, then `cargo gate full`. Within a series of commits,
the `builtins` check is the one check an intermediate commit may fail: re-vendor at the last commit that changes an input,
and earlier only when a commit's own tests need new blobs (a wire change that stale blobs refuse). The tip passes every check.

## Static tables

The tables of `specforge-checks-kit` (`crates/leaf/checks-kit/src/tables/`) are ours, each with a pinned `SOURCE` (ADR 0338 D7).
**Before every release**, refresh each from its source, update its `SOURCE` and its pins
(`every_table_names_its_pinned_source`, `every_table_pins_its_rows`, `iban_table_source_is_pinned`) in the same commit, then
re-vendor. A row's edit without them fails those tests, so a silent edit shows.

| Table | Source | Bound |
|---|---|---|
| `iso3166` | ISO 3166-1, through the mirror its `SOURCE` names | about 250 rows |
| `iso4217` | ISO 4217 list one (SIX) | about 180 rows |
| `iana_tz` | the tzdb release's `tzdata.zi`: `Z` lines are canonical, `L` lines are links | about 600 names |
| `e164` | ITU-T E.164, through libphonenumber's map | at most 4 KiB |
| `iban` | the ISO 13616 registry, through the mirror its `SOURCE` names | about 100 rows |
| SPDX licences | the `spdx` crate: `cargo update -p spdx` | its version is pinned by a test |

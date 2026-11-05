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
(it refuses otherwise; the blobs build in `target/blob-stage/`). It takes about four minutes and needs
`rustup target add wasm32-wasip2`.

It refreshes the lockfile of every committed blob's crate, rebuilds every committed blob
(`specforge_component::committed::COMMITTED`: every builtin, the four fixtures and the marketing example's extension) into `<crate>/wasm/` with its
`inputs.json`, rewrites the declaration snapshots of every SDK blob in
`crates/runtime/component/tests/declarations/` and the builtin vocabulary
(`crates/runtime/component/data/builtin-vocabulary.json`, ADR 0122), runs `build-blobs --check` and the declarations test (with its
ignored tests), and prints the `git add` line that stages exactly those paths.

A new committed blob is listed in `crates/runtime/component/src/committed.rs` first; a new builtin is a row of
`BUILTINS` (`crates/runtime/component/src/builtins.rs`), which `COMMITTED` lists first. A builtin's crate is
`extensions/<dir>` (`@specforge/<dir>`) or, in a family, `extensions/<family>/<pack>` (`@<family>/<pack>`); a crate there
without `wasm/` is an extension library (`extensions/code/junit`), an rlib no blob of its own. A library is an input of
every builtin that links it, so editing it re-vendors those builtins.
`every_committed_wasm_is_listed` (xtask) fails for a `.wasm` file git tracks that the list does not name.

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

# A project example's own extension is a committed blob

**Status:** accepted (2026-10-26, architecture round 29, plan 05). Amends [ADR 0070](0070-every-committed-blob-is-listed-once-and-built-one-way.md)
D1 and D2, and its consequence that the committed blobs are the builtins and the fixtures.

The marketing example (`examples/marketing`) loads its own extension from a committed `marketing.wasm`, a hand copy of
an in-place `cargo build --release`. Nothing compared it with `extension/src/lib.rs` or the SDK it compiles: an edited
message template never reached `check` while every gate check passed. The in-place build cannot reproduce the committed
bytes from another checkout (the SDK's absolute path reaches `-C metadata`), so only a recorded-inputs check or a staged
rebuild can compare it. And `git ls-files '*.wasm'` held 14 files while `COMMITTED` listed 13, so
`every_committed_wasm_is_listed` failed. The blob stage could not build the crate either: its own `[workspace]` table
made cargo refuse the stage ("multiple workspace roots"). The gate's `dir/*/rest` patterns dropped `rest`, so
`examples/*/extension/Cargo.toml` would have matched nothing.

## D1. An example's extension is listed

`specforge_component::committed::EXAMPLES` lists each example project's own extension (today `MARKETING`,
`examples/marketing/extension`), and `COMMITTED` is the builtins, the fixtures, then the examples. It is vendored at
`<crate>/wasm/<artifact>` beside its `inputs.json` (ADR 0070 D2's one layout), built by `revendor` in the stage, checked by
`build-blobs --check` and `--verify`, and its declaration is pinned under `tests/declarations/marketing/`. The project's
entry is `@example/marketing=extension/wasm/marketing.wasm`.

## D2. A staged crate has no workspace or profile of its own

The stage drops every staged manifest's `[workspace]`, `[workspace.*]` and `[profile.*]` tables, as it drops the
dev-dependency tables: a staged crate is a member of the stage's one workspace and is built with its `[profile.blob]`.
The example keeps both tables, so a plain `cargo build` in place still works.

## D3. One expansion rule

`gate::repo::expand` expands `prefix/*/rest` and keeps `rest`. `OUTSIDE_WORKSPACE` (rustfmt), the guests' manifests and
`blob_inputs::BLOB_CRATES` (which blobs a change feeds) all go through it, and name `examples/*/extension`.

## D4. Lints

An example's extension is rustfmt-checked and wasm-clippy-checked (the `guests` check, triggered by
`examples/*/extension/**`). It has no tests and no native use, so no native clippy or test pass.

## Rejected

- Vendoring the blob beside its project (`examples/marketing/marketing.wasm`): `CommittedBlob` would need a second
  layout, learned by about six call sites, for one blob.
- A product check (`extension build` writes a record, `check` compares it): a new record format and diagnostic, and the
  same path dependence. A separate plan if users ask for it.
- Leaving it to the README ("rebuild and copy it again"): that is what let it drift.
- Adding `examples` to the root's `exclude`: the `[workspace]` table would still be needed by anyone who copies the
  crate under another workspace.

## Consequences

- 14 committed blobs; `revendor` and `--verify` take about 25 s more.
- An SDK, macros or protocol-types change re-vendors the example too.
- `examples/marketing/marketing.wasm` is gone; the README says to run `revendor` after changing `lib.rs`.

## What would reopen it

- A second `examples/*/extension`: the stage name (the crate dir's last segment) collides, and
  `every_stage_name_is_unique` and `the_snapshots_are_the_sdk_blobs` say so.
- A user asking for drift detection of their own `.wasm` file entries: the product check above.

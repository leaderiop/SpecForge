# @rust/deps fixtures

cargo-cyclonedx was not installable on the host where these were written, so both fixtures are hand-written.

| Fixture | Source |
|---------|--------|
| `ledger.cdx.json` | hand-written in the shape of the CycloneDX reader's `cargo.cdx.json` (cargo-cyclonedx, specVersion `1.5`): the BOM of the workspace member `ledger` 0.1.0, root `pkg:cargo/ledger@0.1.0?download_url=file://.`, one component `rust_decimal` 1.36.0, which the root depends on |
| `packages.json` | hand-written served facts: the `package` facts the collectors receive, `pkg:cargo/demo` at the project root and `pkg:cargo/ledger` at `crates/ledger` |

The tests also read the CycloneDX reader's hand-written `../../code/cyclonedx/fixtures/cargo.cdx.json` (root
`pkg:cargo/demo`) and the OSV reader's recorded report `../../code/osv/fixtures/osv-scanner-2.7.1.json`.

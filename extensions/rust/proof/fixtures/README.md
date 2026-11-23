# @rust/proof fixtures

The Kani outputs are the Kani reader's own fixtures (`extensions/code/kani/fixtures`), hand-written to Kani's regular
output because Kani was not installable on the authoring host.

| Fixture | Source |
|---------|--------|
| `items.json` | hand-written served facts: the `item` facts `@rust/source` answers for the harnesses of the Kani reader's `one-crate.txt`, `failed.txt` and `workspace.txt`: crate `demo` (`src/lib.rs`: `add`, `#[cfg(kani)] mod proofs`, its harnesses `proofs::check_add` and `proofs::check_bound`), crate `ledger` (`crates/ledger/src/lib.rs`: `proofs::balance_never_negative`, `proofs::transfer_conserves`) and crate `vault` (`crates/vault/src/lib.rs`: `checks::seal_roundtrip`), and their `package` facts. A `#[kani::proof]` function is a plain item: no `test` attribute, no `test_id` |

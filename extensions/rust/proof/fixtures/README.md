# @rust/proof fixtures

The Kani outputs are the Kani reader's own fixtures (`extensions/code/kani/fixtures`), hand-written to Kani's regular
output because Kani was not installable on the authoring host.

| Fixture | Source |
|---------|--------|
| `items.json` | hand-written served facts: the `item` facts `@rust/source` answers for the crate `demo` (`src/lib.rs`: `add`, `#[cfg(kani)] mod proofs` and its harness `proofs::check_add`) and its root `package`. A `#[kani::proof]` function is a plain item: no `test` attribute, no `test_id` |

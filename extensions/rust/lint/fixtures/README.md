# @rust/lint fixtures

| Fixture | Source |
|---------|--------|
| `clippy-0.1.98.jsonl` | recorded with clippy 0.1.98 (cargo 1.98.1) by the Phase 11 research: `cargo clippy --workspace --all-targets --message-format=json -- -W clippy::unwrap_used` over the crate `demo` (one `src/lib.rs`: `Board`, `Board::len`, `first`, `tests::reads`), standard output only, absolute paths replaced by `/project` with `sed -E 's#/private/tmp/[^"<> ]*/research-tools/[^/"<> \#]*#/project#g' <recording> > clippy-0.1.98.jsonl`. Eight `compiler-message` lines, each of the four diagnostics twice (`--all-targets` checks the library as lib and as lib test), two `compiler-artifact`, one `build-finished` |
| `items.json` | hand-written served facts: the `item` facts of the recorded crate's `src/lib.rs` at their real extents (outer attributes included) and its root `package` |

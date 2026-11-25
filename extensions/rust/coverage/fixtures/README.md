# @rust/coverage fixtures

| Fixture | Source |
|---------|--------|
| `outside.info` | hand-written LCOV in `cargo llvm-cov --lcov` form: one section in the project (`/project/src/lib.rs`, the LCOV reader's `add`), one at `/elsewhere/a.rs` (a path dependency beside the workspace) and one at `../../etc/passwd` |

The llvm-cov tracefile the tests read is the LCOV reader's hand-written `extensions/code/lcov/fixtures/llvm-cov.info`:
cargo-llvm-cov was not installed on the authoring host. Replace it with a recording when one is made.

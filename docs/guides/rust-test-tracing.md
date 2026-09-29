# Tracing Rust tests to spec entities

This guide links a Rust project's tests to the entities they prove, so
`specforge analyze coverage` can tell which `verify` obligations are backed by
a passing test, which by a failing one, and which by nothing.

It takes three pieces:

- **`@specforge/testing`** gives kinds like `behavior` and `invariant` their
  `verify` obligations.
- **`specforge-test`** provides the `#[specforge_test]` attribute and records
  every annotated test's result.
- **`@specforge/cargo-test`** is the runner extension: `specforge collect`
  runs `cargo test` for it and maps the results onto the spec
  ([ADR 0002](../adr/0002-test-runner-extensions.md)).

## 1. Enable the extensions

In a project whose `Cargo.toml` sits at the root, `specforge init` enables all
three when you ask for `@specforge/software`:

```bash
specforge init --extensions @specforge/software
```

In an existing project:

```bash
specforge add @specforge/cargo-test   # also enables @specforge/testing
```

## 2. Declare the obligations

```spec
behavior create_user "Create User" {
  contract "..."
  verify unit "rejects a duplicate email"
  verify unit "stores a hashed password"
}
```

## 3. Annotate the tests

Add the crate as a dev-dependency. It isn't published to crates.io yet, so
take it from the SpecForge repository:

```toml
[dev-dependencies]
specforge-test = { git = "https://github.com/leaderiop/SpecForge" }
```

Then annotate each test with the entity it proves and, optionally, the
obligation:

```rust
use specforge_test::prelude::*;

#[specforge_test(behavior = "create_user", verify = "rejects a duplicate email")]
fn rejects_a_duplicate_email() {
    // ...
}
```

**Don't also write `#[test]`.** The attribute registers the test itself.

- A `#[test]` *below* `#[specforge_test]` is a compile error that points at
  the line to delete.
- A `#[test]` *above* it can't be seen by the macro (the compiler expands it
  first), so the test would be registered twice. The second run fails with
  "is registered twice: remove its #[test]".

The attribute takes any entity kind as its first argument (`behavior`,
`invariant`, `type`, `constraint`, ...) and the obligation's text as
`verify`. It works with the attributes you already use:

| Arrangement | What happens |
|---|---|
| `#[specforge_test(...)]` alone | Registered as a test. |
| two `#[specforge_test(...)]` on one function | One test, one recorded result per attribute: a test can prove several entities. |
| `#[specforge_test(...)]` above `#[tokio::test]`, `#[rstest]` or `#[test_case]` | That attribute registers the test; the macro only records it. |
| with `#[should_panic]` | A panic records `pass`; no panic records `fail`. |
| with `#[ignore]` | Recorded as `skipped` without running the body. Skipped tests prove nothing. |

## 4. Collect and analyze

```bash
specforge collect            # runs `cargo test --workspace --no-fail-fast`, records results
specforge analyze coverage   # reads specforge-report.json
```

The first `specforge collect` shows the command and asks you to approve it.
The approval is remembered per project and command, in
`~/.specforge/collector-consent.json` (never in the repository). Failing tests
don't make `collect` fail: they're recorded, and `analyze` reports them
(A014).

`analyze` then lists, per entity, the obligations no passing test names
(A015). A test proves an obligation only by naming its exact text in
`verify = "..."`; a test that names an obligation the entity doesn't declare,
usually a typo or a reworded statement, is A016.

- **Generated files:** `specforge-report.json` and `.specforge/` are
  generated. `specforge init` adds them to `.gitignore`.
- **CI:** there's no terminal to ask, so pass `--yes`, or run `cargo test`
  yourself and then `specforge collect --no-run` to record the report it wrote.
- **Custom target directory:** `collect` sets `SPECFORGE_REPORT`, and
  `specforge-test` writes its per-binary reports there, wherever
  `CARGO_TARGET_DIR` points.
- **Another command** (for example `cargo nextest run`): run it yourself, then
  `specforge collect --no-run`. The reports land in `target/specforge/`.

## Troubleshooting

- **`W115 ... reported tests for unknown entity 'x'`:** a test names an entity
  no spec declares, usually after a rename. Fix the annotation.
- **`A016 tests name obligation(s) ... does not declare`:** the test's
  `verify` text differs from the spec's statement. Make them match exactly.
- **`E059 ... needs your approval`:** you ran `collect` without a terminal.
  Use `--yes` or `--no-run`.
- **`E045 ... produced no report`:** the tests didn't build, or no test
  carries `#[specforge_test]`. The runner's output above the error says which.

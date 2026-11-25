# Recording a Rust project's packages, items and tests as facts

This guide makes a Rust project's manifests, source and nextest tests into
facts the spec can cite: each package becomes a `package` entity, each
function, type and module an `item`, and each test a `test` with its last
status. It takes four extensions:

- **`@code/core`** declares the fact kinds and checks every reference a spec
  writes to them ([ADR 0329](../adr/0329-code-core-holds-test-facts-and-their-ids.md),
  [ADR 0347](../adr/0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md)).
- **`@rust/cargo`** reads every `Cargo.toml` with `cargo_toml`, cargo's own
  manifest model: workspace inheritance and target autodiscovery as cargo
  applies them. It runs nothing.
- **`@rust/source`** walks each target's module tree by rustc's rules and has
  SpecForge's own parser read each file: no rust-analyzer, nothing to install.
- **`@rust/test`** runs `cargo-nextest` and reads its JUnit report.

The packs and the config file `@rust/test` has written are recorded in
[ADR 0348](../adr/0348-a-collector-declares-a-config-file-and-the-rust-packs.md).

## 1. Enable the extensions

```bash
specforge add @rust/test   # also enables @rust/source, @rust/cargo and @code/core
```

`@rust/test` needs nextest:

```bash
cargo install cargo-nextest --locked
```

Without it, `specforge collect` says "`cargo-nextest` was not found on PATH;
install it: `cargo install cargo-nextest --locked`" and records the packages
and items anyway.

## 2. Collect

```bash
specforge collect
```

A `Cargo.toml` at the project root selects the three packs; they run in fact
order, each served the facts of the one before:

1. `rust-cargo` reads the manifests into `package` facts.
2. `rust-source` reads each package's targets into `item` facts.
3. `rust-test` runs
   `cargo-nextest nextest run --workspace --no-tests=warn --tool-config-file specforge:<config> --profile specforge`
   and reads `.specforge/reports/rust-test.xml`.

- **The config file:** nextest writes JUnit only through a config, so before
  the run SpecForge writes `.specforge/reports/rust-test.nextest.toml`
  defining the profile `specforge` (every test runs, JUnit to the report
  path). It is layered under your own `.config/nextest.toml` as a tool config,
  so your settings still apply, and it stays after the run so you can rerun
  nextest with it.
- **Approval:** the first run asks you to approve the command and the config
  file's text, shown under it; a changed config asks again. In CI, pass
  `--yes`.
- **Failing tests:** they don't make `collect` fail. nextest exits non-zero,
  `collect` says so, and the failing tests are recorded as `failed`.
- **Your own run:** run nextest with that config yourself, then
  `specforge collect --runner rust-test --no-run`.
- **Fixture crates:** a `Cargo.toml` that is no package of the project (a
  test fixture, an example) is left out by `@rust/cargo`'s `exclude` setting
  in `specforge.json`. Each entry is a directory relative to the project root
  (one trailing `/` is trimmed); every manifest under it is skipped, so its
  targets yield no items and no test ids. An empty, absolute or `..` entry
  fails the collector with `exclude entry '<entry>' is not a project-relative directory`.

  ```json
  "settings": {
    "@rust/cargo": { "exclude": ["examples", "xtask/tests/fixtures"] }
  }
  ```

## 3. The ids

| Fact | Id | Example |
|---|---|---|
| `package` | `pkg:cargo/<name>` | `pkg:cargo/toy` |
| `item` | `rust:<crate>::<module path>::<name>` | `rust:toy::tests::adds`, `rust:toy/bin/toy::main`, `rust:toy::<Board as Display>::fmt` |
| `test` | `rust:<binary id>#<test path>` | `rust:toy#tests::adds`, `rust:toy::bin/toy#tests::adds`, `rust:toy::api#tests::adds` |

- A crate is its package's name for the lib, `<package>/<kind>/<target>` for
  another target. A file two targets reach has items in each crate.
- A test's binary id is nextest's: the lib's `<package>`, an integration
  test's `<package>::<target>`, any other target's `<package>::<kind>/<target>`.
  `tests::adds` in the lib, a bin and an integration test is three tests.
- Two workspaces of one project may share a package name: the package in the
  shallowest directory keeps `pkg:cargo/<name>`, every other is
  `pkg:cargo/<name>#<dir>`.

## 4. What an item knows about its test

A function under `#[test]`, an attribute ending in `::test`
(`#[tokio::test]`), `#[specforge_test]` or `#[spec]` carries `test_id`, the id
nextest gives it, so `rust-test` places each test where it is written. The
first `<kind> = "<id>"` of a `#[specforge_test(…)]` names the entity the test
proves:

```rust
#[specforge_test(behavior = "add_numbers")]
fn adds() { assert_eq!(add(1, 2), 3); }
```

nextest leaves an `#[ignore]`d test out of its report; `rust-test` records it
as `skipped` from its item, so a spec citing it is not `code/E101`. A test the
report lacks whose `cfg(...)` is false on the host (`windows` on Linux or
macOS, `unix` on Windows, `target_os = "windows"`, …; the host's family is
read from the project root's path) was compiled out, and is recorded
`skipped` the same way. On a unix host `target_os = "linux"` or `"macos"` is
not decided, so such a test the report lacks stays missing. Every `cfg(...)`
that compiles an item out is in its `attrs`: its own, those of the modules
around it (`#[cfg(windows)] mod tests { … }`), its file's `#![cfg(...)]` and
those on the `mod` declarations that reach its file. An attribute naming
`__guard_only` marks no test: that function carries no `test_id`
([ADR 0393](../adr/0393-x2-is-a-floor-every-real-test-reaches-and-only-tests-count.md)).

## Things to know

- `rust-test` places each test and fills the ignored ones from the `item`
  facts `rust-source` stored. A plain `specforge collect` runs the three in
  order; `--runner rust-test` alone uses the items the last collect stored, so
  before any, its tests have no place and no ignored test is filled.
- `@specforge/cargo-test` also detects `Cargo.toml`: a project enabling both it
  and `@rust/test` runs its suite twice.
- A `.config/nextest.toml` of your own that defines a `specforge` profile with
  its own JUnit path wins: no report reaches SpecForge, and the run is skipped
  (`E045` when you name `--runner rust-test`).
- A suite longer than the collector deadline (600 s) is stopped.
- An attribute or doc comment belongs to an item only in the unbroken run of
  attribute and doc lines directly above it: a `//` comment or a blank line
  between `#[test]` and its `fn` ends the run, so that function is no test.
- A `#[path]` that loops back to a file already in the target's module tree
  keeps its `mod` item, and the file is not read a second time.

## Troubleshooting

| What you see | Why, and the fix |
|---|---|
| `` `cargo-nextest` was not found on PATH `` | Install nextest: `cargo install cargo-nextest --locked`. |
| `E045` rust-test produced no report at … | The build failed before nextest ran, or your own config's `specforge` profile wrote the report elsewhere. Run the command above yourself to see nextest's output. |
| `<path>: the host did not serve it (<why>)` | The host withheld a `Cargo.toml` (too large, not UTF-8, unreadable). `rust-cargo` reads every manifest or none. |
| `the read '…' stopped early` | The host's walk of the project reached its cap of 20,000 entries, or the project holds over 2,000 manifests or target files of one kind. |

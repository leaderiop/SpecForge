# A collector may declare a config file, and the Rust packs read cargo, source and nextest

**Status:** accepted (2026-11-09, R2 of the Beyond Parity build plan). **Amends**
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D2.1 (a `process` capability may carry one
config file, and its argv the `{config}` placeholder). Builds on
[ADR 0345](0345-the-syntax-import-and-collectors-that-read-answer-and-are-served-facts.md) (the syntax import, reading
collectors, `answers`, the `facts` input), [ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md)
(`Fact.at`, `Fact::name`) and [ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md) (the ids and
their minting). Protocol 2.0.0, unreleased: `ProcessScope.config` and `CollectorDescriptor.install` go into the 2.0.0
row, no bump. No code: a collector reports only its error or its skip, and `@code/core` judges the facts.

## Context

- **nextest writes JUnit only through a config file.** Its report path is `[profile.<name>.junit] path`, absolute, in
  a config; no flag or environment variable sets it. `--config-file` replaces the project's own
  `.config/nextest.toml`; `--tool-config-file <tool>:<abs path>` layers a tool's config under it. A guest has no file
  system (ADR 0293), and the host cannot know `CARGO_TARGET_DIR`, so the host must write the file.
- **nextest omits ignored tests from its report.** The summary says "1 skipped"; the JUnit holds no `<skipped>` for an
  `#[ignore]`d test. Running with `--run-ignored all` would run what the project chose not to run.
- **One path, three binaries.** `tests::adds` in the lib, a bin and an integration test are three tests; nextest names
  each by its binary id (`toy`, `toy::bin/toy`, `toy::api`), which is the JUnit `<testsuite>` name.
- **cargo's own reader is a library.** `cargo_toml` reads a `Cargo.toml` with workspace inheritance
  (`version.workspace = true`, `[workspace.dependencies]`) and target autodiscovery (`src/bin/*.rs`, `tests/*.rs`, …)
  over an abstract file system: a file listing is enough. It costs about 467 KB in a blob; `cargo metadata` needs cargo
  installed, an approval, reads the root workspace only and may touch the network.
- **rustc's module rules are a pack's knowledge.** `mod b;` is `<dir>/b.rs` or `<dir>/b/mod.rs`; a non-mod-rs file
  `src/a.rs` keeps its children in `src/a/`; `#[path]` is relative to the declaring file's directory outside an
  inline module and to the module's directory inside one; `#[cfg_attr(…, path = …)]` names a cfg twin.

## D1. A process capability may declare one config file

`ProcessScope.config: Option<ProcessConfig { name, template }>`, wire key `config` on a `process` capability. It is
sound (else W021 at load, and the capability grants nothing) when:

- `name` is one path segment `[a-z0-9][a-z0-9._-]*` ending `.toml` or `.json`;
- `template` is non-empty UTF-8 of at most 16 KiB;
- the capability's `call_kinds` are exactly `[collect]`;
- its argv names `{config}` in exactly one element, and no argv element holds two placeholders.

An argv naming `{config}` without a config is W021 too. The SDK's `CollectorBuilder::config(name, template)` puts the
config on the capability `.run(argv)` declares (`Capability::config` for a capability declared by hand).

## D2. The host writes it fresh before the run

Under a run (`Mode::Run`), after the stale report is removed and before the command starts, the host writes the
template, each `{report}` replaced by the report's absolute path, to `.specforge/reports/<collector>.<name>`, held by
the collect's containment (a link that leaves the project is `outside_root`, as for the report), created fresh. The
expansion is escaped for the inside of a double-quoted TOML or JSON string: `\` → `\\`, `"` → `\"`, a control
character → `\uXXXX`, which both formats read. `--no-run` and `--report` write nothing. The file stays after the run, so
a person can rerun the tool with it. `{config}` in the argv expands to the file's absolute path; `argv_matches` treats
`{report}` and `{config}` alike: one placeholder per element, the literal text before and after it matching.

## D3. The config is part of the approved scope

The config lives on `ProcessScope`, so an approval stores it and `covers` (equality) asks again when the template
changes: a config can change what runs (test groups, scripts, wrappers). `ProcessScope::summary` appends
`, writes <name> (sha256:<first 12 hex>)`. The terminal's question shows the file's name and text, each line printable
(ADR 0244), under the command (`Question::Capability.config`).

## D4. A collector may declare an install line

`CollectorDescriptor.install: Option<String>` (one line, at most 200 bytes, printable; else W021 on the collector):
said when its program cannot be found. `ProcessClass::NotFound` becomes `RunFailure::NotFound`; the E045 of a named
collector and the skip reason of a detected one say "`<program>` was not found on PATH" and, when the collector
declares one, "install it: `<line>`". A changed install line asks nothing: it is no effect.

## D5. `@rust/cargo` reads every `Cargo.toml` with `cargo_toml`

`@rust/cargo` 1.0.0 (collector `rust-cargo`, answers `package`, peer `@code/core` `^1.1`): detected by `Cargo.toml`, no
command, no report, no capability. Its `files` inputs read `**/Cargo.toml` with text and the target globs cargo's
autodiscovery reads, without text. The extension library `extensions/rust/manifest` (`specforge-rust-manifest`)
completes each manifest with `cargo_toml` over the listing, inheritance and autodiscovery included, and answers
`specforge_code_facts::Package`s: one `package` fact per package, `pkg:cargo/<name>`, located at its `[package]` table.
A virtual manifest is no package. `depends_on` names the project's packages its path dependencies (normal, dev,
build) reach. Two workspaces of one project may share a package name: the package whose directory is shallowest (fewest
segments, then lexically first) keeps `pkg:cargo/<name>`, every other takes the purl subpath `pkg:cargo/<name>#<dir>`. Every manifest
the walk sees is read, fixtures and nested workspaces included (the walk skips `target`, `vendor`, `node_modules`). A
manifest the host withheld or `cargo_toml` cannot read, and a read that says `truncated`, are the collector's error,
never a dropped package.

## D6. `@rust/source` mints items from each target's module tree

`@rust/source` 1.0.0 (collector `rust-source`, answers `item`, input `facts [package]`, peers `@code/core` `^1.1` and
`@rust/cargo` `^1.0`): detected by `Cargo.toml`, no command; an fs read capability on the `project` root backs its
syntax import. Per target of every served package it walks the **module tree** breadth-first from the target's root by
rustc's rules (Context), following every `path` a `cfg_attr` names; each round's new files are parsed in one
`syntax.parse` with the pack's query (`extensions/rust/source/queries/rust-tags.scm`), the files the answer's cap left
unparsed asked again. A file outside the project, or absent, keeps its `mod` item and no items: rustc owns that error,
and a cfg-gated module's file may be absent by design.

Every definition at module level, in inline modules and in `impl` and `trait` blocks is an `item` (not inside a
function body, not variants or fields), minted by ADR 0347's `item_facts`: `ItemKind::Method` for a function in an
`impl` or `trait`, `Visibility::rust` of the captured modifier, `at` the definition's span. An attribute or doc comment
belongs to the item when it is in the contiguous run of attribute and doc lines directly above it. A file two targets
reach mints items in each crate.

## D7. A test function knows its runner id and what it names

A function marked by an attribute whose path is `test`, ends in `::test`, or is `specforge_test` or `spec` carries
`attrs` `test` and `test_id` = `ItemId::test_id(edition)`: the id nextest gives it. `#[ignore]` adds `ignore`, each
`#[cfg(…)]` its `cfg(…)`. The first `<kind> = "<id>"` of a `specforge_test`, `spec` or
`{specforge_test_macros,specforge_test,specforge}::test` attribute (the names `check-links` reads, never another
`…::test` such as `tokio::test`) is written to `names` with `Fact::name`, at the id's string. An alias by `use … as`
is not recognised.

## D8. `@rust/test` runs nextest and fills the ignored tests

`@rust/test` 1.0.0 (collector `rust-test`, answers `test` and `test_run`, input `facts [item]`, peers `@code/core`
`^1.1` and `@rust/source` `^1.0`): detected by `Cargo.toml`, it runs
`cargo-nextest nextest run --workspace --no-tests=warn --tool-config-file specforge:{config} --profile specforge`
with the config `nextest.toml`:

```toml
[profile.specforge]
fail-fast = false

[profile.specforge.junit]
path = "{report}"
store-success-output = false
store-failure-output = false
```

and reads `.specforge/reports/rust-test.xml` with the JUnit library. The program is `cargo-nextest`, not `cargo`: a
missing nextest is then the process port's `NotFound`, and the install line says
`cargo install cargo-nextest --locked`; through `cargo` it would be cargo's "no such command", which reads as a failed
build. Each case sits in exactly one `<testsuite>`, its name the binary id, else the report is not nextest's (the
collector's error). A case is `TestId::rust(<binary id>, <name>)`; one whose id an item's `test_id` holds takes the
item's `at`, `file` and `names`. Every item whose `attrs` hold `test` and `ignore` and whose `test_id` the report lacks
becomes a `skipped` test without a run: a cited ignored test is then held, not `code/E101`.

## D9. No codes

A collector has no finding channel (`CollectOutput` holds no diagnostics); the facts are judged by `@code/core` (E101,
W102, W401, I402). A pack that only collects catalogues none (ADR 0328 D2).

## Rejected

- **`cargo metadata` instead of `cargo_toml`.** It needs cargo, an approval and the network, and reads one workspace.
- **The config on the collector.** A changed template would run under an old approval.
- **nextest's `--config-file`.** It replaces the project's own config.
- **Writing into the target directory.** The host cannot know `CARGO_TARGET_DIR`.
- **A guest-written file.** A guest has no file system (ADR 0293).
- **`cargo nextest` as the program.** A missing nextest would read as a build failure.
- **`--run-ignored all`.** It runs tests the project chose not to run.
- **An error for a shared package name.** A fixture would block every collect, with no exclusion to offer.
- **The subpath on every package id.** ADR 0347 D5's id is the plain purl.

## Consequences

- A collector's command may read a file the host writes for it, and the person approves its text.
- `@specforge/cargo-test` and `@rust/test` both detect `Cargo.toml`: a project loading both runs its suite twice.
- A project's `.config/nextest.toml` that defines a `specforge` profile with its own JUnit path wins: no report reaches
  ours, and the run is skipped, or E045 when named.
- The 600 s collector deadline (`DEADLINE_CEILING`) stops a longer suite.

## What would reopen it

- A tool that reads a config format other than TOML or JSON: the escape of D2 is a string's.
- A project whose manifests the `files` read caps (2,000 files): a manifest listing of its own.
- An attribute alias projects use widely enough that names go missing.

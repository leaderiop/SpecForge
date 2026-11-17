# The root spec runs on `@gsd/core`, and its tests name requirements

**Status:** accepted (2026-11-17). **Amends**
[ADR 0090](0090-text-a-user-types-is-tested-by-properties-over-the-corpus.md) (the corpus lists the frozen pre-swap spec
beside the live roots: D8), [ADR 0091](0091-the-crate-map-is-generated-from-the-workspace.md) D4 (the spec no longer
models crates, and `check-modules` retires: D7),
[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) D14 to D17 and
[ADR 0367](0367-the-self-spec-sample-retires-at-the-swap-and-absorb-stays-while-the-old-builtins-ship.md) D1 (carried
out: D7), and [ADR 0372](0372-the-converter-gives-every-old-kind-one-fate-and-writes-gsds-ids.md) D8 (the census's root
half retires once it has held on the pre-swap spec: D2, D7). **Carries out**
[ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md) (the type and event drift
tests: D7), [ADR 0365](0365-a-module-is-dropped-and-no-kind-is-added-to-hold-it.md) D1 (D7) and ADR 0367 D2 and D3 (D9).
Builds on [ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) (the kinds the spec is written in),
[ADR 0371](0371-links-come-from-gsds-fields-and-collected-data.md) (a requirement's computed `proof`), ADR 0372 (the
converter and its id map) and [ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md)
(the vendored blob the root now loads). No protocol, SDK, host-rule or attribute-grammar change comes with this
decision.

## Context

**What the swap moves.** This repository's own spec, `spec/` and `specforge.json`, was written in the old
`@specforge/*` builtins' kinds, and its tests linked those kinds' entities: `#[specforge_test(behavior = "...", verify =
"...")]`. The swap converts the spec to `@gsd/core`'s kinds with `cargo xtask absorb`, rewrites each test's link to name
a requirement, and re-points the two gate steps that read the links: `check-links`, which checks each link names an
entity, and `dogfood`, which judges what the tests prove. It is one commit, because the four agree only together: the
rewritten attributes name requirements only the converted spec holds, `check-links` judges them against that spec, and
`dogfood` reads the proofs the converted spec computes from them. Any three without the fourth leave a red gate.

**The research's figures.** These were measured on scratch copies of the tree at `925ee143a`, before the decisions; the
swap commit's own figures, as built, are added to this record when it lands, and where the two differ the as-built
figure is the record.

- **The conversion.** `absorb` over a `git archive` of the tree, with the root's three Rust packs as `--extension`,
  exits 0 in 9.07 s and converts 469 entities: 353 requirements, 100 decisions, 3 milestones and 13 phases. It drops
  1,821 entities whole, and its ledger holds 20,130 entries with 469 ids. It writes 73 `.spec` files, 72 at their old
  path plus the scaffold's `hello.spec`, of the old 200; `spec/` also tracks 65 files that are not `.spec` (counted
  with `git ls-files`). Collecting facts in the source first changes no byte of the output (compared with `cmp` and
  `diff -r`).
- **The check.** The converted spec checks with 7 `gsd/W103` warnings and nothing else under `gsd-compat`; the same 7
  are errors under `gsd-strict` and `gsd-plus`. They are 7 phases with 7 to 11 success criteria, where gsd/W103 allows
  two to five. Merging each phase's criteria into five groups of neighbours clears all three profiles (run on a
  git-backed scratch tree, the Rust packs' facts collected).
- **The attributes.** A Rust-aware scanner (it skips strings, raw strings, comments and char literals) counts 6,201
  spec-link attributes in 930 files. Reached through the old graph and the converter's id map, 6,077 are rewritten
  into 6,469 attribute lines, 101 reach no requirement and are removed (95 of them registered their test alone, so
  `#[test]` goes back), and 23 belong to other projects and are kept. 442 reach more than one requirement; after the
  rewrite, a function names at most five.
- **What a naive swap breaks.** The whole workspace on a swapped, rewritten scratch tree: 9,663 tests, 175 failed. By
  cause: 135 spec-types and 2 spec-events drift tests (they compare code with `spec/types` and `spec/events`); 28
  corpus tests (14 through the parser's corpus roots with a `>= 200` file threshold, 14 reading `spec/` with their
  own); 4 census tests (the root half); 4 that a rewrite of a fixture crate's attribute broke; 2 that read a spec file
  the conversion drops. One guest failed to compile on an `include_str!` of a dropped file.
- **The proofs.** On that tree, `@rust/source` collected 5,948 items with names, and `@rust/test` over nextest's JUnit
  report gave 147 requirements `proven`, 15 `failed` and 191 `no_test` (with 161 tests failing for the causes above);
  14 of the `no_test` requirements are named only by guests' tests, which run under `cargo test` and write no JUnit.
- **The crate map.** `crate-map` renders `ARCHITECTURE.md`'s stack and `crates/README.md`'s table from `cargo metadata`,
  each member's folder and its manifest `description`, and never read the `module` kind. The 77 collected `package`
  facts carry no `description`.
- **One report.** `@rust/test` reads one report file: a directory is E045 and a second `--report` is E028 (both quoted
  from the run). CI runs the tests in four partitions, each writing its own report.

**Measured by this decision's tracer.** On a small gsd project (`xtask/tests/fixtures/requirement-links`), a test with
two stacked `requirement = "..."` attributes makes both requirements `proven` through `@rust/source`, nextest's JUnit
and `@rust/test`; the report it collects is two partitions merged as D5 says, so `@rust/test` accepts the merged
shape. nextest writes the `gate` profile's report at `<workspace root>/target/nextest/gate/junit.xml`.

## D1. The root's project file

`specforge.json` loads `@specforge/checks` (`@gsd/core`'s required peer), `@code/core`, `@gsd/core`, `@rust/cargo`,
`@rust/source` and `@rust/test`: absorb's target list plus the three Rust packs. `@rust/test` replaces
`@specforge/cargo-test` as the runner whose report becomes `test` facts. It loads none of `@specforge/product`,
`@specforge/software`, `@specforge/testing`, `@specforge/cargo-test`, `@specforge/governance` or `@specforge/formal`.
It keeps `"$schema": "./schema/specforge.schema.json"` and sets no profile, so the root runs under `gsd-compat`.

`spec/specforge.spec` holds the one `spec` block absorb's scaffold writes. A `spec` block configures nothing, so "the
spec block loads `@gsd/core`" is read as the project file's list. The spec checks with no error and no warning under
`gsd-compat`, `gsd-strict` and `gsd-plus`: each converted phase with more than five success criteria has them merged
in order into five, neighbours joined, in the old milestone's own words, since gsd/W103 allows two to five.

The reason: the list is what absorb writes for a gsd target, and the Rust packs are what turns this repository's code
and tests into facts. A spec that passes the strictest profile cannot regress when the profile is raised.

## D2. The conversion and its record

`cargo xtask absorb` converts the pre-swap tree, tagged, into a scratch target outside the tracked tree. Only
`spec/**/*.spec` and `specforge.json` come back: the scaffold's `hello.spec` as `spec/specforge.spec`, each old `.spec`
file absorb wrote nothing for deleted, and every tracked file under `spec/` that is not a `.spec` file kept. The ledger,
with its id map and every `dropped` entry, is kept with the round's records outside the tracked tree. The census held
on the pre-swap spec, measured before its root half retired.

The reason: ADR 0367 D4 makes the whole-spec run the swap's evidence and keeps conversion artifacts out of the
repository; copying back `.spec` files alone keeps the research notes and READMEs `spec/` tracks.

## D3. A test names a requirement

Each spec-link attribute of this repository's own tests becomes `requirement = "<id>"`, one attribute per requirement,
stacked when a test proves several. The requirements are reached through the old graph and absorb's id map: a behavior
through its features, an invariant or a constraint directly. A type, event, port or failure-mode attribute reaches
none and is removed, with `#[test]` written back where no other attribute registers the test. A duplicate on one
function is removed, since the runtime guard refuses one. No `verify` text remains. Other projects' tests (the example
projects, the crates under `tests/fixtures/`, the macro's self-tests) keep theirs. The macro and its grammar are
unchanged.

The reason: the macro takes one `<kind> = "<id>"` pair per attribute, then only `verify`, and its guard keys on the
entity id, so several requirements are several attributes and the same one twice is a panic. Reaching a behavior's
invariants as well would stack three or more lines on 3,596 attributes.

## D4. `check-links` reads requirements

Each spec-link attribute under `crates/`, `extensions/` and `integrations/rust/` (not the macro's self-tests, not a
crate under a `tests/fixtures/` directory) names, with the kind word `requirement`, a requirement of the root spec, and
carries no `verify` text. The findings are `UNKNOWN` (no entity has the id), `KIND` (the entity is of another kind) and
`VERIFY` (a `verify` text); `check-links` exits 1 on any. A016 and its nearest-text hint leave with the `verify` texts.
The gate's `links` check runs it on a `.spec` change, a Rust file under `tests/`, or a Rust file holding a spec-link
attribute.

The reason: the kind word was ignored, so a link to a decision or a phase passed, and A016 read the `verify` texts the
swap removes. A crate under `tests/fixtures/` is another project's, its links judged against its own spec.

## D5. `dogfood` reads `proof`

nextest writes a JUnit report under its `gate` and `ci` profiles, at
`<workspace root>/target/nextest/<profile>/junit.xml`. The step merges every `junit*.xml` there, at any depth (CI
downloads each partition's into its own directory), into one report: every `<testsuite>` of each, in order, under one
`<testsuites>` root, since `@rust/test` reads one report. It then runs `specforge collect --runner rust-cargo`,
`--runner rust-source` and `--runner rust-test --report <merged>`, each `--yes --effects online --format json`, then
`specforge export --format graph --kinds requirement` and `specforge anchors --file extensions --format json`.

It fails when no requirement is `proven` or any is `failed`, and prints `requirements proven: N of M` with the count of
each other state and how many `no_test` requirements only guests' tests name. Guests run under `cargo test` and write no
JUnit; running them under nextest is a later change. The report directory the old step read and its variable go.

The reason: a requirement's `proof` is `@gsd/core`'s own answer from the facts that name it (ADR 0371); the gate only
reads it, so the gate and a user's `specforge` cannot disagree on what is proven.

## D6. `specs` collects before it checks

A project whose project file loads `@rust/cargo` and `@rust/source` is collected by both in the check's copied tree
before `specforge check`, so a fact reference is judged as a user's would be. The root's baseline entry falls away:
W102 is 0, and no warning is left.

The reason: the converted spec cites no fact today, so the collect changes no count; it keeps a fact reference written
later from being judged against an empty store.

## D7. What retires, and what stays

`self-spec-sample` (its check, its CI step, its goldens and the checkout-copy variable only it set) and the census's
root half retire (ADR 0367 D1, ADR 0372); the census's fixture half stays. `spec-modules` and `check-modules` retire:
they read only `module`, which ADR 0365 D1 drops whole, and the `package` facts that stand for crates are collected
from the same manifests, so they cannot drift. The spec-types and spec-events drift tests retire, since types and
events have no home (ADR 0364). `crate-map` stays on `cargo metadata`: it never read `module`, so ADR 0365's "crate-map
loses its input" was moot, and package facts carry no `description`, which the map prints.

The reason: each retired reader has no input after the swap, and the one that keeps its input keeps its source.

## D8. The old spec is a frozen corpus

`fixtures/old-spec/` holds the pre-swap spec's 200 `.spec` files under `spec/` and its project file, never edited. The
parser, formatter, project, LSP, tree-sitter, ops and component tests that need the old kinds' syntax read it, and
`specforge_parser::testing::CORPUS_ROOTS` lists it beside the live roots. This amends ADR 0090's corpus. The gate
checks it as no project.

The reason: the parser's, formatter's and LSP's property tests need the old kinds' syntax (contracts, `ensures`
blocks, `verify` lines, methods, annotations), which the live spec no longer holds; lowering their thresholds would
keep the tests and lose what they read.

## D9. The old builtins and `cargo xtask absorb` stay

They stay (ADR 0367 D2, D3), because tracked projects still load the old builtins. At this decision,
`git ls-files '*specforge.json'` lists these that name one:

- the example projects `examples/product-launch`, `examples/regulated-process`, `examples/shop`,
  `examples/spec-driven-method` and `examples/todo-app`, and `integrations/rust/spec`;
- fifteen fixtures of the CLI's tests under `crates/surfaces/cli/tests/fixtures/` (`coverage/fx1`, four under
  `navigation/`, nine under `parity/`, `read_views/rv1`);
- three projects under `xtask/tests/fixtures/`: absorb's `absorb/old` and `absorb-sample`, and `links`, the old-builtin
  fixture of the link check's test;
- the editor extension's `integrations/vscode/test/fixtures/sample-project`;
- after the frozen corpus lands, `fixtures/old-spec`.

`integrations/vscode/snippets/specforge.json` names them too, as snippet text, and is no project.

The reason: deleting a builtin a tracked project loads breaks that project, and absorb is the way off the old builtins
for each of them.

## Rejected

- **Re-pointing `crate-map` at `package` facts.** They carry no description, which the map prints.
- **Lowering the corpus thresholds instead of freezing.** The old blocks leave the corpus, and the tests keep their
  names while reading less.
- **Freezing the drift tests' spec files as goldens.** A golden no live spec follows checks nothing.
- **Reaching a behavior's invariants too.** 3,596 attributes would stack three or more lines.
- **A tracked Rust tool for the attribute rewrite.** The rewrite ships nothing and runs once.
- **Hand-curating the seven phases' success criteria.** The merge keeps the old milestones' own words; a reviewer can
  reword them later.
- **Running the guests under nextest now.** A change of its own; their requirements read `no_test` and are counted.
- **Keeping A016 for other projects.** `check-links` checks this repository's tests against its own spec only.

## Consequences

- The swap commit touches about 925 Rust files and the spec.
- A requirement is proven only by a test nextest runs; a guest's test leaves its requirement `no_test`.
- The frozen corpus adds about 2.6 MB of tracked text.
- CI's test partitions upload their JUnit reports for the `dogfood` job.
- The old builtins and absorb remain for the projects D9 lists.

## Would reopen it

- The guests running under nextest.
- The last project on the old builtins leaving: then the old builtins, absorb and the frozen corpus can go.
- A `@gsd/core` kind for a test's link.

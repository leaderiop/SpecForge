# X2 is a floor every real test reaches, and only tests count

**Status:** accepted (2026-11-25, round r6-breadth, Phase 14). **Amends**
[ADR 0388](0388-x2-is-the-share-of-test-ids-a-standard-report-saw.md) D2 (X2 is no longer print only: D8), D3 (the
projects and how the fixtures collect: D8, D9) and its Consequences (taskboard and vault read 100%: D4), and
[ADR 0374](0374-the-root-spec-runs-on-gsd-core-and-its-tests-name-requirements.md) D5 (guests and the `@gsd` workspace
write JUnit the step merges, and the guests-only count retires: D1 to D3). **Keeps**
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) D2 (one lockfile per blob, so
one test run per guest manifest) and
[ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D3 (a blob that grows
re-measures the size budget in the same commit).

## Context

- ADR 0388 defined X2 (the share of items carrying a `test_id` whose id a standard test report saw) and printed it,
  leaving the floor to this phase. The last full gate before it (2026-11-25, `dogfood.log`) read
  `X2 SpecForge: 9772/13660 (71.5%)`, `X2 taskboard: 0/0 (no item has a test_id)` and `X2 ledger: 6/6 (100.0%)`.
- SpecForge's missing ids fall in five groups: the extension guests' tests (935), which run under `cargo test` and
  write no JUnit; the `@gsd` workspace's tests (2,923), whose nextest run wrote no JUnit either; tests of fixture and
  example crates (22), which are other projects' and never run as SpecForge's; `#[cfg(windows)]` tests (3), compiled
  out on the unix hosts that run the gate; and `__guard_only` helpers (5), which carry a test attribute but are not
  tests. Moving the guests under nextest alone reaches about 78%.
- Taskboard and vault are JavaScript projects. `@javascript/source` mints no `test_id`, so their X2 is `0/0` whatever
  their reports hold (ADR 0388, research finding F3).
- The owner settled the phase on 2026-11-25: the A/B fixtures taskboard, ledger and vault are X2's fixtures beside
  SpecForge; JavaScript test calls carry the id node-test mints; SpecForge reaches 100% by reporting every real test
  and counting only tests; X2 becomes a hard floor; the fixtures collect with the Phase 11 packs from recorded reports.

## D1. Guests run under nextest, one run per guest manifest

The `guests` check runs `cargo nextest run --no-fail-fast --no-tests=pass --profile <p> --config-file <file>
--manifest-path <guest>` once per guest manifest (`extensions/*/Cargo.toml` and `extensions/*/*/Cargo.toml`), with
`CARGO_TARGET_DIR` the shared guest target as before; `<p>` is `ci` under `cargo gate ci`, else `gate`. There is no
shared `extensions/` workspace: each blob keeps its own lockfile (ADR 0366 D2).

The config file is generated per guest under `<target>/gate/nextest-guests/<slug>.toml`: a `[store]` whose `dir` is
the repository root's `target/nextest`, written as a TOML literal string (a Windows path's backslashes need no escape;
a path holding `'` or a line break is refused), then the committed `.config/nextest-guests.toml` (the `default`, `ci`
and `gate` profile bodies), then `[profile.<p>.junit]` with `path = "junit-guests/<slug>/junit.xml"`. The slug is the
manifest's directory under `extensions/`, each `/` a `-` (`extensions/rust/test` is `rust-test`), and only ASCII
lowercase letters, digits and `-` are accepted. So each run writes
`target/nextest/<p>/junit-guests/<slug>/junit.xml`. The check empties `target/nextest/<p>/junit-guests` before its
loop, so a report from an earlier run is never read as current. `--no-tests=pass` keeps a future guest with no test
from failing the check (nextest's default for no tests is to fail).

The root `.config/nextest.toml` cannot be passed: for a manifest outside the root workspace nextest finds no config
of its own, and the root file's package overrides (`package(specforge-ops)`, `binary_id(xtask::absorb_e2e)`) match
nothing in a guest, which nextest refuses as a parse error. Without a `[store]` the report would land in the guest's
own `target/`, where the gate never looks.

The reason: nextest writes the JUnit report X2 and the requirement verdict read, and one run per manifest is what
the guests already were under `cargo test`.

## D2. The `@gsd` workspace writes its report beside them, and CI keeps both

`gsd/.config/nextest.toml` sets `[store] dir = "../target/nextest"` (relative to `gsd/`, so the repository root's
`target/nextest`) and `[profile.gate.junit]` and `[profile.ci.junit]` with `path = "junit-gsd/junit.xml"`; the `gsd`
check passes `--profile <p>` and empties `target/nextest/<p>/junit-gsd` first. The custom profiles inherit `default`
and its override, so the suite runs as before; a plain `cargo nextest run` in `gsd/` now stores under the root's
`target/nextest/default`.

CI keeps `target/nextest/ci/junit-guests` from the specs shard (which runs `guests`) and
`target/nextest/ci/junit-gsd` from the gsd shard, as artifacts named `junit-guests` and `junit-gsd`, each failing
when no file is there. Each artifact is named as its directory, so a download lands where a local run writes; the
dogfood job downloads every `junit-*` artifact.

The reason: the gate reads every `junit*.xml` under the report directory already (ADR 0374 D5); naming the
directories as the artifacts keeps one reader for a local run and CI.

## D3. `dogfood` runs after the three test checks, and the guests-only exception retires

`dogfood` runs after `tests`, `guests` and `gsd`, and refuses, naming those checks, when any of the three report sets
is absent (a missing set would otherwise read as thousands of missing ids and hide the cause). The requirement
verdict no longer reads `specforge anchors --file extensions`, and the "named only by guests' tests" count goes: the
guests' tests are in the merged report like every other test.

The reason: ADR 0374 D5 carved that exception out because the guests wrote no JUnit; once they do, it only hides a
requirement a guest test fails to prove.

## D4. A JavaScript test call is an item, and a leaf test carries node-test's id

`@javascript/source` reads test files for their test calls only. A call whose callee is `test`, `it`, `describe` or
`suite`, or one of those with `.skip`, `.todo` or `.only`, or a subtest `<name>.test(...)` inside a test, whose first
argument is a string or a template with no substitution and not empty, becomes a `function` item with the attribute
`test`; its path is the enclosing suite and test names, then its own. A plain call needs a function argument.

node's JUnit reporter writes a test or a suite as a `<testsuite>` exactly when at least one test, suite or subtest
ran inside it, and as a `<testcase>` otherwise (checked against node v26.13.0): an empty `describe`, a `describe`
with no function and any `.skip` call are test cases. So a call carries `test_id = TestId::js(package, file, path)`,
the id node-test mints, when it is called with `.skip`, or when no test, suite or subtest call sits inside it, minted
or not: a subtest with a dynamic name, a function reference or only options still makes its parent a `<testsuite>`.
node never runs a skipped call's function, so nothing inside a `.skip` call or a call with a `skip` option is an
item; a call with a `skip` option that holds calls carries no id either, since whether it is skipped is decided when
it runs. A `.todo` call runs its function, so what it holds is read. A dynamic name (an identifier, a concatenation, a
substitution, `.each`, `.concurrent`) or an empty one (node names it `<anonymous>` or after its function) mints
nothing and raises nothing. Shortcut: a subtest a helper registers out of the call's span (`await check(t)`) is not
seen, so that test keeps an id node reports as a suite; X2 names it missing, and the test is rewritten to call
`t.test` in its own body. Helpers in test files stay no item. No new item kind is added.

The reason: the definition stays ADR 0388 D1's, so taskboard and vault read their number from the same computation
as every other project; a new kind would re-vendor every code-facts linker and `@code/core`.

## D5. A test compiled out on this platform is reported skipped

`@rust/test` reports a test item that the report lacks and whose `cfg(...)` attribute is false on a unix host as
`skipped`, as it already does for `#[ignore]`. The predicate is three-valued: `windows`, `unix`, `target_family` and
`target_os = "windows"` are decided; `feature`, `test` and anything else are unknown, so not excluded. Shortcut: it
assumes a unix host, the only kind that runs the gate and the dogfood job.

The reason: a `#[cfg(windows)]` test is a real test that cannot run here; reading it as missing would make the floor
unreachable, and an unknown predicate must never hide a test a forgotten feature compiled out.

## D6. `__guard_only` marks no test

An attribute whose arguments name the identifier `__guard_only` marks no test, so the helper it decorates gets no
`test_id`.

The reason: those helpers exist to type-check a guard, never run, and are not tests.

## D7. `@rust/cargo` excludes directories by a setting

`@rust/cargo` declares the setting `exclude`, a list of project-relative directories: a manifest in or under one is
no package of the project. SpecForge's root `specforge.json` lists its fixture and example directories.

The reason: every consumer computes the same number from the same facts (ADR 0388 D1), so the census leaves those
crates out where the facts are made, not in a gate-side filter.

## D8. X2 is a floor

`dogfood` fails when X2 is below 100%, or `0/0`, on SpecForge, taskboard, ledger or vault. It logs every project's
line first, then fails naming at most 10 missing ids and the count of the rest. The negative control is `x2::floor`
unit tests on literals plus one end-to-end gate test over a ledger copy whose report lacks one case; that test is
`#[ignore]`d in the workspace run, because it needs the built `specforge` (`SPECFORGE_BIN`, which the root workspace's
test partitions do not build), and the dogfood check, which sets it, runs it first. Ignored, `@rust/test` reports it
skipped, so X2 still counts it. This closes ADR 0388 D2's "print only until Phase 14".

The reason: a ratio nothing enforces drifts; a floor with a control that fails end to end proves the floor can fail.

## D9. The fixtures collect with their packs from recorded reports

Taskboard, ledger and vault collect with the packs each detects: taskboard and vault with `@javascript/coverage`;
ledger with `@rust/coverage`, `@rust/lint`, `@rust/deps` (sbom and osv) and `@rust/api`, not `@rust/proof` (it has no
Kani harness). Their reports are recorded once on linux/amd64 by `gsd/parity/record-reports.sh` into
`gsd/parity/goldens/<fixture>/reports/`, the LCOV `SF:` prefix rewritten to a relative path, and each row collects
with `--no-run --report <recorded>` into a scratch copy, with no Warning and no Error (an Info note is allowed). The
vault row loads no non-builtin extension. The fixtures stay untouched.

The reason: live tool runs in the gate would need seven tools and drift (an OSV database is a snapshot), so "no
diagnostics" would not reproduce; recorded output keeps the A/B trees and their pins as they are.

## Rejected

- **A shared `extensions/` workspace** to run every guest at once: one lockfile per blob (ADR 0366 D2).
- **Passing the root nextest config** to a guest: its overrides fail the parse (D1).
- **A gate-side exclude** of fixture and example crates: the gate would compute another number than a user's
  `specforge` (ADR 0388 D1).
- **A test id on every `describe`, or on none**: node reports a suite as a test case exactly when nothing ran inside
  it (D4), so the id follows what the suite holds.
- **A new item kind** for JavaScript tests: it re-vendors every code-facts linker and `@code/core`.
- **Live tool runs in the gate** for the fixtures' packs: too many tools, and their output drifts.
- **Editing the fixtures** so every pack applies: it re-pins three A/B trees and re-records their baselines.
- **`dogfood` on pull requests**: deferred; the owner keeps it push only, so a missed test turns main red after merge.

## Consequences

- X2 reads 100% on SpecForge, taskboard, ledger and vault, and a regression fails `dogfood`.
- JavaScript test items appear in every JavaScript project's graph and in the A/B B arm (costly, owner-accepted); the
  A/B tasks, trees and pins are not changed.
- The guests' and the `@gsd` workspace's reports are CI artifacts; the CI time budgets are re-measured on the first
  push (ADR 0375 D3).

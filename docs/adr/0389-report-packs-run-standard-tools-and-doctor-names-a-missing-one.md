# Report packs run standard tools, and doctor names a missing one

**Status:** accepted (2026-11-23, round r6-breadth, Phase 11). Amended by
[ADR 0390](0390-a-milestone-computes-its-api-breaks-and-release-evidence-and-two-guards-read-them.md) (D9: every
accepted risk listed). **Amends**
[ADR 0348](0348-a-collector-declares-a-config-file-and-the-rust-packs.md) (a report may be a glob; the host reports a
collector's warnings). **Follows** [ADR 0388](0388-x2-is-the-share-of-test-ids-a-standard-report-saw.md) (the readers
and kinds), [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) and
[ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md) (test, item and package ids),
[ADR 0345](0345-the-syntax-import-and-collectors-that-read-answer-and-are-served-facts.md) (collectors served facts),
[ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md) (builtin
vocabulary), [ADR 0328](0328-a-builtin-is-embedded-and-a-non-specforge-builtin-catalogues-its-codes.md) (extension
libraries), [ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) D4 (the walk rules) and
[ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D3 (the size budget rule).

## Context

- Phase 10 gave `@code/core` five report readers (`specforge-code-{lcov,sarif,cyclonedx,osv,api}`) and five kinds
  (`coverage`, `static_finding`, `dependency`, `vulnerability`, `api_item`), but no pack runs a tool whose report they
  read. Requirements PACK-01 to PACK-07 ask for those packs, Rust and JavaScript, and for `specforge doctor` to name a
  tool the machine lacks with the codes that then cannot be judged.
- The guest never runs anything: the host runs the declared argv with consent, reads one report (a file, or the
  `*.json` files directly in a directory) or the captured standard output, and hands it to the pure export. It cannot
  pipe, and it sets no variable but `SPECFORGE_REPORT`.
- A protocol-types or SDK change makes every builtin blob stale, and blobs rebuild on linux/amd64 only. So every
  protocol and host change the packs need lands first (plan 11-01) and the series re-vendors once (plan 11-09).
- Two reader outcomes had no way to reach the user: `Vulnerabilities::skipped` (records that minted nothing) and a
  report path outside the project, which made the LCOV and SARIF readers fail the whole report.

## D1. Twelve packs, one crate and one blob each

Rust: `@rust/coverage`, `@rust/lint`, `@rust/deps`, `@rust/api`, `@rust/proof`. JavaScript: `@javascript/coverage`,
`@javascript/lint`, `@javascript/deps`, `@javascript/api`, `@javascript/vitest`, `@javascript/jest`,
`@javascript/playwright`. Each peers `@code/core` and its source pack (`@rust/source` or `@javascript/source`). The two
`deps` packs hold two collectors each (CycloneDX and OSV), because a collector runs one command: fourteen collectors.
Every pack joins both builtin families (`Family::Old` and `Family::Gsd`); the legacy `@specforge/vitest` stays in
`Family::Old` beside `@javascript/vitest` (owner decision D4). vitest, jest and Playwright report JUnit XML, read by
`specforge-code-junit`; their test ids and places follow `@javascript/node-test`.

## D2. Kani for Rust only: no JavaScript proof pack

`@rust/proof` runs Kani. There is no JavaScript proof pack, because no JavaScript tool proves a property of all inputs
and reports per-harness verdicts the way Kani does: property-based testing such as fast-check samples inputs and
already reports as tests through the runner packs, and TypeScript's checker proves types, not behaviour. A JS proof
pack would be a test runner under another name. Owner decision D2.

## D3. Doctor's tool notes

- The program is the collector argv's first item, probed through the process port's `which`. Existence checks only:
  nothing is run.
- A collector whose command runs another program declares it in `requires` (`CollectorDescriptor.requires`, protocol
  2.3.0; the SDK's `.requires(&[..])`): `rust-coverage`'s `cargo-llvm-cov llvm-cov nextest` requires `cargo-nextest`.
  Doctor looks each up on PATH as it does the first, and a missing one is its own note naming the collector and its
  install line (which installs all its programs). A name that is no bare program name is W021 and never looked up.
- For `npx --no -- <bin>` the program is `<bin>`, present when `node_modules/.bin/<bin>` (or `<bin>.cmd`) is under the
  project root, or under an npm workspace package directory the root `package.json` names (`workspaces`: a literal
  directory or `dir/*`), or `which` finds it. Every file check is held by the project's containment, so nothing above
  the root is read: a binary hoisted to a parent directory is not seen, and the note says it is missing. For
  `node … node_modules/<pkg>/…` it is `node` and the script, the note naming `<pkg>`.
- Only collectors detected in this project are probed.
- One Info note per missing program, grouped (two collectors on one program give one note), with the union of their
  codes and the collector's install line (ADR 0348 D4).
- The codes are every loaded code whose `facts` (D4) meet the missing collectors' `answers` and meet no kind a
  runnable detected collector answers (its program found, or no program at all, like a source reader), derived
  through `Codebook::switched_off`, never hand-written per pack: a code that judges a kind another collector still
  answers is still judged, so it is not named.
- A missing tool never fails doctor: `ok()` does not change.

## D4. `CodeDescriptor.facts`, declared once by `@code/core`

A code entry may name the collected fact kinds it judges (`facts`, protocol 2.3.0; the SDK's `.facts(&[..])`), so a
host can say which codes no finding can speak for when no collector answers those kinds. `@code/core` declares it on
its own codes once. It is absent from the wire when empty, so a code about authored entities keeps today's bytes. The
registry keeps it on the code book and does not validate the kind names: a kind no extension declares is simply never
matched (T-11-06 accepted).

## D5. `CollectOutput.warnings` and W203

A collector's output may carry `warnings` (protocol 2.3.0): lines about entries it read but could not use. The host
reports each as one core W203 naming the collector and its extension, escaped through `printable`, at most 100 per
collector and then one W203 counting the rest. The facts it did answer are kept. A warning is never an error: an error
is the export's `Err`, and fails the collect as before.

## D6. A report may be a glob

cargo-cyclonedx writes `<name>.cdx.json` beside each member manifest and has no output path, so `report(...)` accepts
a project-relative glob (`**/rust-sbom.cdx.json`). Its last segment is a literal file name, so a glob names the files
one tool writes and never every file of a kind (`**/*`, `**/*.pem`, `**/.env*`): the SDK's `report` panics on a
wildcard there, the registry refuses a raw declaration of one (E030), and the host's collect refuses it too (E058).
After a run the host reads every match modified since the run started; with `--no-run`, every match. A report's files
hold at most 64 MiB together (`REPORT_BYTES_MAX`, a captured output's cap), past which the collect is E045. The walk is `Input::files`' own (`glob_paths`, ADR 0293 D4): directories in
`SKIPPED` (`target`, `node_modules`, …) are not entered unless the glob names them, a directory reached through a link
is not entered, and the walk stops at 20,000 entries, which is E045 naming the cap. Each match is held by containment:
one whose link lands outside the project is never read, and one W203 per collector names such matches (the E045
"produced no report" names them too when no other match was read). Nothing is removed before the run, and the command is told no
path: `{report}` in the argv or config, or `capture_stdout()`, beside a glob is E058, and `SPECFORGE_REPORT` is not
set. No match is the usual E045 ("produced no report").

## D7. The LCOV and SARIF readers skip a path outside the project

A file record whose path lies outside the project is skipped and counted, and the count is surfaced as a W203 line,
instead of failing the whole report. The readers still touch no file system, so ADR 0388's threat holds. A SARIF
artifact with a non-`file` URI scheme still fails the report. A path naming no file is not "outside": one that
resolves to the project root itself (`.`, `a/..`, the root's absolute path) or a compiler's pseudo name in angle
brackets (rustc's `<std macros>`, `<anon>`) is `Place::NoFile`. SARIF and clippy skip and count such a result in a
warning line of its own, never minting a finding at a file no one can open; an LCOV `SF` naming no file fails the
report, as an empty `SF` does.

## D8. The commands per pack

| Collector | argv (first items) | Report | Filter | Install line |
|-----------|--------------------|--------|--------|--------------|
| `rust-coverage` | `cargo-llvm-cov llvm-cov nextest --workspace --ignore-run-fail --lcov --output-path {report}` | `.specforge/reports/rust-coverage.lcov` | none | `cargo install cargo-llvm-cov --locked` |
| `rust-lint` | `cargo-clippy clippy --workspace --all-targets --message-format=json` | captured stdout under `.specforge/reports/rust-lint` | none | `rustup component add clippy` |
| `rust-sbom` | `cargo-cyclonedx cyclonedx --format json --override-filename rust-sbom.cdx.json …` | glob `**/rust-sbom.cdx.json` | none | `cargo install cargo-cyclonedx --locked` |
| `rust-osv` | `osv-scanner scan source --lockfile Cargo.lock --format json --output {report}` | `.specforge/reports/rust-osv.json` | none | `brew install osv-scanner` (or the release binary) |
| `rust-api` | `cargo-public-api --color never -ss` | captured stdout under `.specforge/reports/rust-api` | `--package {filter}` | `cargo install cargo-public-api --locked` |
| `rust-proof` | `cargo-kani --workspace --output-format regular` | captured stdout under `.specforge/reports/rust-proof` | `--harness {filter}` | `cargo install --locked kani-verifier && cargo kani setup` |
| `javascript-coverage` | `npx --no -- c8 --reporter=lcovonly --reports-dir=.specforge/reports/javascript-coverage …` | `.specforge/reports/javascript-coverage` | none | `npm install --save-dev c8` |
| `javascript-lint` | `npx --no -- eslint --format @microsoft/eslint-formatter-sarif --output-file {report}` | `.specforge/reports/javascript-lint.sarif` | none | `npm install --save-dev eslint @microsoft/eslint-formatter-sarif` |
| `javascript-sbom` | `npx --no -- cyclonedx-npm --output-format JSON --spec-version 1.6 --package-lock-only …` | `.specforge/reports/javascript-sbom.cdx.json` | none | `npm install --save-dev @cyclonedx/cyclonedx-npm` |
| `javascript-osv` | `osv-scanner scan source --lockfile package-lock.json --format json --output {report}` | `.specforge/reports/javascript-osv.json` | none | `brew install osv-scanner` (or the release binary) |
| `javascript-api` | `npx --no -- api-extractor run --local --config {config}` | `.specforge/reports/javascript-api.api.json` | none | `npm install --save-dev @microsoft/api-extractor typescript` |
| `javascript-vitest` | `npx --no -- vitest run --reporter=default --reporter=junit --outputFile.junit={report}` | `.specforge/reports/javascript-vitest.xml` | `-t {filter}` | `npm install --save-dev vitest` |
| `javascript-jest` | `node --env-file={config} node_modules/jest/bin/jest.js --reporters=default --reporters=jest-junit` | `.specforge/reports/javascript-jest.xml` | `-t {filter}` | `npm install --save-dev jest jest-junit` |
| `javascript-playwright` | `npx --no -- playwright test --reporter=junit …` | captured stdout under `.specforge/reports/javascript-playwright` | `--grep {filter}` | `npm install --save-dev @playwright/test` |

The plan that builds each pack fixes its exact argv and names; this table is the design they implement.

A captured output that cannot be told from a clean run is never read as one. clippy's capture without cargo's last
`build-finished` record is a run cut off (killed between two lines), and is the collector's error. Kani's and
cargo-public-api's captures with nothing read are a warning (W203) saying nothing was checked or recorded, not an
error: "no harness" and "no public item" are real outcomes the text cannot tell from a tool that stopped at the build,
a wholly empty capture is already the host's E045, and an export's error would fail the whole collect.

## D9. Tool limits, accepted and documented

- vitest's and jest's `-t` filters match test names, not ids.
- jest needs `jest-junit` installed in the project and runs through `node --env-file`, because the report path reaches
  jest-junit only through a variable.
- Playwright reports paths relative to its `testDir`.
- `cargo public-api` reads one package per run (the root package; the filter names another) and needs a nightly
  toolchain.
- api-extractor reads built declarations: the pack's config names `<projectFolder>/dist/index.d.ts` as its entry, so
  the project builds them there first; the pack runs no build.
- Each risk below is accepted, and each runs only after the collector's command is approved; the consent prompt shows
  the command.
  - The coverage packs and the runner packs (vitest, jest, Playwright) run the project's own tests.
  - osv-scanner sends the project's package names and versions to the OSV API.
  - ESLint loads the project's own plugins and configuration code.
  - Kani builds and model-checks the project's code.
  - cargo-cyclonedx, cargo public-api and clippy run the project's build scripts.
- osv-scanner exits 1 when it finds vulnerabilities, which the host does not judge: it reads the report whatever the
  exit code.

## D10. Kani ids

`@rust/proof` mints `rust:<crate>#<harness path>` (`TestId::rust`), the crate found as the one whose served items hold
every harness of a `Complete` block. `@rust/source` is unchanged.

## D11. Enablement and size

`specforge init` enables none of the packs. `TOTAL_BUDGET` is measured once the blobs exist and raised by ADR 0373
D3's rule.

## D12. Windows

Kani is unsupported on Windows (documented). The `npx` and `node --env-file` routes are unverified on Windows, and
said so.

## D13. The five report kind names stay

A project loading `@code/core` cannot name an entity `coverage`, `api_item`, `static_finding`, `dependency` or
`vulnerability` (E013), as ADR 0388 records. Renaming them would re-vendor every blob that links `code-facts` for no
gain the owner asked for.

## Rejected

- **A collector `switches_off` field.** Twelve packs would restate one table, and drift from the codes' own passes.
  The code says what it judges once (D4).
- **A shared pack library.** It would be an input of every pack blob, so a change to it would re-vendor all twelve.
- **Running `npx --no -- <bin> --version` in doctor.** It executes project code; existence checks run nothing.

## Consequences

- One re-vendor for the whole series (plan 11-09); the `builtins` gate check is red until then.
- The readers' out-of-project tests change from "fails the report" to "skips and counts".
- W102 names every runner pack.
- `CollectOutput.warnings`, `CodeDescriptor.facts` and report globs join the 2.3.0 protocol row; a host that does not
  know them ignores them.

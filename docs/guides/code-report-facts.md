# Standard reports as code facts

Five standard reports become facts a spec can cite: coverage, API snapshots, lint findings, bills of materials and
vulnerability scans. Each is read by its own extension library, and `specforge-code-facts` mints the facts from what it
reads. `@code/core` declares the five report kinds and checks them
([ADR 0388](../adr/0388-x2-is-the-share-of-test-ids-a-standard-report-saw.md)).

| Report | Library | Kind | Scheme |
|--------|---------|------|--------|
| LCOV tracefile | `specforge-code-lcov` | `coverage` | `cov` |
| `cargo public-api` text, api-extractor `.api.json` and `.api.md` | `specforge-code-api` | `api_item` | `api` |
| SARIF 2.1.0 log | `specforge-code-sarif` | `static_finding` | `sarif` |
| CycloneDX JSON BOM | `specforge-code-cyclonedx` | `dependency` | `dep` |
| `osv-scanner --format json` report | `specforge-code-osv` | `vulnerability` | `vuln` |

The libraries are rlibs in `extensions/code/`. The report packs link them: each runs one standard tool, after you
approve its command once per project, and collects its report
([ADR 0389](../adr/0389-report-packs-run-standard-tools-and-doctor-names-a-missing-one.md)). Enable one with
`specforge add @rust/coverage`; it also enables the source pack whose facts it reads, and `specforge collect` runs it
when its project file is found. A report fact cites the facts the source and manifest packs collected, the `item`
holding its location or the `package` it is about, so those packs collect first. `specforge doctor` names each tool a
detected pack runs that is not installed, with its install line and the codes it feeds.

| Pack | Collector | Tool and install line | Reader | Facts |
|------|-----------|-----------------------|--------|-------|
| `@rust/coverage` | `rust-coverage` | cargo-llvm-cov over nextest: `cargo install cargo-llvm-cov cargo-nextest --locked` | `specforge-code-lcov` | `coverage` |
| `@rust/lint` | `rust-lint` | clippy, its JSON captured: `rustup component add clippy` | its own clippy converter | `static_finding` |
| `@rust/deps` | `rust-sbom` | cargo-cyclonedx: `cargo install cargo-cyclonedx --locked` | `specforge-code-cyclonedx` | `dependency` |
| `@rust/deps` | `rust-osv` | osv-scanner on `Cargo.lock`: `go install github.com/google/osv-scanner/v2/cmd/osv-scanner@latest` | `specforge-code-osv` | `vulnerability` |
| `@rust/api` | `rust-api` | cargo public-api, captured: `cargo install cargo-public-api --locked` (and a nightly toolchain) | `specforge-code-api` | `api_item` |
| `@rust/proof` | `rust-proof` | Kani, captured: `cargo install --locked kani-verifier && cargo kani setup` | `specforge-code-kani` | `test`, `test_run` |
| `@javascript/coverage` | `javascript-coverage` | c8 around `npm test`: `npm install --save-dev c8` | `specforge-code-lcov` | `coverage` |
| `@javascript/lint` | `javascript-lint` | ESLint's SARIF formatter: `npm install --save-dev eslint @microsoft/eslint-formatter-sarif` | `specforge-code-sarif` | `static_finding` |
| `@javascript/deps` | `javascript-sbom` | cyclonedx-npm: `npm install --save-dev @cyclonedx/cyclonedx-npm` | `specforge-code-cyclonedx` | `dependency` |
| `@javascript/deps` | `javascript-osv` | osv-scanner on `package-lock.json`, as for Rust | `specforge-code-osv` | `vulnerability` |
| `@javascript/api` | `javascript-api` | api-extractor, a config the host writes: `npm install --save-dev @microsoft/api-extractor typescript` | `specforge-code-api` | `api_item` |
| `@javascript/vitest` | `javascript-vitest` | vitest's JUnit reporter: `npm install --save-dev vitest` | `specforge-code-junit` | `test`, `test_run` |
| `@javascript/jest` | `javascript-jest` | jest with jest-junit: `npm install --save-dev jest jest-junit` | `specforge-code-junit` | `test`, `test_run` |
| `@javascript/playwright` | `javascript-playwright` | Playwright's JUnit, captured: `npm install --save-dev @playwright/test` | `specforge-code-junit` | `test`, `test_run` |

`@rust/proof` mints a `test` per Kani harness, `rust:<crate>#<harness path>`, placed at its function. Kani runs on
Linux x86_64 and macOS only, not on Windows. There is no JavaScript proof pack: no JavaScript tool proves a property of
all inputs with a verdict per harness the way Kani does, property-based testing already reports as tests through the
runner packs, and TypeScript's checker proves types, not behaviour (ADR 0389 D2). The legacy `@specforge/vitest`
stays a separate pack beside `@javascript/vitest`; their collectors are `vitest` and `javascript-vitest`.

## What each reader accepts and mints

**LCOV.** `read` takes a tracefile as geninfo(1), `cargo llvm-cov --lcov` and c8 or istanbul `lcovonly` write it. It
reads `FN`/`FNDA` (the n-th `FNDA` of a name pairs with the n-th `FN` of it, so two impls' `new` keep their own
counts), `FNL`/`FNA`, `DA` and `BRDA`, and ignores `TN`, `VER`, the file totals, `MCDC`, comments and unknown
keys. Sections of one file merge. One `coverage` fact per innermost item holding reported lines, `cov:<item id>`, cites
that item (edge `covers`); what no item of a file holds is `cov:@<file>`. The counts are `functions_found`/`_hit`,
`lines_found`/`_hit` and `branches_found`/`_hit`.

**API snapshots.** `read_rust_text` reads a `cargo public-api` text snapshot; `read_js_model` reads an api-extractor
doc model. No format is sniffed: a pack calls the reader of its own. One `api_item` per public entry,
`api:rust:<crate>::<path>` (a trait-impl member `…::<Type as Trait>::m`) or `api:js:<package>::<name>[.<member>]`, cites
the item with that path (edge `api_of`). The id holds no line, no order and no source file; `signature` is kept
verbatim. The same id with another signature takes `~2`. `read_js_report` reads an api-extractor API report
(`.api.md`), the accepted side below.

**SARIF.** `read` takes version `2.1.0` only. The rule is `ruleId`, else `rule.id`, else the driver rule its index
names; the level is the result's, else `none` for a `kind` other than `fail`, else the rule's
`defaultConfiguration.level`, else `unknown`. SARIF's own `warning` default is never applied. One `static_finding` per
result, `sarif:<tool>/<rule>#<file>:<line>` (`~2` on a repeat), cites the innermost item holding its line (edge
`found_in`), else the package holding its file (edge `found_in_package`). Of two packages at one directory (a crate
with a `package.json`), it is the one of the file's ecosystem, read from an item on the file, else its manifest name or
extension; with no such evidence the finding names neither.

**CycloneDX.** `read` takes `bomFormat` `CycloneDX` with `specVersion` `1.4`, `1.5` or `1.6`, and needs
`metadata.component`, the project package the BOM describes: of two packages of one name, the one whose directory is
the root purl's subpath, else the one whose id is the purl's exactly. Components flatten depth first; a missing `scope` reads
`required`; `direct` is set only when the BOM states the root's `dependsOn`. One `dependency` per component,
`dep:<package id>/<purl without qualifiers and subpath>`, cites that package (edge `dependency_of`); a component with
no purl is `dep:<package id>/@<[group/]name[@version]>`.

**OSV.** `read` takes an osv-scanner report whose sources are `lockfile` or `sbom`. Each scanned lockfile cites the
project package of its ecosystem at its directory, else each one under it. One `vulnerability` per package and OSV id,
`vuln:<package id>/<OSV id>`, cites that package (edge `affects`); purls, versions and aliases of one id are united.
A package of an ecosystem other than `crates.io` and `npm`, or one whose lockfile no collected package of its type
holds, mints nothing and is named in `Vulnerabilities::skipped` (name, version, lockfile, record count and reason), so
a `requirements.txt` beside a `Cargo.lock` never hides the cargo findings.

A report id's parts are percent-encoded (`%`, `/`, `#`, `:`, `~`, `@`, `?`, whitespace and control characters), so an
id parses back to its parts and no report text forges a delimiter or a `~2` suffix. A Rust API path segment keeps a
plain space (`<Type as Trait>`), and a JavaScript API name encodes its own `.` (`%2E`), which joins a member path.

## The accepted API side

An `api_item` says what the public API is now. The API packs also say how it moved since it was last accepted
([ADR 0390](../adr/0390-a-milestone-computes-its-api-breaks-and-release-evidence-and-two-guards-read-them.md) D3). The
accepted side is the report the project commits for review, each tool's own baseline:

| Pack | Read | Glob | The committed file |
|------|------|------|--------------------|
| `@rust/api` | `accepted` | `**/public-api.txt` | cargo-public-api's snapshot (`tests/public-api.txt`, `tests/snapshots/public-api.txt`), written with the pack's own flags: `cargo public-api --color never -ss > public-api.txt` |
| `@javascript/api` | `accepted` | `**/etc/*.api.md` | api-extractor's API report, `etc/<package>.api.md` |
| `@javascript/api` | `reviews` | `common/reviews/api/*.api.md` | rush's API report folder |

Never `**/*.api.md`: api-extractor's `temp/` holds the run's own copy of the report, which would hide every break. The
`.alpha.api.md`, `.beta.api.md` and `.public.api.md` variants are not read either.

The packs read the file as a declared file read, `Input::files(..)` with `FileReadSpec::text_up_to(4_194_304)`: the host
refuses a file outside the project or reached through a link, and never sends a credential-looking file as text, as
for any read. `text_up_to`
raises a read's text ceiling from 64 KiB to at most 4 MiB, and its budget to the larger of 1 MiB and four times the
ceiling (protocol 2.3.0). The file is not read as a report: a report is read only when the run wrote it, and a
committed file is older than the run.

**`change`.** Each `api_item` of a crate or package present on both sides carries `change`: `added`, `unchanged`,
`changed`, `narrowed` or `removed`. A changed, narrowed or removed entry also carries `accepted_signature`,
`accepted_tag` (when the accepted entry has one) and `accepted_visibility` (when it differs). The id does not change,
and an entry gone from the code is a fact with `change` `removed`, which `code/W104` skips. A crate or package on one
side only is not judged: its facts carry no `change`. Accepted `@internal` entries are skipped. The release tags rank
Public, Beta, Alpha, Internal, and a member with no tag of its own inherits its container's, so narrowing a class
narrows each member. Entries pair by base id, then by comparable signature, never by a `~2` suffix. `changed`,
`narrowed` and `removed` are breaking, `added` is not: `@gsd/core` lists the breaking ones in `milestone.api_breaks`
and `gsd/E533` refuses to close a milestone whose version does not allow them.

**Outcomes.**

- No accepted file: no fact carries `change`. Committing the report is the opt-in.
- An accepted file of the run's own crate or package (its `pub mod <crate>` line, its report title) that does not
  parse (a merge-conflict marker, a broken fence), or two accepted files of one crate or package (an old
  `tests/public-api.txt` kept beside `tests/snapshots/public-api.txt`): the collector's error naming them, and the
  collect stores nothing of the run.
- A file the host did not serve (`too_large`, `budget`, `not_utf8`), a matched file that is no report or snapshot
  (prose in an `etc/` folder), or one naming no crate or package the run read, broken or not: one warning naming it,
  shown as W203, no break judged against it, and the collect goes on.

**Accepting a break.** The packs never write the accepted file: api-extractor's `apiReport` stays disabled in the pack's
config. You accept a break by committing the updated report: `cargo public-api --color never -ss >
tests/public-api.txt`, or `api-extractor run --local` with `apiReport` enabled in your own config (it copies
`temp/<package>.api.md` to `etc/`). Then run `specforge collect`. Write the file with the tool version CI runs: a tool
upgrade can change every line.

**Limits.** A required method added to a trait cannot be seen in the text. A Rust visibility narrowing reads as
`removed`, because cargo public-api prints public items only. An attribute change (`#[non_exhaustive]`, `#[must_use]`)
reads as `changed`. The globs walk the whole project but the skipped folders (`.git`, `node_modules`, `target`, …), and
a walk stops after 20,000 entries (`GLOB_VISITS_MAX`) or 2,000 matches (`FILE_READ_MAX`): in a larger repository the
accepted files past the cut are not read, no break is judged for their crates or packages, and the pack's W203 says the
read stopped early, naming both caps.

## What a reader refuses

A reader returns an error, and the collect mints nothing from that report, when:

- a SARIF URI is of a scheme other than `file:` (`https:`), so it names no file of the project;

  A path outside the project is not refused: an LCOV `SF` section, or a SARIF result whose `file:` URI (or relative
  URI climbing out through `..`) the project does not hold, is skipped and counted, never opened, and the pack reports
  the count and the paths as W203 (ADR 0389 D7).
- the version is one it does not read (SARIF other than `2.1.0`, CycloneDX other than `1.4` to `1.6`);
- the document is malformed or lacks a required key: the error names the line, the JSON path or the value;
- a BOM's `metadata.component` belongs to no collected package.

## Reserved entity names

The five kinds are declared by `@code/core`, so a project that loads it cannot name an entity `coverage`, `api_item`,
`static_finding`, `dependency` or `vulnerability`: the compiler reports E013. Rename the entity (for example
`test_coverage`); the reader facts are not entities and are not affected. ADR 0388 records the choice.

## What stays unmapped

An entry the reader cannot place is kept, never dropped, and the `reports` pass of `@code/core` raises one warning per
class. The pass is sent only the five report kinds, and skips a pending fact.

| Code | Class |
|------|-------|
| `code/W104` | A `coverage`, an `api_item` of a kind an item can have (not a crate root), or a `static_finding` that names no item; a finding is reported only when no package holds its file either |
| `code/W105` | A `static_finding` whose rule or level reads `unknown` |
| `code/W106` | A `dependency` with no purl, named by name and version only |
| `code/W107` | A `vulnerability` whose severity is `unknown` |

`specforge explain code/W104` prints each code's explanation and remedy.

## Severity

A vulnerability's severity is one of `critical`, `high`, `medium`, `low`, `unknown`, in that order. It is the OSV
record's own label (`database_specific.severity`; `MODERATE` reads `medium`). With no label, it is the rating of the
record's highest CVSS v3 base score: the v3.0 or v3.1 base formula with its integer round-up, in tenths, rated `low`
under 4.0, `medium` under 7.0, `high` under 9.0, else `critical`. A v4 or v2 vector, or a malformed v3 one, is not
rated: with nothing else the severity is `unknown` (`code/W107`), never a guess. One id merged from several records keeps the
highest.

## The X2 line

X2 is the share of items carrying a test id whose test a standard test report saw. `cargo gate full` prints it in its
dogfood log for SpecForge, the taskboard fixture and the ledger fixture:

```
X2 SpecForge: n/m (p.p%)
X2 taskboard: 0/0 (no item has a test_id)
X2 ledger: 6/6 (100.0%)
```

It is printed, not enforced: ADR 0388 sets no floor. The percent rounds down, so `100.0%` means no item is missing. An
export of another shape is the check's error, never a `0/0` line.

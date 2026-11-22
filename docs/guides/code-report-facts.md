# Standard reports as code facts

Five standard reports become facts a spec can cite: coverage, API snapshots, lint findings, bills of materials and
vulnerability scans. Each is read by its own extension library, and `specforge-code-facts` mints the facts from what it
reads. `@code/core` declares the five report kinds and checks them
([ADR 0388](../adr/0388-x2-is-the-share-of-test-ids-a-standard-report-saw.md)).

| Report | Library | Kind | Scheme |
|--------|---------|------|--------|
| LCOV tracefile | `specforge-code-lcov` | `coverage` | `cov` |
| `cargo public-api` text, api-extractor `.api.json` | `specforge-code-api` | `api_item` | `api` |
| SARIF 2.1.0 log | `specforge-code-sarif` | `static_finding` | `sarif` |
| CycloneDX JSON BOM | `specforge-code-cyclonedx` | `dependency` | `dep` |
| `osv-scanner --format json` report | `specforge-code-osv` | `vulnerability` | `vuln` |

The libraries are rlibs in `extensions/code/`. No builtin links one yet: the packs that run the tools and collect these
reports come next (Phase 11 of the round). A report fact cites the facts the source and manifest packs collected, the
`item` holding its location or the `package` it is about, so collect those packs first.

## What each reader accepts and mints

**LCOV.** `read` takes a tracefile as geninfo(1), `cargo llvm-cov --lcov` and c8 or istanbul `lcovonly` write it. It
reads `FN`/`FNDA`, `FNL`/`FNA`, `DA` and `BRDA`, and ignores `TN`, `VER`, the file totals, `MCDC`, comments and unknown
keys. Sections of one file merge. One `coverage` fact per innermost item holding reported lines, `cov:<item id>`, cites
that item (edge `covers`); what no item of a file holds is `cov:@<file>`. The counts are `functions_found`/`_hit`,
`lines_found`/`_hit` and `branches_found`/`_hit`.

**API snapshots.** `read_rust_text` reads a `cargo public-api` text snapshot; `read_js_model` reads an api-extractor
doc model. No format is sniffed: a pack calls the reader of its own. One `api_item` per public entry,
`api:rust:<crate>::<path>` (a trait-impl member `…::<Type as Trait>::m`) or `api:js:<package>::<name>[.<member>]`, cites
the item with that path (edge `api_of`). The id holds no line, no order and no source file; `signature` is kept
verbatim. The same id with another signature takes `~2`.

**SARIF.** `read` takes version `2.1.0` only. The rule is `ruleId`, else `rule.id`, else the driver rule its index
names; the level is the result's, else `none` for a `kind` other than `fail`, else the rule's
`defaultConfiguration.level`, else `unknown`. SARIF's own `warning` default is never applied. One `static_finding` per
result, `sarif:<tool>/<rule>#<file>:<line>` (`~2` on a repeat), cites the innermost item holding its line (edge
`found_in`), else the package holding its file (edge `found_in_package`).

**CycloneDX.** `read` takes `bomFormat` `CycloneDX` with `specVersion` `1.4`, `1.5` or `1.6`, and needs
`metadata.component`, the project package the BOM describes. Components flatten depth first; a missing `scope` reads
`required`; `direct` is set only when the BOM states the root's `dependsOn`. One `dependency` per component,
`dep:<package id>/<purl without qualifiers and subpath>`, cites that package (edge `dependency_of`); a component with
no purl is `dep:<package id>/@<[group/]name[@version]>`.

**OSV.** `read` takes an osv-scanner report whose sources are `lockfile` or `sbom`. Each scanned lockfile cites the
project package of its ecosystem at its directory, else each one under it. One `vulnerability` per package and OSV id,
`vuln:<package id>/<OSV id>`, cites that package (edge `affects`); purls, versions and aliases of one id are united.

A report id's parts are percent-encoded (`%`, `/`, `#`, `:`, `~`, `@`, `?`, whitespace), so an id parses back to its
parts.

## What a reader refuses

A reader returns an error, and the collect mints nothing from that report, when:

- a path lies outside the project (a SARIF `file:` URI or an LCOV `SF` path the project does not hold);
- the version is one it does not read (SARIF other than `2.1.0`, CycloneDX other than `1.4` to `1.6`);
- the document is malformed or lacks a required key: the error names the line, the JSON path or the value;
- a BOM's `metadata.component` or a scanned lockfile belongs to no collected package, or an OSV package's ecosystem is
  neither `crates.io` nor `npm`.

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

It is printed, not enforced: ADR 0388 sets no floor.

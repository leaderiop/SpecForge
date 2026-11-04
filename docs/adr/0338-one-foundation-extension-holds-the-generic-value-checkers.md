# One foundation extension holds the generic value checkers

**Status:** accepted (2026-11-04). Builds on
[ADR 0336](0336-a-field-declares-the-checks-its-values-pass.md), and records its D8. **Amends 0336:** its probe set
(D9) gains `fs.digest{sha256}`. The host streams a file of any size under `Contained`, so a pin can hold a large
artifact. **Amends 0336 again** for the structured, media and registries families: a read names its byte range
([D10](#d10-a-read-names-its-byte-range)), a fixed-host capability may POST a query ([D12](#d12-a-fixed-host-capability-may-post-a-query)),
and a judge may ask two more rounds of probes, which amends its D4 ([D11](#d11-a-judge-may-ask-twice-more)).

## Context

- A check per extension costs each one the SDK floor, 428 KB, in every binary.
- A project that wants `url` and `file` and `iso_date` would approve, load and update three extensions.
- Standards that take no judgement (a SemVer, a Luhn digit, a BCP 47 tag) belong in one place; identifiers that
  belong to a sector or a jurisdiction do not.

## D1. The foundation

`@specforge/checks`, the twelfth builtin, holds every generic checker, in families. Domain packs (`@pharma/ids`:
CAS, NDC, ATC) peer-depend on it and build on [the kit](#d5-the-kit); they are never inside it.

## D2. The admission rule

A checker belongs only if its validity is decided entirely by a cross-industry, non-jurisdictional standard (ISO/IEC,
IETF, W3C, Unicode, SPDX, GS1 mod-10; ITU-T for E.164), with no registry lookup and no sector meaning.

A notation's own published grammar is its standard when it is cross-industry and has no sector or jurisdiction, and the
checker names that grammar. So `regex` (the `regex` crate's syntax) and `semver_range` (Cargo's `VersionReq`) belong.

## D3. Families, scopes and switches

| Family | Checkers | Scope | Capability |
|---|---|---|---|
| files | `file`, `dir`, `glob`, `size`, `digest`, `encoding`, `line_range` | `files` | `fs` |
| formats | `email`, `semver`, `semver_range`, `spdx`, `iso_date`, `iso_duration`, `cron`, `regex`, `uuid`, `mime`, `bcp47`, `iso3166`, `iso4217`, `iana_tz`, `e164`, `ip`, `cidr`, `hostname`, `port`, `hex_color` | none | none |
| digits | `luhn`, `mod11`, `mod97`, `gtin`, `isbn`, `iban` | none | none |
| links | `url`, `url_anchor`, `content_type`, `tls`, `redirect`, `dns` | `links` | `http` |
| git | `commit`, `ref`, `file_at`, `tracked` | `git` | `git` |
| structured | `json`, `yaml`, `toml`, `pointer`, `markdown_anchor` | `files` | `fs` |
| media | `image`, `media`, `svg`, `pdf` | `files`, and `media-remote` for a URL | `fs`, `http` (GET) |
| registries | `package`, `advisory`, `license_of`, `doi`, `orcid` | `registries` | `http` (GET, POST query; [D15](#d15-registries)) |

- There is no `dns` capability and no Dns port: name resolution and TLS information are operations of the Http port, in
  the `links` scope.
- Each family has a bool setting, `families.<family>`, on by default. It works twice:
  - each checker carries `when: ["families.<family>"]`, so a family that is off is not called, sends no probe and asks
    for no scope;
  - it `gates` the codes only its family reports, so `specforge settings` shows what a switch silences.
- A code more than one family reports is shared, so no switch gates it, and `when` alone switches it: E103, E016,
  W176 (files, structured, media), W172, W173, W181, W182 (links, media, registries).

## D4. Codes

The codes are bare, with owner `checks`, in the static catalog, and `@specforge/checks` declares no `codes` (ADR 0286
D2; ADR 0328 D2's rule is for a builtin outside `@specforge`). Another severity is another code (ADR 0026 D3), so no
checker takes a `severity` argument.

| Code | Family | Meaning |
|---|---|---|
| E016 | files, structured, media | the file does not exist; on the value's span, with a sibling only the host found inside the root |
| E096 | files | a file's digest differs from its pin |
| W176 | files, structured, media | a file's size, encoding or line range is not what the check asks, or a file is over what a check reads |
| E091 | links | the URL is malformed, relative or carries credentials; it is never fetched |
| E092 | links | the scheme is not allowed, on the scheme; `javascript:`, `data:` and `file:` never are |
| E093 | links | the URL answered 404 or 410 |
| W183 | links | the same, on a field whose check says `status: "warn"` |
| W182 | links, media, registries | 401 or 403: the page needs credentials, and whether it exists is not decided |
| W181 | links, media, registries | 429, 5xx or another non-success |
| W172 | links, media, registries | DNS, connect, TLS handshake or timeout failure, or a name that does not resolve |
| W173 | links, media, registries | a redirect that leaves the grant, an https to http downgrade, more than 5 hops, or a final host outside `hosts` |
| E104 | links | a `Content-Type` outside `allow`, or none |
| E112 | links | the URL's fragment names no `id` or `name` in the page; on the fragment |
| W178 | links | the certificate's `not_after` is within `days` |
| E106 | git | a commit, ref or file at a revision is not found |
| W180 | git | a path git does not track |
| E097 | structured | a JSON, YAML or TOML file does not parse; at the parser's line and column |
| E098 | structured | a pointer or a Markdown anchor names nothing in its file; on the `#` part |
| E099 | media | the file is not the image, SVG, PDF or container the check reads, or its format is not allowed |
| E113 | media | an SVG carries a script, an `on…` attribute, a `javascript:` URL or an unexpanded entity |
| W177 | media | a file over its `max_bytes`, `max_pixels` or `max_pages` |
| E105 | registries | the resolver's definite no: no such package, no release the requirement admits, an unknown DOI or iD |
| W179 | registries | the resolved release has an OSV advisory, or a licence outside `allow` |
| E103 | formats, digits, files, git, structured, media, registries | the value's text is malformed |

**The links status table.** An error only for a definite answer (404, 410), a malformed URL or a forbidden scheme. What
was not performed (`cached`, no approval, offline) has no finding: the host reports I206 or W170.

**Machine codes.** W172, W173, W181 and W182 are `machine`, the remote's answer as this machine's network, approvals or
secrets saw it: `--strict` never promotes one and the baseline never counts it. W178 (the target's certificate, which
the project asked about through `days`) and W183 (the project's own lowering) are not.

## D5. The kit

`specforge-checks-kit` is a leaf crate with no SpecForge dependency, for wasm32-wasip2 and native, with one feature per
family and no default features. It exports bare algorithms, parsers and pinned tables. It holds no probes, no roots
(the host alone reads a `<root>/` prefix, ADR 0336 D6) and no codes: codes and severity are the foundation's policy.

## D6. Engines

One meaning of "a regex": what the kernel's `matches` compiles with `regex::Regex::new`. The `regex` checker runs the
steps of `Regex::new` that can refuse a pattern, on the crates `regex` is built on: `regex-syntax`'s parser and
`regex-automata`'s forward and reverse Thompson NFA compilers under the same 10 MiB limit. It links no search engine
(the meta strategy, the DFAs, Aho-Corasick), about 390 KB of code, and a test compares its every refusal with
`Regex::new`'s.

## D7. Static tables

Each table is a module with a `SOURCE` naming its standard, edition, URL and retrieval date, and a test pins every
`SOURCE`. Each is refreshed from its source before every release, `SOURCE` and its pin in the same commit, then the
builtins re-vendored (`docs/agents/revendor.md#static-tables`). A silent edit shows in the pin, and staleness is a
decision at the release. SPDX's list comes from its crate, so refreshing it is `cargo update -p spdx`.

`e164` is structural: `+`, an assigned country calling code, at most 15 digits. Per-country numbering plans are
jurisdictional, about 1 MB, and go to packs.

## D8. The blob cap

Every committed blob is at most 2,500,000 B, held by the gate's `blob-size` check. It replaces the component test's
per-blob budget. The test's `TOTAL_BUDGET` stays: it alone catches a build without `[profile.blob]`.

Measured: `@specforge/checks` with its eight families and its collector is 2,381,815 B, 118,185 B under the cap. Its
largest parts are the `regex-syntax` parser with its Unicode tables and the Thompson NFA compiler (formats, D6), the
`url` parser with its IDNA tables (links), `jiff`, and the YAML and TOML parsers (structured). The twelve builtins
total 9,255,648 B, and `TOTAL_BUDGET` is 9,300,000.

| Families | `@specforge/checks` |
|---|---|
| files, formats, digits, links, git | 1,818,514 B |
| + structured, on the kit's heading scanner | 2,103,563 B |
| + media, on the kit's magic-number table | 2,215,486 B |
| + registries | 2,377,067 B |
| quick-xml 0.42 for 0.39 | 2,381,815 B |

## D9. Outcomes as facts

The `checks` collector declares the `check_outcomes` input and no detection files; `check_outcome` is the kind it
writes, in scheme `check`. A plain `collect` never runs it unless `checks.publish` is true (ADR 0336 D5). A fact's
`status` is `passed`, `failed` or `skipped`, the kind's result; `outcome` beside it holds the class. No outcome is ever
pending, so it sets no `complete_when`.

## D10. A read names its byte range

A read on either port names which bytes it wants: `ByteRange` is `Whole`, `Prefix`, `Suffix` or `Span`, on `fs.read`
and on `http.get`. It is one probe kind with a range, not a probe kind per end, so the journal digests one shape. The
host holds an fs prefix to 64 KiB and any ranged http read to 1 MiB, or to the capability's `max_bytes`. A ranged read
skips `regular`'s 64 MiB ceiling, which only stops a whole read of a huge file, and keeps its kind check. A server that
answers a suffix or a span with the whole body is sliced when the body is small, and is otherwise `range:ignored`
(W172).

## D11. A judge may ask twice more

A judge that has read the outcomes may ask for more probes (`Judged::more`, with a carry it reads back), at most three
rounds a value. The outcomes of every round reach it, in order. A PDF's page tree sits wherever its cross-reference
says, and a version requirement resolves to a release only once the index is read: one round would count neither.

## D12. A fixed-host capability may POST a query

An http capability whose only call kind is `check_values`, whose `hosts` are fixed and whose methods include POST may
send a bounded JSON query (`Probe::HttpPostQuery`); its approval line reads `POST (query)`. Its `max_bytes` may reach
16 MiB, the size of a large package's index. OSV's `v1/query` is POST-only. Such a POST reads: the grant says so
(`Granted::queries`), and that flag lets a fresh record answer it again as one answers a GET (ADR 0335 D4). A POST
from any other capability is never answered from a fresh record.

## D13. One tree for pointers

YAML and TOML convert to a `serde_json::Value` before a pointer runs, so one pointer reads all three, and a YAML file
means what its JSON twin means. A heading's anchor is the slug the kit computes from the CommonMark headings it scans.

## D14. Media

- **SVG active content is its own code, E113.** It is a security meaning distinct from a malformed file (ADR 0026 D3).
- **Limits are opt-in.** `max_bytes`, `max_pixels` and `max_pages` have no default, so a docs tree of large images is
  not a page of warnings. `formats` defaults to the web image set for `image` and to every known container for `media`.
- **A container is known by its magic number**, from the kit's table of eleven, not by a sniffing crate.

## D15. Registries

A registries checker belongs to the foundation only when both hold:

- its identifier is cross-industry: a package of crates.io, npm or PyPI, a DOI, an ORCID iD;
- its host is that identifier's own public resolver.

D2's "no registry lookup" is about the identifiers of a sector or a jurisdiction: they go to packs. The `registries`
scope names exactly seven hosts (`index.crates.io`, `crates.io`, `registry.npmjs.org`, `pypi.org`, `api.osv.dev`,
`doi.org`, `pub.orcid.org`), GET and POST, https only. A definite no from the resolver is E105; an outage, a rate
limit or a demand for credentials is the machine's (W181, W182, W172), never E105.

An index read whole can be large: a popular npm packument or PyPI project document passes the journal's 4 MiB record,
and a few pass the 16 MiB read. Neither a smaller document nor a declared `Accept` header serves every checker:

- npm's abbreviated document (`Accept: application/vnd.npm.install-v1+json`) lists the releases but states no licence,
  and PyPI's simple JSON API lists files and yanked marks but no classifier, so `license_of` would still ask the full
  per-release record;
- a header a capability declares is one more thing the grant must hold to its fixed hosts and the journal must digest,
  for a size the cap already bounds.

So the read stays whole and capped. An answer cut at the cap leaves its value unchecked (the judge answers
`Judged::Unchecked`, and the value counts toward I206 as `budget`), never W172: a large index says nothing of the
project, nor of the machine's network. An answer over 4 MiB is performed but not kept, so a `cached` run misses it.

## D16. The cap ladder, and JSON Schema left out

When a re-vendored blob crosses D8's cap, the cuts come in this order, re-measured after each: one regex engine (taken
before these families); the kit's magic-number table for `infer`; the kit's CommonMark heading scanner for
`pulldown-cmark`; and, last, JSON Schema. The first three cost no meaning. The last fired: JSON Schema through `boon`
alone would put the blob over the cap.

| Build (`[profile.blob]`, wasm32-wasip2) | Bytes |
|---|---|
| the blob before JSON Schema, structured family on the kit's scanner | 2,107,340 |
| + `boon` | 2,688,721 (+581,381) |
| + `boon` + `pulldown-cmark` (ladder step 3 not taken) | 2,849,261 |
| + `pulldown-cmark` alone | 2,269,321 (step 3 saves 161,981) |

`json`, `yaml` and `toml` check that a file parses. A schema is a pack's to bring, on the kit's tree.

## Rejected

- **A `pattern` checker.** The kernel keeps `matches` (ADR 0336 D10).
- **`regex-lite` beside the kernel's `regex`.** Two meanings of "a regex".
- **libphonenumber's metadata.** About 1 MB, and jurisdictional.
- **`RootedPath` and probe builders in the kit.** A second parser that could disagree with the host's, and a copy of the
  SDK's probes.
- **A per-checker severity.** ADR 0026 D3.
- **One extension per family.** A 428 KB floor each.
- **A `dns` capability and a Dns port.** The name is already the http capability's scope.
- **`infer` and `pulldown-cmark`.** A magic-number table and a heading scanner in the kit do their part here for
  about 160 KB less.
- **`lopdf`.** About 593 KB to count pages the cross-reference already locates.
- **JSON Schema through `boon`.** About 580 KB, over the cap ([D16](#d16-the-cap-ladder-and-json-schema-left-out)).
- **A `schema` URL loader, and registries beyond a cross-industry identifier's own resolver.** [D15](#d15-registries).

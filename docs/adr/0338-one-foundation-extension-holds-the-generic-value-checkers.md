# One foundation extension holds the generic value checkers

**Status:** accepted (2026-11-04). Builds on
[ADR 0336](0336-a-field-declares-the-checks-its-values-pass.md), and records its D8. **Amends 0336:** its probe set
(D9) gains `fs.digest{sha256}`. The host streams a file of any size under `Contained`, so a pin can hold a large
artifact.

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

- There is no `dns` capability and no Dns port: name resolution and TLS information are operations of the Http port, in
  the `links` scope.
- Each family has a bool setting, `families.<family>`, on by default. It works twice:
  - each checker carries `when: ["families.<family>"]`, so a family that is off is not called, sends no probe and asks
    for no scope;
  - it `gates` the codes only its family reports, so `specforge settings` shows what a switch silences.
- E103 is shared by the families that read a name, so no switch gates it, and `when` alone switches it.

## D4. Codes

The codes are bare, with owner `checks`, in the static catalog, and `@specforge/checks` declares no `codes` (ADR 0286
D2; ADR 0328 D2's rule is for a builtin outside `@specforge`). Another severity is another code (ADR 0026 D3), so no
checker takes a `severity` argument.

| Code | Family | Meaning |
|---|---|---|
| E016 | files | the file does not exist; on the value's span, with a sibling only the host found inside the root |
| E096 | files | a file's digest differs from its pin |
| W176 | files | a file's size, encoding or line range is not what the check asks |
| E091 | links | the URL is malformed, relative or carries credentials; it is never fetched |
| E092 | links | the scheme is not allowed, on the scheme; `javascript:`, `data:` and `file:` never are |
| E093 | links | the URL answered 404 or 410 |
| W183 | links | the same, on a field whose check says `status: "warn"` |
| W182 | links | 401 or 403: the page needs credentials, and whether it exists is not decided |
| W181 | links | 429, 5xx or another non-success |
| W172 | links | DNS, connect, TLS handshake or timeout failure, or a name that does not resolve |
| W173 | links | a redirect that leaves the grant, an https to http downgrade, more than 5 hops, or a final host outside `hosts` |
| E104 | links | a `Content-Type` outside `allow`, or none |
| E112 | links | the URL's fragment names no `id` or `name` in the page; on the fragment |
| W178 | links | the certificate's `not_after` is within `days` |
| E106 | git | a commit, ref or file at a revision is not found |
| W180 | git | a path git does not track |
| E103 | formats, digits, files, git | the value's text is malformed |

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

One engine, the workspace's full `regex`, the engine of the kernel's `matches`. The `regex` checker accepts exactly what
`matches` accepts, so "a regex" means one thing in the kernel and in the guest.

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

Measured: `@specforge/checks` with its five families and its collector is 2,149,827 B, 350,173 B under the cap; nearly
all of it is the one `regex` engine with its Unicode tables (formats) and the `url` parser with its IDNA tables (links).
The twelve builtins total 9,010,614 B, and `TOTAL_BUDGET` is 9,100,000.

## D9. Outcomes as facts

The `checks` collector declares the `check_outcomes` input and no detection files; `check_outcome` is the kind it
writes, in scheme `check`. A plain `collect` never runs it unless `checks.publish` is true (ADR 0336 D5). A fact's
`status` is `passed`, `failed` or `skipped`, the kind's result; `outcome` beside it holds the class. No outcome is ever
pending, so it sets no `complete_when`.

## Rejected

- **A `pattern` checker.** The kernel keeps `matches` (ADR 0336 D10).
- **`regex-lite` beside the kernel's `regex`.** Two meanings of "a regex".
- **libphonenumber's metadata.** About 1 MB, and jurisdictional.
- **`RootedPath` and probe builders in the kit.** A second parser that could disagree with the host's, and a copy of the
  SDK's probes.
- **A per-checker severity.** ADR 0026 D3.
- **One extension per family.** A 428 KB floor each.
- **A `dns` capability and a Dns port.** The name is already the http capability's scope.

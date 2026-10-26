# A spec root outside the project has one spelling

**Status:** accepted (2026-10-26, architecture round 27, plan 04). Amends [ADR 0242](0242-a-project-file-has-one-spelling-from-the-project-root.md) D3 (an outside key's directory is canonical, and the spec root's prefix is minted by the same rule) and reopens [ADR 0195](0195-a-spec-file-a-request-names-is-read-into-its-source-key-once.md) D3 (as 0242 amended it): outside is a key outside both roots, and a `..` is applied where it stands.

## Context

Round 26 keyed a source from the project root. A spec root outside the project became two spellings: the prefix kept a
linked spec root's own name (`/x/link`) while every key under it was minted with its directory canonical
(`/x/real/b.spec`). So `check` reported E025 on every import under a linked outside spec root, `watch` dropped an edited
file and missed a new one, and `new --in` refused every spelling of a source under any outside spec root, the exact
file the graph prints included ("leaves the project root").

```text
BEFORE  prefix /x/link   keys /x/real/b.spec   view: any `..` or absolute key -> not_a_source
AFTER   prefix /x/real   keys /x/real/b.spec   view: not_a_source only outside both roots
```

## Decision

- **D1. The spec root's prefix is the directory of the key a file directly in it gets** (`ProjectPath::key` of
  `dir/<name>`): one rule for the prefix and every key under it. Every key of a listed file starts with the prefix;
  `SourceRoot::path` opens any key under it as the walk spells the spec root.
- **D2. A request's named spec file is read from the project root into the key `SourceRoot::key` mints for it**, a
  `..` applied where it stands; `not_a_source` only when that key lies outside both the project root and the spec
  root; membership decides the rest (0195 D1 and D4 unchanged).
- **D3. A typed create may name a source under an outside spec root.** The CLI writes it (as `set` already does,
  [ADR 0092](0092-a-path-an-mcp-request-names-resolves-inside-the-served-project.md) D6); MCP's containment refuses
  the write (unchanged).

## Rejected

Keys keeping the link spelling (a second mapping inside `SourceRoot::key`, every outside span changing); a second
canonicalization beside `ProjectPath::key`; membership without an outside predicate (`../x.spec` would become
`file_not_found`).

## Consequences

No printed key changes. E025 is gone under a linked or `..`-linked spec root; watch keeps such a source. The outside
refusal reads "lies outside the project root and the spec root". No Graph Protocol bump, no re-vendor. Reopen it if a
spec root must be watched through a link whose target changes while served (the prefix is minted at load).

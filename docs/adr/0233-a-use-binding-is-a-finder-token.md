# A use binding is a finder token

**Status:** accepted (2026-10-23, architecture round 26, plan 04). Amended by [ADR 0274](0274-a-closing-comment-run-is-the-bodys.md): D3, the statement's doc comment goes with it. Extends [ADR 0163](0163-a-written-token-is-found-by-one-finder.md)
D1 and D3 and [ADR 0176](0176-the-finder-is-the-only-record-of-where-a-name-is-written.md) D1; answers the last
consequence of the round-22 amendment of [ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md);
extends [ADR 0055](0055-imports-name-sources.md) D2 (W027). Extended by [ADR 0284](0284-a-name-is-unique-within-its-scope.md) (a binding may be a path; resolution unchanged).

## Context

- The finder is the one record of where a name is written (ADR 0163, ADR 0176 D1), but it never read a selective
  `use { f1 } from "a.spec"` binding, which the LSP's cursor and semantic tokens already treat as naming `f1`.
- So a rename (MCP or LSP) left `use { f1 }` behind (a `pub use` then warned W027, a plain `use` stayed silent), the
  editor's references and prepareRename skipped the binding, `delete --unlink-references` left a binding naming
  nothing, and a plain `use { Nope }` naming nothing was never reported.
- Three callers of the finder answer a user's question and all three missed a binding: `Navigator::found`,
  rename's capture guard and a delete's referrer scan.

```text
BEFORE  use { f1 } -> LSP cursor only      rename, references, prepareRename, delete: miss it
AFTER   use { f1 } -> Holder::Use token    every finder question answers it; W027 for any selective import
```

## Decisions

- **D1.** The finder reads each selective `use` binding's imported name as a written token of the entity it names:
  `Holder::Use`, `Form::Binding`, held by no block, `field` the statement's path, spanned at
  `ImportBinding::name_span`. An alias is no token (it names nothing). A full or namespace import writes no name.
- **D2.** Every finder question answers it unchanged: rename's edits and its capture guard (a binding naming the new
  ID for no entity is a capture), the editor's references and prepareRename. MCP `find_references` lists holders and
  lists none for it. The cursor keeps its own binding question (ADR 0023 D5/D7: a stale buffer is never asked of
  navigation).
- **D3.** A delete with `unlink_references` removes a binding that imports the entity, its statement with it when it
  was the last: the first typed edit that writes a `use` statement (ADR 0163 "What would reopen it").
- **D4.** W027 is every selective import's, `pub` or not, placed on the binding; its title is "Import binding not
  found".

## Rejected

- A binding walk in ops: a second finder, ADR 0163's "third walk over parsed files".
- The graph holding import tokens: ADR 0163 rejected recording tokens no node holds.
- An `Option` holder on `Occurrence` so MCP lists the binding: ADR 0163 rejected it, and it is a schema change.
- Deleting the cursor's binding question: D2.
- An alias as a token: it names nothing a reader resolves.
- A new W163 for a plain `use`: two codes for one fact.
- Refusing the delete: the remedy would be a hand edit of a line no typed edit writes.

## Consequences

- MCP `specforge.rename` and LSP rename edit bindings; LSP references and prepareRename answer them; a rename onto a
  binding naming nothing is refused as a capture; `delete --unlink-references` removes bindings; W027's title, text,
  span and reach change.
- Rust API: `ImportBinding::{span, name_span}`, `Holder::Use`, `Form::Binding`, `Holder: Display`,
  `WrittenToken::place`. No schema, Graph Protocol or registry change; no blob input.

## What would reopen it

A binding that changes resolution (scoped imports, a `paths` key, ADR 0055's trigger), or a typed edit that writes a
`use` statement for another reason than a delete.

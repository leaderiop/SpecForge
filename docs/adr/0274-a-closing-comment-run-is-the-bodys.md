# A closing comment run is the body's

**Status:** accepted (2026-10-26, architecture round 28, plan 11). Amends [ADR 0248](0248-a-members-comment-lines-are-where-the-parse-read-them.md) D2 (which side of a closing comment run a new last member takes), extends [ADR 0202](0202-a-blocks-doc-comment-is-where-the-parse-read-it.md) D1/D2 (a `use` statement's doc comment, its removal) and amends [ADR 0233](0233-a-use-binding-is-a-finder-token.md) D3 (the statement's doc comment goes with it).

## Context

- `link --verify`, `set` of a field in a block that writes none, and `set field.entry` inserted a new last member at the closing brace's row, below the whole-line comments directly above it. The parse then reads that run as the new member's comment lines, so `unlink --verify` or `unset` deleted a comment the user wrote: add then remove was not the identity.
- `delete --unlink-references` removed a `use` statement from its own line, so the comment above it became the next block's doc comment: `migrate` lifted it into a title, and a later delete of that block removed it.
- The statement's removal copied the block delete's blank-line rule but not its doc-comment rule, and both copies read the final newline's empty last element as a blank line, so a delete could leave a file without its final newline.

```text
┌─ BEFORE ──────────────────────────────┐   ┌─ AFTER ───────────────────────────────┐
│   problem "p"                         │   │   problem "p"                         │
│   // TODO        <- the new member's  │   │   verify "x"   <- comments_above: none│
│   verify "x"                          │   │   // TODO      <- the body's          │
│ }   remove-verify -> TODO gone        │   │ }   remove-verify -> TODO stays       │
└───────────────────────────────────────┘   └───────────────────────────────────────┘
```

## Decisions

1. The whole-line comments directly above a body's closing brace (a block's or a block field's) are the body's. A new last member (field, entry, verify statement) goes above them (`Block::end_of_members`, over `Entity::comments_above`); no member's removal takes them.
2. A `use` statement's doc comment is the parse's (`ImportDeclaration::doc_span`, the block's rule, the format header never one). A delete that removes the statement removes it, as a deleted block's.
3. One rule removes whole lines with one blank line beside them (`block::cut`), and the final newline is no blank line.

Rejected: the run is the last member's (no coherent identity: removing the member takes the run, including one an edit just added); the run is the new member's (the code before this ADR); keeping an orphaned comment when its statement goes; a lexer read of the line above the statement (ADR 0248 D2); an `Entity::closing_comments` field (`comments_above(close)` already answers).

Breaking: `specforge-parser` `ImportDeclaration` gains the pub field `doc_span`. CLI `link --verify`, `set`, `set field.entry`, MCP `specforge.edit` and the editor's verify stub write a new last member above a closing comment; `delete --unlink-references` removes the comment above a statement it removes; a delete keeps the final newline. No schema, Graph Protocol, registry or blob change; no file on disk is rewritten.

Would reopen: a surface that shows a comment, or a typed edit that writes a `use` statement for another reason than a delete.

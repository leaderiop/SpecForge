# A member's comment lines are where the parse read them

**Status:** accepted (2026-10-26, architecture round 27, plan 06). Reopens [ADR 0202](0202-a-blocks-doc-comment-is-where-the-parse-read-it.md)
D5 by its "Reopen when" (a member's comment must be read from the parse); amends
[ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md) D2 (a second defect refusal beside
`undeclared_change`) and [ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md)'s Consequences (an unset removes
the comment lines the parse read).

## Context

- "Which lines above a member are its comment lines" was read from text, by `trim_start().starts_with("//")` on the
  line above, in two modules: the typed edit's `Block::with_comments_above` and the defaults hoist's `one_line_list`.
- A line inside a multi-line string that starts with `//` was taken for a comment. `unset`, `unlink`,
  `delete --unlink-references`, `unlink --verify` and the repeat cleanup cut the string with the member below it and
  refused the user's valid block as `block_unreadable`; `set` of an absent field wrote its new line inside a string
  (refused `undeclared_change`); `migrate` skipped a hoist it could make.
- A W142 found only after the edit changed the block was told to the user as their own unreadable block, as ADR 0187
  had once blamed the parser for a valid file.

## Decision

- D1. The parse records every own-line comment of a block's body, nested block fields' included
  (`Entity::body_comments`). A comment on the row where the item before it ends trails that item. `Entity::comments_above`
  answers the run above a member by the doc comment's rule, one function (`parse/doc.rs`).
- D2. A typed edit removes and inserts around that run; the defaults hoist leaves a field with one as written. No
  module reads a comment line from text.
- D3. A block an edit leaves unreadable is refused `edit_unreadable` (`CompilationFailed`), a defect of the edit;
  `block_unreadable` is what the first read finds. `create` keeps `block_unreadable`: its W142 can come from the
  file around the insertion point.

## Rejected

- The lexer (the edit and the hoist lex the block text): a second and third reader of which lines are comments.
- A span per `FieldEntry` and per statement: a second vector and a field on every entry literal for the same answers.
- `undeclared_change` for a W142: its data is a graph delta, and there is none.

## Consequences

- Breaking (Rust API, specforge-parser): `Entity::body_comments`, `Entity::comments_above`.
- A new refusal code: CLI `error[edit_unreadable]`; MCP shows it as `compilation_failed` until the MCP refusal
  shape (plan 10) says otherwise.
- No file on disk changes; a file whose edit was refused before can now be edited.

## Reopen when

A surface must show a member's comment (hover, export), or a comment between a field's key and its value must move
with it.

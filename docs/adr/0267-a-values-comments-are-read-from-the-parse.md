# A value's comments are read from the parse

**Status:** accepted (2026-10-26, architecture round 28, plan 04). Extends [ADR 0248](0248-a-members-comment-lines-are-where-the-parse-read-them.md) D1/D2 (the parse's own-line record gains the comments inside and after a member's value) and reopens [ADR 0202](0202-a-blocks-doc-comment-is-where-the-parse-read-it.md) D5 for what 0248 left out.

## Context

- `set` replaced a value's whole span, so a list's item comments and a block field's member comments went with it, silently, even for the value already written; `format` keeps the same statement as written ([ADR 0038](0038-one-lexer-and-one-cst-vocabulary.md) D5).
- `link` and `unlink` lexed the field's whole lines and refused any comment, a trailing one included, before asking whether the target was already there; `set` kept a trailing comment by lexing the text after the value, a second reader of a fact the parse had classified and dropped.
- The refusal named neither entity nor file, which a `delete --unlink-references` over many referrers cannot answer.

## Decisions

1. The parse records, per block, the comments inside a member's value (`Entity::value_comments`) and the comments that end the row of the item before them (`Entity::trailing_comments`), answered by `Entity::holds_comment(value)` and `Entity::trailing_comment(value)` in `parse/doc.rs`, beside `comments_above`. The record is on `Entity`, never on `FieldEntry`: the graph node holds the `FieldMap`, and a comment's position would reach every node and every equality.
2. Every whole-value rewrite of a typed edit goes through `Block::place`: a value holding a comment is kept as written when the new value reads the same as the graph stores both (`field_form::write` of `FieldForms::stored`, [ADR 0182](0182-the-graph-answers-what-a-blocks-field-holds.md) D4), else refused `not_editable` naming its field, entity and file (MCP carries `entity_id`).
3. A trailing comment is its member's and stays on its row for `set`, `link` and `unlink`; removing a member removes it.

Rejected: a span per `FieldEntry` (graph equality, as 0248); re-attaching each comment beside its item (a second layout engine beside the formatter's "kept as written"); writing the new value and dropping the comments with a notice; routing the defaults hoist's layout rule through the record (it subsumes the comment case and agrees with the parse).

Breaking: `specforge-parser` `Entity` gains `trailing_comments` and `value_comments`; `set` of a value holding a comment writes nothing or is refused; `link` and `unlink` of a list with a trailing comment succeed; a link to a target already there succeeds whatever comment the list holds; the refusal message changes. No file on disk changes.

Would reopen: a typed edit that must change a list item by item (an LSP code action linking in place), or a comment between a key and its value that must move.

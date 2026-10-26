# The formatter reads a value's text from the parse

**Status:** accepted (2026-10-26, architecture round 27, plan 05). Reopened by [ADR 0266](0266-a-source-line-break-has-one-reader.md): a raw CR LF was a second fact of a value format read apart from the parse. Reopens [ADR 0234](0234-a-values-spelling-is-the-spellers.md) D1 (`Respell` carries the spelling) and D3 (the formatter reads the parse's reading of the written value, as W159 does); amends [ADR 0147](0147-a-value-is-written-in-its-fields-one-form.md) D4 (check and format ask the rule of one reading of the value).

## Context

`check`, the graph build and W159 read a written value through the parser (a `"..."` with its escapes read, never
trimmed). The formatter read the same token a second way, from the CST's raw text with its quotes stripped and
`trim()`med. So `format`, which promises layout only, rewrote `status " draft"` to `status draft` and
`produces [" ev_one "]` to `produces [ev_one]`: two errors disappeared and an edge appeared, and every typed edit of
such a block was refused as `undeclared_change`. The same split left W159s that `format` never cleared (a bare `true`
list item on a kind that takes members, a one-word `"""..."""` on several lines) and made `format` write `rpn +5`,
a syntax error, for `rpn "+5"`, where W159 said `as rpn  5` and `set` wrote `rpn 5`. Three earlier commits each
taught the formatter one more case the parser already knew.

## Decisions

1. One reading of a value token: `specforge_parser::value_token(node, source) -> Option<(Written, String)>`. The
   parse's field values and list items are built from it; the formatter's rewrite reads it. A `"..."` is unescaped, a
   `"""..."""` dedented, `true` / `false` in a list are literals; nothing is otherwise trimmed.
2. A padded value is kept as written (`status " draft"` stays, E061 and E003 keep reporting it). Rejected: trimming in
   the parser, which knows no field types and would trim a contract's prose; reading a word field trimmed in the
   graph (stored values change, `id_span` with them, a typo is forgiven silently).
3. A respelled value carries its spelling: `Respell::spelling`, an integer as the number it reads to (`" 5"`, `"+5"`,
   `"007"` are `5`, `5`, `7`), else `spelled`. Format writes it, W159 prints it, `spell::value` writes the same.
4. Format rewrites a value exactly when `FieldForms::rewrite` names a rewrite of the parse's reading, except a multi-line
   text on a list field (format cannot tell where one item ends, ADR 0234 D4). W159 reads the same rule on the same
   reading, so "W159 iff format rewrites" holds by construction.
5. `rewrite` leaves the items of a list on a number, boolean or date field (E061 reports the list): a bare date is no
   list item and `"+3"` respelled `3` would change what the item stores. Found by the property test.

Rejected: the formatter laid out from the parse's entries (a second parse per format, matching CST rows to spans,
~250 lines for no gain over D1); a copy of `unescape` / `dedent` in the formatter. Would reopen: a second fact of a
value that format must read the way the parse does.

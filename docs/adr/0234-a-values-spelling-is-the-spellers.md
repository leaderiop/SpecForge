# A value's spelling is the speller's

**Status:** accepted (2026-10-23, architecture round 26, plan 05). Widens [ADR 0147](0147-a-value-is-written-in-its-fields-one-form.md), amends [ADR 0153](0153-a-value-is-spelled-by-one-module.md), extends [ADR 0064](0064-a-declared-field-type-decides-one-wire-form.md) and [ADR 0034](0034-one-field-type-read-from-the-declaration.md). Reopened by [ADR 0247](0247-the-formatter-reads-a-values-text-from-the-parse.md) (D1: `Respell` carries the spelling; D3: the formatter reads the parse's reading of the written value, as W159 does). Extended by [ADR 0283](0283-a-name-may-hold-a-dash-and-a-dot-where-a-name-is-expected.md).

## Context

Two tables answered "how is this value written": `spell::value` (the typed-edit writer) knew integers, booleans and
the list shape; `FieldForm::written` (what `format` and W159 read) knew only bare-or-quoted. So `set` wrote
`rpn 100` where `format` left `rpn "100"`, and `features f1` was in form for W159 and not for `set`. The builtin
dates (`decision.date`, the milestone and release dates) were `string` fields guarded by three regex rules
(I053, I086, I087) that accepted `2026-13-01`.

## Decisions

1. One table, `spell::form`; `spell::value` and `FieldForms::rewrite` read it. `FieldForm::written` is gone.
2. `rewrite` reads the value's place (`Place::Whole` / `Item`) and answers a `Respell { to, listed }`: a quoted
   number, boolean or calendar day is bare, a literal on a text field is quoted, a whole value on a list field is
   written `[...]`. Exceptions: a bare word on a members kind (ADR 0147 D5), a quoted scheme ref id on a
   `string_list`, a `block`, and a text a number/bool/date field refuses (E061 reports it first).
3. W159 reads the written entry (`written_fields`), the build's files, not the stored value; its message prints the
   whole value as the field writes it.
4. A multi-line text on a list field is W159 ("a list written as one text") and format leaves it: format cannot tell
   where one item ends.
5. W159's message writes the whole value (`as watchers [t1, t1]`).
6. A `date` field type: one calendar day, bare `YYYY-MM-DD`; a quoted day is read as the day and warned, anything
   else is E061. The five builtin date fields declare it and I053/I086/I087 retire into E061. Its JSON is a string
   (schema `format: date`); `date` is a type-language primitive too. Protocol 1.6.0.

Rejected: a written-shape memo on `Node` (breaks coercion's idempotence on incremental rebuilds); quoting the dates;
refusing a quoted day; a date crate in the graph.

Would reopen: a date-and-time or a partial date (`2026-03`) a field must hold.

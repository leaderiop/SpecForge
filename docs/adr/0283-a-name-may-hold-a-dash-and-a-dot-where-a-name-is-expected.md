# A bare value is one word its field's type reads and means, and a name may hold a dash and a dot

**Status:** accepted (2026-10-27, architecture round 30, plan K1). Amends [ADR 0147](0147-a-value-is-written-in-its-fields-one-form.md)
D1. Extends [ADR 0093](0093-a-fields-declared-type-decides-what-its-value-references.md),
[ADR 0234](0234-a-values-spelling-is-the-spellers.md), [ADR 0038](0038-one-lexer-and-one-cst-vocabulary.md) and
[ADR 0063](0063-one-reading-of-a-broken-document.md) and
[ADR 0282](0282-an-unknown-type-name-is-the-hosts-e004-and-a-project-has-one-spec-block.md) D1 (a type name written
whole may hold `-` and `.`). E013 and E014 are unchanged.

## Context

- Extensions declare every kind's fields with a type. The grammar still split values by token shape: `03` was an
  integer, `2026-10-26` a date, and `03-02`, `AUTH-01`, `v1.0` and `2-1` were E001.
- GSD names its entities `AUTH-01`, `03-02`, `03.1`, `999.1`, `v1.0`, `D-01`, `G-03-1`, `03`.
- The graph build already coerced a value by its declared type (ADR 0093). A reference field refused a bare number
  or date with E061.

```text
BEFORE  requirement AUTH-01 { }   E001       AFTER  entity AUTH-01
        parent 2-1                E001              the reference "2-1"
        points 2-1                E001              the integer 1 (written 2-1)
        due 2026-10-26            a date            a date (never 1990)
        type L = 01 | 2           "01", "2"         unchanged
        expr { a-b < 3 }          two conjuncts     unchanged
```

## Decisions

1. **A bare value is one word.** In a field value or a list item the grammar reads one word,
   `-?[A-Za-z0-9_]+([.-][A-Za-z0-9_]+)*`. The parser stays domain-free.
2. **A name is the same word without a leading `-`.** That covers an entity header and a selective `use` binding.
   Every other name stays `identifier`.
3. **The parser tags a word by shape and keeps its text.** The tag is canonical integer, `true`/`false`, date shape,
   or text. That tag is the reading where no type is declared.
4. **The declared type reads and means the word.**
   - A reference: the name as written.
   - An integer: the word as an integer difference, `-?N(-N)*`, evaluated in i64 (`2-1` is 1, `007` is 7).
   - A date: a calendar day.
   - A bool: `true` or `false`.
   - A string or valueless enum: text. An enum: a declared value.
   - An identifier: one word.
   - Anything the type cannot take is E061, which names the type and the operand. `.` in an integer is "no
     fraction".
5. **The formatter writes a word as written.** Hover shows the value beside it. Exports carry the value.
6. **Where no type exists, literals stay:** expressions, union variants, type expressions. A value that is one word
   holding `-` or `.` and that a derived field reads as a type is one type name (ADR 0282's E004 names it whole). Other arithmetic
   (`+ * / ( )`) stays in `expr { }`.
7. **One rule, three readers.** The grammar, the scanner's form-start read and the lexer read a word in the same
   slots. The lexer agrees with the grammar on the corpus and on every form a random edit leaves intact.
8. **Create and rename take a name.**

## Rejected

- A marked literal, or a derived `gsd_id` field.
- Telling names from numbers by token shape (a leading-zero rule, a new code, and still no `03` as a name).
- Widening `identifier`; a new `word` node kind; a `FieldValue::Word`.
- The parser reading declared types.
- The editor reading the tree.
- An integer field taking literals only (safer against typos, but it denies the user's rule).
- `+ * / ( )` in a value word: they collide with K2's `/` path, the `*` glob and the `()` type.
- Formatting an integer word to its value: it loses the author's text.

## Trade-offs accepted

- A typo such as `10-5` in an integer field is 5 without a diagnostic. Hover shows it.
- The same text means different things in fields of different types. Without the declaring extension it is text.

## Breaking

- Bare values that were E001 are now read by their field's type: `AUTH-01`, `2-1`, `1.5`, `[2026-10-26]`, `[-5]`, an
  overflow. An integer, date or bool field refuses them with E061; other fields take them as text.
- A bare number, date or `true`/`false` in a reference field is a reference.
- `extra 007` on an undeclared field is the text `"007"`.
- IDs such as `9lives` or `a.b` are legal.
- The CST has no `boolean` or `date_literal` node; `kind::{BOOLEAN, DATE_LITERAL}` are removed.
- No schema, Graph Protocol or blob change.

## Would reopen it

- K2's paths.
- A decimal field type.
- Arithmetic beyond `-` in a typed value.
- A union variant or enum value holding a dash.

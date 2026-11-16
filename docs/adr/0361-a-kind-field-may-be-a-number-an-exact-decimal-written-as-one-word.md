# A kind field may be a number, an exact decimal written as one word

**Status:** accepted (2026-11-14). Amends [ADR 0283](0283-a-name-may-hold-a-dash-and-a-dot-where-a-name-is-expected.md)
(the shape tag gains a canonical decimal) and [ADR 0305](0305-a-bool-setting-gates-codes-while-it-is-on-and-a-setting-may-be-a-number.md)
(D2's W021 clause and its `i64` bounds compared as `f64`). Takes extension protocol 2.2.0.

## Context

`FieldType::Number` existed for settings only (ADR 0305). A kind field or a block member of type `number` was W021
and dropped, because "the `.spec` grammar has no decimal literal". That reason was wrong. The grammar's bare word
(ADR 0283) already reads `1.5`, `-0.25` and `1.5e3` as one word, the lexer lexes it, and the parser keeps it as text.
What was missing is a reading: a type that reads the word as an exact number, carries it through every surface
without float drift, and compares it by value.

A number setting was "any JSON number" with `i64` bounds compared as `f64`. `settings set` stored any finite `f64`, so
it kept `0.30000000000000004`, which no `.spec` word, field text or bound reads back the same way, and a warning
threshold between 0 and 1 could not be bounded.

```text
BEFORE  measure m1 { ratio 0.75 }     W021, the field is dropped
        setting ratio 0.05..0.95      the bounds are i64: unwritable
        settings set k 0.1            stored 0.1, compared as f64

AFTER   measure m1 { ratio 0.75 }     the number 0.75; graph JSON 0.75, field text "0.75"
        ratio {minimum 0.05, maximum 0.95}   exact decimal bounds, E061 outside them
        settings set k 1.50           stored 1.5; more than 15 significant digits is refused
```

## Decisions

- **D1. The literal.** `specforge_protocol_types::Decimal::parse` reads `[-+]?D+(\.D+)?([eE][-+]?D+)?`, `D` an ASCII
  digit. It is the one reader of a decimal word: the graph's coercion, E061's remedy, the one-form table, the typed
  edits, `settings set` and the parser's tag all call it.

  | Written | Read | Why |
  |---|---|---|
  | `0.75`, `-0.25`, `100`, `1.5e3`, `2E-4` | 0.75, −0.25, 100, 1500, 0.0002 | a sign, a fraction, an exponent: what every JSON, Rust and JS reader takes |
  | `007.5`, `1.50`, `-0.0` | 7.5, 1.5, 0 | a leading and a trailing zero read as `007` reads 7 for an integer |
  | `"+1.5"`, `"1e+3"` (quoted, or edit text) | 1.5, 1000 | `+` is no bare-word character; W159 writes it bare |
  | `.5`, `5.`, `1_000`, `inf`, `NaN`, `0x10`, `1.2.3`, `1e` | E061 "'…' is no decimal number" | not decimals |
  | `0.1234567890123456` | E061 "a number keeps at most 15 significant digits" | beyond what binary64 round-trips |
  | `1e308`, `1e-308` | E061 "'…' is outside the range a number holds" | outside binary64's normal range |

  `2-1` is not a difference on a number field: that reading is the integer's alone (ADR 0283). `2026-10-26` on a
  number field is E061.
- **D2. The value.** `Decimal { coefficient: i64, exponent: i16 }`, normalized: no trailing zero digit in the
  coefficient, zero is `0e0`, no negative zero; `|coefficient| < 10^15`, the adjusted exponent in `−307..=307`.
  `Display` is the canonical text: plain notation, no exponent, no trailing fraction zero (`0.0002`, `1500`,
  `-2.25`); it equals what Rust's `{}` prints for the `f64` the decimal converts to.

  The cap of 15 significant digits is the design. Every decimal of at most 15 significant digits maps to a distinct
  binary64, and printing that binary64 shortest gives the decimal back (C's `DBL_DIG` is 15). Within the cap a
  binary64 is exact, injective and order-preserving, so the `.spec` word, the graph's JSON number, the field text a
  pass reads, the `f64` a guest or a JSON client reads and the JSON Schema bound all hold one value. I-JSON (RFC 7493
  §2.2) asks interoperable JSON numbers to stay within binary64 precision.
- **D3. Comparison by value.** Two decimals are equal when their values are (`1.5` = `1.50` = `15e-1`), and order by
  value. The written word is not part of the value: the source keeps it.
- **D4. Bounds are exact.** `FieldDescriptor` and `SettingDescriptor` keep `minimum`/`maximum` as JSON numbers
  (`Option<serde_json::Number>`, the JSON shape unchanged). The field form reads them by its type into
  `specforge_graph::Bounds`: an `integer` takes whole `i64` bounds, a `number` takes `Decimal` bounds (`0.05..0.95`).
  A bound its type cannot read is W021 and dropped: a fraction or a value beyond `i64` on an `integer`, more than 15
  significant digits on a `number`. A value is compared `Decimal` against `Decimal` or `i64` against `i64`, never
  through `f64`, on the word path (E061) and on the JSON path (settings, computed values, facts).
- **D5. The parser's tag.** A word that is a canonical decimal, `-?(0|[1-9][0-9]*)\.[0-9]*[1-9]` of at most 15
  significant digits, is tagged `FieldValue::Number`, as `42` is tagged an integer. Such a word is exactly the
  decimal's `Display`, so the tag loses nothing (ADR 0283 D3). Every other decimal-looking word (`1.50`, `03.1`,
  `1.5e3`) stays text, and a `number` field reads it. A number lexeme is `-?[0-9]+(\.[0-9]+)?` or a date, so `1.5` and
  `03.1` are `number` tokens and literal list items, as `10` and `03` are. A declared `reference` still reads `3.1` as
  a name; a `string`, `enum` or `identifier` field stores a tagged decimal's text.
- **D6. One rule everywhere.** Every `number` is a `Decimal`: a kind field, a block member, a setting, a computed
  value and a fact. What `Decimal::parse` or `Decimal::from_json` refuses is refused on each path, and the refusal
  names the cap.
- **D7. The wire.** The graph JSON holds a JSON number, an integer when the value is whole and an `i64` or `u64`
  holds it (`100`, never `100.0`), otherwise the binary64 whose shortest text is the decimal (`1e20`, `0.75`;
  `Decimal::to_json`; `from_json` is the way back). A JSON number a compute pass, the harness ledger or a setting
  gives a `number` is held in that one form. A pass's field text is the canonical text, and a number member of a
  typed block is a JSON number in the pass entity's `values`. The JSON Schema publishes
  `{"type":"number", minimum, maximum}`. The Graph Protocol's `FieldJson` allows a decimal number. DBML writes
  `decimal`. A guest reads a number field with `GraphNode::number` or `PassEntity::number`.
- **D8. Writing.** `specforge format` keeps the word as written (`1.50` stays `1.50`). W159 writes a quoted number bare
  as given (`"1.50"` → `1.50`), or in its canonical text when the given text cannot be a bare word (`"+1.5"` → `1.5`).
  `specforge new`, `specforge set` and edit plans write a text input as given once it parses, and a JSON number in its
  canonical text; a required number or integer field of a new entity is created as `0`, or as the bound nearest it
  when `0` lies outside its bounds (`0.05` for `0.05..0.95`). `settings set k 1.50` stores `1.5`.
- **D9. Declaring one.** The SDK's `FieldBuilder::number()` declares a number field or member, and its `minimum` and
  `maximum` take `impl Into<serde_json::Number>` (an integer, or a `Decimal`). The SDK prelude exports `Decimal`. The
  gsd kit's `num(name)` declares one, bounded by `.range(lo, hi)` (whole) or `.within("0.05", "0.95")` (decimal
  text, read when the kind is declared).
- **Protocol.** 2.2.0, a minor bump. A 2.1 host drops a kind field of type `number` with W021, so E088 refuses a 2.2
  guest there; a 2.1 guest loads on a 2.2 host.

The owner chose one 15-digit rule for every number (D6) over kind fields only, decimal bounds (D4) over `i64` bounds
for both types, and an SDK builder `number()` (D9).

## Rejected

- `f64` alone: it cannot say whether `0.1` was written, and any print that is not the shortest drifts.
- A decimal crate (`rust_decimal`, `bigdecimal`): a new dependency in every guest, and its 28 or more digits would
  still be cut to binary64 by the graph's JSON and by every JSON client.
- serde_json's `arbitrary_precision`: a workspace-wide feature that changes every `Number` in every crate and guest,
  while JS clients still read binary64.
- Keeping the written text in the value: the source already keeps it, and a graph value that differs for `1.5` and
  `1.50` breaks comparison by value.
- The integer's difference reading (`2-1`) for a number: a number is one decimal.
- `i64` bounds for a number: they cannot state `0.05..0.95`.
- Number settings keeping any finite `f64`: a stored value no word, field text or bound reads back alike.

## Trade-offs accepted

- The cap: a value needing more than 15 significant digits is refused, and the refusal names the cap.
- `+` is written only in quotes or edit text, since it is no bare-word character; W159 writes it bare.
- An exponent word (`1.5e3`) is read by a `number` field but never tagged: the tag keeps only words that are their
  canonical text.
- Bounds are inclusive only.

## Breaking

- A `number` field's or setting's `minimum`/`maximum` is an exact decimal; a fractional bound, or one beyond `i64`, on
  an `integer` is W021 and dropped. The SDK's `minimum`/`maximum` take `impl Into<serde_json::Number>`.
- A number setting, a computed value or a fact of more than 15 significant digits is refused.
- A kind field or block member of type `number` is registered and read as an exact decimal (it was W021 and dropped);
  its value is exported as a JSON number.
- A canonical decimal word (`1.5`) is tagged a number: on an undeclared field or an open block it is exported as a JSON
  number, not a string, and as a list item it is a literal, as `10` is.
- The Graph Protocol's `FieldJson` allows a decimal number. A quoted number on a number field is W159, and format
  writes it bare.
- `specforge settings set` stores a number setting as its decimal (`1.50` is stored `1.5`); a JSON number given to a
  number field or member is written in its plain decimal text.
- DBML writes a number column as `decimal` (it was `double`).
- Extension protocol 2.2.0: a guest built with this SDK is refused by a 2.1 host (E088).

## Would reopen it

- A field needing more than 15 significant digits: it would be a string with a checker.
- Arithmetic in `.spec` beyond the integer's difference.
- Exclusive bounds (`exclusiveMinimum`/`exclusiveMaximum`), the only way to state a half-open range.

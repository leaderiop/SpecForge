# A pass sees a list field as its items, and a writing call is told the time

**Status:** accepted (2026-10-28, kernel round 35, plan K13 and K14). Extends [ADR 0019](0019-one-entity-snapshot-per-compile.md)
(the snapshot a pass reads), [ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md) (list
field types and `values`), [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (writing calls,
`set_verdict`'s clock) and [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (a hook's input).
Takes extension protocol 1.11.0.

## Context

A pass receives a list field as its items joined by `", "`, which cannot be split back when an item itself holds the
joiner (a sentence of a success criterion does). A model that counts the criteria of a phase, or compares the files two
tasks name, needs the items. A writing command stamps what it records (when a summary started, when a blocker was
raised, when a checkpoint was answered), and its input carries only a date; only `set_verdict`'s `at` is the host's
clock, and that stays a date.

## Decision

- **D1. `values` carries every list field as its items.** A field whose value is a `string_list`, `reference_list` or
  `enum_list` is also in `values` as a JSON array of its items' texts (`["a, b", "c"]`); a written empty list is `[]`; a
  field not written has no key. One function feeds `PassEntity.values` and `ValidatorEntity.values`, so a pass and a
  validator get the same. The joined text stays in `fields`, so a 1.10 guest reads what it read. It is unconditional:
  `values` is a map and every reader looks a name up. A variant list, a type union and an expression group keep their
  text. A computed list value was already in `values`: now authored and computed agree.
- **D2. `now` is `YYYY-MM-DDTHH:MM:SSZ`**: RFC 3339, UTC, seconds, a literal `Z`; fixed width, so it sorts as text. The
  host takes the instant once, and `today` and `now` are its two renderings, so they never straddle midnight.
- **D3. Which inputs carry it.** A command (`CommandInput.now`), a hook (`HookInput.now`) and a writing collector
  (`CollectInput.now`, as `today`). Not a prompt (a rendered prompt is reproducible, as a render input's `today` is blank
  on purpose), not a pass (a pass is a function of the project), and `set_verdict`'s `at` stays a date (ADR 0289).
  An empty `now` is not written, so a guest and a golden of 1.10 are unchanged.

## Rejected

An opt-in for list items (a flag per pass, for a map whose readers look names up). `today` made a timestamp (every
reader of the date would change). A clock for passes (a check would no longer be a function of the project, and the
session would have to re-run it as time passes). An SDK helper to read a list (a guest reads `values[name]`).

## Consequences

`values` is larger for a project with list fields; a guest that iterates `values` meets keys it did not know. Three input
fields, each omitted while empty. A test that pins a command's input sees one new key.

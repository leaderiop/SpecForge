# A plan op's scalar is read by its field's type

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md). No protocol change.

## Context

A scalar plan op on an integer field went through `one()`, which refuses a JSON number or boolean, while a block member
was already converted. `gsd-git` and `/gsd-windows` wrote their counts as decimal text.

## Decision

- **D1.** An `Integer` field accepts a JSON number that is a whole number, written as its decimal text. A `Bool` field
  accepts a JSON boolean, written `true` or `false`. A fraction on an integer field is refused with "takes a whole
  number". Every other field type still refuses a number or boolean with today's message.

## Rejected

A protocol field-type tag on the op.

## Limits

A number given as a string still works.

## Consequences

None: this is the rule block members already follow.

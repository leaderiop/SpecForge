# A numeric segment finds its entity whatever its padding

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR 0284](0284-a-name-is-unique-within-its-scope.md). No protocol change.

## Context

`/gsd-plan-phase 3` could not find phase `03`.

## Decision

- **D1.** After an exact miss, `Graph::find` matches the last k segments of a path against the spelling's k segments; two numeric ids match when they normalize alike (upstream's `normalizePhaseName`: the integer part as a number, the letter and decimals exactly). One match is `One`; several `Ambiguous`.

## Rejected

Normalizing in the parser (it would change what a spec reference names).

## Limits

A reference in a spec's text is unchanged. A plan id `3-1` is not numeric and does not find `03-01`.

## Consequences

`find` widens only after an exact miss.

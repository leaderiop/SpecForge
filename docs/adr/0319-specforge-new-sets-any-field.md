# `specforge new` sets any field with `--field`

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). No protocol change.

## Context

A kind's field named like an option of `new` (`path`, `format`) had no flag, so an agent could not create an artifact through the CLI.

## Decision

- **D1.** `--field <name>=<value>` (repeatable) sets any field of the kind, read as `specforge set` reads it. A name that is not a field, or given by `--field` and its own flag, is `invalid_input`.

## Rejected

A `--` escape.

## Limits

None.

## Consequences

CLI only.

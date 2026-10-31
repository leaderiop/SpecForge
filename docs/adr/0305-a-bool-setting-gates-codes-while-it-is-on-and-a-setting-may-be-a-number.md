# A bool setting gates codes while it is on, and a setting may be a number

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Extends
[ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md). Takes extension protocol 1.12.0.

## Context

A setting gated a code only while it was off, so a warning that exists to say "this setting is off" (gsd/W514) could
not be built. There was no `number` setting type: gsd's context thresholds are integers while upstream accepts decimals.

## Decision

- **D1. `gates_when_on`.** `SettingDescriptor.gates_when_on: Vec<String>` (default, skipped when empty): local codes
  reported by no source while this bool setting is true. Checked like `gates`; a code in both lists is W021 and
  dropped from `gates_when_on`. The CLI and LSP list them beside `gates`.
- **D2. `FieldType::Number`.** A setting may be a number: any JSON number, within inclusive `i64` bounds compared as
  `f64`; `settings set` parses a finite `f64`. A kind field of type `number` is W021 (the `.spec` grammar has no
  decimal literal).
- **Protocol.** 1.12.0 (two optional additions). A 1.11 host ignores `gates_when_on` and drops a `number` setting (W019).

## Rejected

The pass reads the setting and stays silent itself (the host would not know the code is gated: listings, hover and
judged gating would be wrong); a `decimal` flag; `f64` bounds.

## Limits

Inclusive integer bounds refuse a warning threshold between 0 and 1 and a critical one between 99 and 100, which
upstream accepts.

## Consequences

Every exhaustive match on `FieldType` gains an arm.

# `collect` re-keys a test id as the test report does

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR 0284](0284-a-name-is-unique-within-its-scope.md). No protocol change.

## Context

The test report re-keys a short id to its entity (ADR 0284) but `collect` filed an inexact id under `unknown` (W115), so `gsd-junit` resolved `[gsd:<id>]` itself.

## Decision

- **D1.** `KnownEntities::resolve(id)` answers `Exact`, `Rekeyed`, `Ambiguous` or `Unknown`: an exact key; else the id with one leading `/` stripped; else the one key equal to the id or ending with `/{id}` at a segment boundary.
- **D2.** A `Rekeyed` result is filed under the full path. Two spellings of one entity merge, and a `(name, status)` already recorded by this collect is not added twice.
- **D3.** `Ambiguous` and `Unknown` keep W115; an ambiguous id adds ` (it names more than one entity: a, b)` (paths sorted, three at most then `, ...`).

## Rejected

A collector-side `find` export (every collector would redo it).

## Limits

A partial segment never matches.

## Consequences

A collector reports the id as it read it.

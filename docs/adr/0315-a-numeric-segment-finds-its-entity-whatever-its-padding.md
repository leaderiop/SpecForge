# A numeric segment finds its entity whatever its padding

**Status:** accepted (2026-10-31, kernel round K20b, plan 14). Extends [ADR
0284](0284-a-name-is-unique-within-its-scope.md). No protocol change.

## Context

`/gsd-plan-phase 3` could not find phase `03`.

## Decision

- **D1.** After an exact miss, `Graph::find` matches the last k segments of a path against the spelling's k segments;
  two numeric ids match when they normalize alike (upstream's `normalizePhaseName`: the integer part as a number, the
  letter and decimals exactly). One match is `One`; several `Ambiguous`. A rooted spelling (a leading `/`) is never read
  by suffix: it matches whole paths only (`/v1.0/3` finds `v1.0/03`, `/3` and `/01-01` find nothing unless the root
  holds them), as ADR 0284 says. A spelling with no numeric segment skips the scan.
- **D2.** The edit surface (`set`, `unset`, `link` and the typed edits) reads an entity through the same `Scopes::find`,
  so `specforge set 3 goal x` edits phase `03`. Pinned by
  `three_and_03_name_the_same_phase_on_the_prompt_query_and_edit_surfaces` (gsd), which also holds the prompt and query
  surfaces; the prompt surface owes its reading to this ADR's `find`, not to K20a.

## Rejected

Normalizing in the parser (it would change what a spec reference names).

## Limits

A reference in a spec's text is unchanged. The LSP's token colouring and go-to, and the quick-fix that offers to create
a missing block, use `find`: they take `3` for `03` where the linker reports E003. The kernel strips every leading zero
(`003` is `03`), a superset of upstream's `normalizePhaseName`, which only pads to two digits. A plan id `3-1` is not
numeric and does not find `03-01`.

## Consequences

`find` widens only after an exact miss.

# The formal pass reads the host's coverage verdict

**Status:** accepted (2026-10-26, architecture round 28, plan 06). Amends [ADR 0019](0019-one-entity-snapshot-per-compile.md) (the counting facts: the formal pass reads them too) and [ADR 0040](0040-formal-diagnostics-the-design-can-check.md) (I008, W035: what a coverage item is). Holds ADR 0004 D2-b and D2-f.

## Context

- The pass entity carries the three facts the host's `Standing` counts by (`testable`, `owes_none`, the obligations), pinned equal to `Standing::counts` over all 32 combinations. `@specforge/testing` and `@specforge/product` read it through `coverage::Entity::from`.
- `coverage_tracking` overwrote `testable` with "testable, or an invariant". On a project with `@specforge/formal` and no `@specforge/testing`, W035 named an invariant that `stats`, `analyze coverage`, `inspect` (`not_testable`) and W158 do not count, and I008 called it proven.
- W035's suggestion sent the user to `specforge analyze coverage`, which lists nothing for an item when no results are recorded (A015 needs results; A001 needs no obligation).
- No ADR named the override, so it survived every later change to the counting rule.

```text
BEFORE  host: invariant not testable   formal: invariant always an item   (W035/I008 name it)
AFTER   host and formal: the entity counts or it does not                 (one answer)
```

## Decisions

1. Every pass reads `PassEntity`'s `testable`, `owes_none` and obligations as the host sent them. A pass may fill what the host leaves to it (`risk`) and never widens which entities count.
2. A coverage item (W035, I008) is an entity that counts toward coverage. No kind is an item by kind alone.
3. W035's suggestion says what `analyze coverage` lists with and without recorded results: A001 for an item declaring no obligation, A015 once results are recorded.

Rejected: a `counts` field on the pass entity (a second statement of the rule on the wire, as ADR 0216 rejected for the status); `testable` meaning "counts" (breaks `Counting::status` and testing's `testable_exempt`); invariants testable by default (contradicts "nothing is testable by default" and W158, puts a software kind in the registry).

## Consequences

- `!` `specforge analyze` on a project with formal and without testing no longer reports W035 or I008 for entities of a kind nothing makes testable; `check` still says W158.
- The formal blob is re-vendored. No protocol, SDK or host change; no new diagnostic code.
- W035 and I008 are re-texted in the catalog and `docs/diagnostics.md`.

## What would reopen it

- A pass that needs a counting answer the three facts cannot give.

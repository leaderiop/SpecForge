# A module is dropped, and no kind is added to hold it

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D8 (package values wait for nothing: a module, its own fields and the references that name one are dropped) and
[ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md) D13 (its rule that Later is for
module references, and its three carry-over rows `term.module`, `deliverable.modules` and `milestone.modules`: the
references are dropped and wait for no `@code/core` package). Cites
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D7 (a collected fact is read-only).
Builds on [ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md) (the same rule,
applied to the software kinds). No protocol or SDK change comes with this decision.

## Context

On 2026-11-16 the owner decided that the self-spec moves to `@gsd/core` with the kinds gsd-core defines, that no kind
is added to port the old spec, and that what has no home is dropped with a recorded reason (ADR 0364 states the rule
in full).

The census, measured at `f61370cb9` over the old graph (`specforge export --format graph` of a scratch copy, binary
built with the test profile). The full command list is kept in `../SpecForge-planning/r4b/BASELINE.md`, a local
folder this repository does not track.

- **54 modules** are entities of the root spec, and **187** `depends_on` edges run from a module to a module (`jq`
  over the graph's edges, source kind `module`, label `depends_on`).
- **42 of the 54** titles equal the name of a package of the root workspace (`cargo metadata --no-deps` lists 44
  packages, compared with `comm -12` against the module titles). None equals a package of the `gsd/` workspace (3
  packages). **43** equal the title of a collected `package` fact (78 facts: the root, fixtures and extensions
  workspaces), the extra one being `specforge-javascript-workspace`. So the collectors already record the package of
  43 of the 54 modules.
- **12 modules** match no root-workspace package: `specforge-javascript-workspace`, `specforge-package-cargo-test`,
  `-checks`, `-formal`, `-governance`, `-javascript-node-test`, `-javascript-source`, `-product`, `-software`,
  `-testing`, `-vitest` and `specforge-provider-gh`.
- **11 rows** of the absorb table wait for a package (`later(Package)` in `xtask/src/absorb/table.rs`, counted by
  `grep -c`): eight of a module's own fields (`family`, `description`, `features`, `depends_on`, `reason`, `tags`,
  `ports`, `ports_defined`), and `deliverable.modules`, `milestone.modules` and `term.module`.
- **260 ledger entries** of the whole-spec trial wait for a package, one per entity and field: module `family` 54,
  `description` 54, `tags` 54, `features` 26, `depends_on` 44, deliverable `modules` 12 and milestone `modules` 16
  (`../SpecForge-planning/r4b/trial/ledger.json`, grouped by kind and field). Four fields held no value: `reason`,
  `ports`, `ports_defined` and `term.module`.

No home exists for these values. gsd-core defines no module kind. A `package` of `@code/core` is a collected fact,
and a collected fact is read-only: writing one in a `.spec` file is E115 (ADR 0327 D7). So nothing authored can hold a
module's family, description or dependencies.

## D1. No `module` kind is added

None in `@gsd/core`, none in `@code/core`.

## D2. The conversion drops the modules, their fields and their dependencies

The conversion drops the 54 modules, their own fields and their 187 `depends_on` edges, each ledgered with its reason.
The 11 package-pending rows become dropped rows in the conversion change.

## D3. The references that name a module are dropped with it

`deliverable.modules`, `milestone.modules`, `term.module` and a module's `features` are dropped with the module.
Nothing is inverted onto a requirement, a milestone or a term.

## Rejected

- **A `module` kind authored in `@gsd/core`, with an optional package reference.** This ADR first recorded it. It is a
  kind gsd-core does not define.
- **A `module` kind in `@code/core`, anchored to the package.** It is a kind added to port the old spec, and
  `@code/core` holds collected facts only.
- **Authored fields on the collected `package`.** ADR 0327 D7 forbids it: a fact is read-only.
- **Keeping modules in `@specforge/product`.** The root would load an old builtin, the milestone's goal reversed.

## Consequences

- The gates that read the `module` kind (`spec-modules`, `crate-map`) lose their input at the swap. The swap's change
  re-points each at collected `package` facts, or retires it with a recorded reason.
- The hand-written module dependencies leave the spec. The ledger, with the tagged pre-swap commit, is their record.
- `@gsd/core` and `@code/core` are unchanged.

## Would reopen it

- gsd-core defining a module-like kind.
- A gate that needs a fact about a package that no collector records.

## Review findings

A review of ADRs 0364 to 0369 on 2026-11-16 raised 25 findings: CR-01, WR-01 to WR-19 and IN-01 to IN-05. Each has one
row, in the ADR it concerns most; these are this ADR's. Phases are those of milestone v4.1: 2 the vocabulary change, 3
the review of `@gsd/core`'s kinds and fields, 5 the conversion, 6 the vendoring, 7 the swap.

| Finding | What it said | Outcome | Delivered in |
|---|---|---|---|
| WR-06 | A module's `features` was given both to the module and, inverted, to the requirement | Moot. D1 and D3: no module kind and no inversion, so `features` is stored nowhere twice | - |
| WR-07 | A module whose package fact is absent was refused, against the rule that an uncollected fact is a warning | Moot. D1: no package reference exists to refuse | - |
| WR-08 | `module` would be an eighth shared keyword, where ADRs 0366 and 0369 said seven | Moot. D1: `@gsd/core` declares no `module` keyword. Which keywords are shared is read from the staged declaration when the vocabulary is generated | phase 2 |
| IN-03 | This ADR overturned ADR 0357's "later: a `@code/core` package" without amending ADR 0357 | Fixed in the status paragraph, which amends ADR 0357 D13 and its three rows; closed by a later ADR | phase 3 |

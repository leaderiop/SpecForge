# A module is an authored @gsd/core kind that may name its @code/core package

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D8 (package values wait for an authored `module` kind of `@gsd/core`, not for `@code/core` packages). Cites
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D7 (a collected fact is read-only: a
`package` is collected, and writing one in a `.spec` file is E115) and
[ADR 0349](0349-an-anchor-is-a-graph-edge-between-an-entity-and-a-located-fact.md) (an anchor is a graph edge between
an entity and a located fact). Builds on
[ADR 0364](0364-the-software-kinds-become-authored-kinds-of-code-core-and-the-self-spec-converts-them.md) (a module is
not one of the software kinds: its home is `@gsd/core`, not `@code/core`). No protocol or SDK change comes with this
decision.

## Context

ADR 0360 D8 says package values (module references, a module's own fields) wait "for `@code/core` packages once modules
move there". A `package` of `@code/core` is a collected fact: a collector records it, no `.spec` file writes it, and
nothing authored can hold a module's family, description or dependencies. So the home D8 names cannot take the values.

The census, measured at `f61370cb9` over the old graph (`specforge export --format graph` of a scratch copy, binary
built with the test profile), each figure with its command in `../SpecForge-planning/r4b/BASELINE.md`:

- **54 modules** are entities of the root spec, and **187** `depends_on` edges run from a module to a module
  (`jq` over the graph's edges, source kind `module`). The research's 54 and 187 stand.
- **42 of the 54** titles equal the name of a package of the root workspace (`cargo metadata --no-deps` lists 44
  packages, compared with `comm -12` against the module titles). None equals a package of the `gsd/` workspace (3
  packages). 43 equal the title of a collected `package` fact (78 facts: the root, fixtures and extensions
  workspaces), the extra one being `specforge-javascript-workspace`. 12 modules match no root-workspace package:
  `specforge-javascript-workspace`, `specforge-package-cargo-test`, `-checks`, `-formal`, `-governance`,
  `-javascript-node-test`, `-javascript-source`, `-product`, `-software`, `-testing`, `-vitest` and
  `specforge-provider-gh`. The research's 42 of 54 stands.
- **11 rows** of the absorb table wait for a package (`later(Package)` in `xtask/src/absorb/table.rs`): eight of a
  module's own fields (`family`, `description`, `features`, `depends_on`, `reason`, `tags`, `ports`, `ports_defined`),
  and `deliverable.modules`, `milestone.modules` and `term.module`. The whole-spec trial ledgered 260
  entries (one per entity and field) against them: module `family` 54, `description` 54, `tags` 54, `features` 26,
  `depends_on` 44, deliverable `modules` 12 and milestone `modules` 16
  (`../SpecForge-planning/r4b/trial/ledger.json`, grouped by kind and field). `reason`, `ports`, `ports_defined`
  and `term.module` held no value.
- `@gsd/core` already declares `@code/core` an optional peer (`c.optional_peer("@code/core", ...)` in
  `gsd/core/src/lib.rs`), so a project may load one without the other.

## D1. `@gsd/core` declares an authored `module` kind

A `module` is an authored kind of `@gsd/core`, holding the old module's fields (its family, description, features and
dependencies on other modules) and an optional reference to the `@code/core` `package` fact it is delivered as.
`@code/core` does not change for it. The reference is an edge from an authored entity to a fact, as ADR 0349's anchor
is; the package fact stays read-only (ADR 0327 D7). The field-level design is the change that
lands the kind, with its own ADR.

## D2. A milestone, a term and a requirement name their modules

The references that wait for a package now name a module: `milestone.modules` (the old deliverable's and milestone's
`modules`, both land on the milestone), `term.module`, and `requirement.modules` (the old module's `features`, inverted
onto the requirement).

## D3. The converter links a module to the package of its title

absorb writes each module and links it to the package whose name equals its title (43 of 54 on collected package
facts). Every other module is mapped by hand or ledgered, none unaccounted, and every one of the 187 `depends_on` edges
between modules survives the conversion.

## D4. A module that names a package the project does not hold is refused

A module whose package reference names no `package` fact in the project is refused with a diagnostic, not kept dangling.
A module with no reference is valid: the reference is optional.

## Rejected

- **An authored `module` kind in `@code/core`, anchored to the package.** It forces a `@code/core` change and a
  re-vendor before `@gsd/core` is vendored, and it splits one planning concept (a module, a milestone's modules, a
  term's module) across two families.
- **Authored fields that enhance the collected `package`.** ADR 0327 D7 forbids it: a fact is read-only.
- **Keeping modules in `@specforge/product`.** The root would load an old builtin, the milestone's goal reversed.

## Consequences

- A project without `@code/core` still declares modules, because the package reference is optional.
- The gates that read modules (`spec-modules`, `crate-map`) are re-pointed at this kind at the swap.
- Whether a module's `depends_on` agrees with its package's manifest dependencies is not checked by this decision;
  the round's requirements list it as a future requirement, outside this milestone.
- The 11 table rows that wait for a package get homes when the kind lands, and the package pending (260 ledger entries
  in the trial) clears with them.

## Would reopen it

- `@code/core` gaining an authored module concept of its own.
- A project that needs the package reference to be required.

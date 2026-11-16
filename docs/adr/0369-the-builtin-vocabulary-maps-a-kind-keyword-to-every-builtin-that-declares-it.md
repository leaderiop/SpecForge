# The builtin vocabulary maps a kind keyword to every builtin that declares it

**Status:** accepted (2026-11-16). **Amends** [ADR 0122](0122-the-builtins-vocabulary-is-generated-beside-them.md) D1,
D4 and D5 (the generated vocabulary maps a kind keyword to the list of builtins that declare it, and `revendor` no
longer refuses a keyword two builtins declare; two builtins adding one field to one kind are still refused). Keeps
[ADR 0096](0096-an-enhancements-fields-are-credited-to-its-declarer.md) (an enhancement field is credited to its
declarer: a field two builtins add to one kind is still refused). Builds on
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) (the vendored `@gsd/core` row
needs it) and
[ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md)
(`@code/core`'s new kinds reuse keywords `@specforge/software` declares). No protocol change comes with this decision.

## Context

`crates/pipeline/registry/src/compilation/candidates.rs` builds the candidate vocabulary: `CandidateVocabulary::of`
inserts each kind keyword (the kind's `keyword`, else its `name`) with the extension that declares it, and returns Err
when a second extension declares the same keyword. `revendor` writes the result to
`crates/runtime/component/data/builtin-vocabulary.json` and stops on the Err. A spike that vendored `@gsd/core` as a
builtin (ADR 0366) met it at the step "builtin vocabulary", after the blobs were vendored and 304 snapshot files written,
and `revendor` printed:

```
revendor: @specforge/product and @gsd/core both declare the kind keyword 'persona'
```

The keyword is not the only one. Measured from the pinned declarations of the spike's `@gsd/core` against the fourteen
old builtins (the full recipe is `../SpecForge-planning/r4b/spike-recipe.md`), seven kind keywords are shared:
`decision` and `invariant` with `@specforge/governance`, and `deliverable`, `journey`, `milestone`, `persona` and `term`
with `@specforge/product`. Once ADR 0364 lands, `@code/core` declares `behavior`, `event`, `type` and `port`, which
`@specforge/software` declares too. A keyword two builtins share is the design, not a mistake: `@gsd/core` and
`@code/core` are the destination of the old kinds, and the old builtins stay while `examples/` and `integrations/rust/spec`
still load them and `absorb` reads the old project with them.

One environment still cannot hold two declarers of a keyword: a project that loads both gets E026, and the first
registration wins (ADR 0360 Context). That is not changed here. What the vocabulary answers is a different question,
"which builtin declares this keyword", asked by suggestions and by the unloaded-peer rule, and it has two answers.

The call sites that load several builtins at once, measured by `git grep` at `f61370cb9`
(`../SpecForge-planning/r4b/BASELINE.md`): 11 call sites of `load_builtins(` (nine in
`crates/runtime/component/tests/builtins.rs`, one helper in `crates/runtime/component/tests/product_blob.rs`, one in
`crates/app/ops/tests/operations/migrate.rs`) and, apart, 11 call sites of `load_builtins_for(`, which names a subset.
The tests that load every builtin into one runtime are those nine in `builtins.rs` plus the helper and the one in
`migrate.rs`: with `@gsd/core` among the builtins, a registry built from all of them would register `persona` twice.

## D1. The vocabulary maps each keyword to the sorted list of its declarers

`crates/runtime/component/data/builtin-vocabulary.json` holds, for each kind keyword, the sorted list of builtins that
declare it (`"persona": ["@gsd/core", "@specforge/product"]`), and `revendor` writes it with `@gsd/core` beside the old
builtins. `CandidateVocabulary::kinds` becomes a keyword to list, and the reader of the written form reads the list.

## D2. E024's suggestion names every builtin that declares the unknown keyword

E024 (an entity block uses a kind keyword no loaded extension declares) suggests installing the extension that
declares it; with two declarers it names both, each with its `specforge add` command, in the vocabulary's order. The
other readers of the same answer (the suggestion of W151 and of I210, and `UnloadedPeers::declare`, which asks whether
a declarer is an unloaded optional peer) follow the list: `UnloadedPeers` asks whether any declarer is one.

## D3. E026 is unchanged, and the tests that load builtins together load one family at a time

A project loads one declarer of a keyword (E026, the first registration wins). The tests that load builtins together
load one family at a time: the old `@specforge/*` builtins, or `@gsd/*` with its peers, and none of them reports E026.

## D4. Enhancement fields stay credited to one declarer

A field two builtins add to one kind is still refused at generation, as ADR 0122 D5 and ADR 0096 D2 say: the list is
for kind keywords only.

## Rejected

- **Deleting the old builtins in the same change as the `@gsd/core` row.** `examples/` and `integrations/rust/spec`
  still load them, and `absorb` reads the old project with them.
- **A precedence order where the first builtin wins.** It hides the second declarer from E024's suggestion, and ADR 0122
  rejected "first in load order wins" for a duplicate already.
- **Renaming gsd's keywords.** They are the destination names.
- **Making revendor skip a shared keyword.** The vocabulary would then leave out a keyword a project can be told to
  install, and E024 would suggest nothing for it.

## Consequences

- The vocabulary's committed form changes to a list per keyword, so its readers (the suggestion of E024, W151 and I210,
  and `UnloadedPeers`) change in the same commit, and the test that asserts two declarers are refused,
  `two_candidates_declaring_one_keyword_are_refused`, is rewritten to assert the list.
- The vocabulary change lands before `@gsd/core` is vendored (ADR 0366 D6): without it revendor refuses the row, and the
  vendoring change cannot be committed.
- A project that loads both families still gets E026, by design.
- The generated file grows: the spike's revendor added 60 lines for the `@gsd/core` kinds no old builtin declares, and
  each of the shared keywords gains a second name.

## Would reopen it

A project that must load two declarers of one keyword at once (namespaced keywords).

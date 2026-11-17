# The builtin vocabulary maps a kind keyword to every builtin that declares it

**Status:** accepted (2026-11-16). **Amends** [ADR 0122](0122-the-builtins-vocabulary-is-generated-beside-them.md) D1,
D4 and D5 (the generated vocabulary maps a kind keyword to the list of builtins that declare it, and `revendor` no
longer refuses a keyword two builtins declare; two builtins adding one field to one kind are still refused, which is
D5's rule). Keeps [ADR 0096](0096-an-enhancements-fields-are-credited-to-its-declarer.md) (an enhancement field is
credited to its declarer). Builds on
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) (the vendored `@gsd/core` row
needs it). No protocol change comes with this decision. D2 amended by
[ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D4: a suggestion offers the
declarers that collide with nothing loaded.

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

The keyword was not the only one. Measured from the pinned declarations of the spike's `@gsd/core` against the
fourteen old builtins (the full recipe is `../SpecForge-planning/r4b/spike-recipe.md`), the spike's `@gsd/core` shared
seven kind keywords with them: `decision` and `invariant` with `@specforge/governance`, and `deliverable`, `journey`,
`milestone`, `persona` and `term` with `@specforge/product`.

`@code/core` gains no kind
([ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md)), so it shares no
keyword. `@gsd/core` shares keywords with the old builtins because both name the same things, and the old builtins
stay while tracked projects load them
([ADR 0367](0367-the-self-spec-sample-retires-at-the-swap-and-absorb-stays-while-the-old-builtins-ship.md)) and
`absorb` reads an old project with them. A keyword two builtins share is the design, not a mistake. The set of shared
keywords is read from the staged declaration when the vocabulary is generated, never copied from the spike's seven. A
later ADR that changes `@gsd/core`'s kinds changes that set, not this rule.

One environment still cannot hold two declarers of a keyword: a project that loads both gets E026, and the first
registration wins (ADR 0360 Context). That is not changed here. What the vocabulary answers is a different question,
"which builtin declares this keyword", asked by suggestions, by the unloaded-peer rule and by the graph build (D2),
and it has more than one answer.

The call sites that load several builtins at once, measured by `git grep` at `f61370cb9`
(`../SpecForge-planning/r4b/BASELINE.md`): 11 call sites of `load_builtins(` (nine in
`crates/runtime/component/tests/builtins.rs`, one helper in `crates/runtime/component/tests/product_blob.rs`, one in
`crates/app/ops/tests/operations/migrate.rs`) and, apart, 11 call sites of `load_builtins_for(`, which names a subset.
The tests that load every builtin into one runtime are those nine in `builtins.rs` plus the helper and the one in
`migrate.rs`: with the spike's `@gsd/core` among the builtins, a registry built from all of them would have registered
the `persona` keyword twice.

## D1. The vocabulary maps each keyword to the sorted list of its declarers

`crates/runtime/component/data/builtin-vocabulary.json` holds, for each kind keyword, the sorted list of builtins that
declare it (`"decision": ["@gsd/core", "@specforge/governance"]`), and `revendor` writes it with `@gsd/core` beside the
old builtins. `CandidateVocabulary::kinds` becomes a keyword to list, and the reader of the written form reads the list.

## D2. E024's suggestion names every builtin that declares the unknown keyword

E024 (an entity block uses a kind keyword no loaded extension declares) suggests installing the extension that
declares it; with two declarers it names both, each with its `specforge add` command, in the vocabulary's order.

Every other reader of the single-owner answer follows the list. With two declarers each shows every one, and none
takes the first silently:

- **W151** (entities are written, and no loaded extension declares a kind) reads `kind_owner` directly, not the remedy
  E024 uses (`crates/pipeline/registry/src/checks/kinds.rs`). Its suggestion names every declarer of the kind it
  suggests for.
- **I210** (a route skips a kind that is not loaded) takes the remedy E024 takes, so it names every declarer too.
- **`UnloadedPeers::declare`** asks whether the declarer of a keyword is an unloaded optional peer. It asks whether any
  declarer is one.
- **I004 and E022** read `GraphInputs.target_owners`, a map from a kind to one extension in
  `crates/pipeline/graph/src/build.rs`. `kind_owner` feeds it from two call sites of
  `crates/pipeline/registry/src/compilation/build.rs`. The map holds every declarer of a kind, and both diagnostics
  name each extension that would provide it. The graph crate's public type changes with it, from one owner to a list.

## D3. E026 is unchanged, and the tests that load builtins together load one family at a time

A project loads one declarer of a keyword (E026, the first registration wins). The rule for the builtins follows: two
builtins that declare one keyword never load into one registry. A family is one of two sets, defined by name:

- **The old set** is the fourteen builtins of today: `@specforge/product`, `@specforge/software`,
  `@specforge/governance`, `@specforge/formal`, `@specforge/testing`, `@specforge/cargo-test`, `@specforge/vitest`,
  `@code/core`, `@javascript/node-test`, `@specforge/checks`, `@rust/cargo`, `@rust/source`, `@rust/test` and
  `@javascript/source`.
- **The gsd set** is `@gsd/core` with its peers, `@specforge/checks` (required) and `@code/core` (optional), and the
  collectors that need `@code/core`: `@rust/cargo`, `@rust/source`, `@rust/test`, `@javascript/source` and
  `@javascript/node-test`.

So `@gsd/core` is in the gsd set only. `@specforge/product`, `@specforge/governance`, `@specforge/software`,
`@specforge/formal`, `@specforge/testing`, `@specforge/cargo-test` and `@specforge/vitest` are in the old set only
(the last two need `@specforge/testing`). The other seven are in both: `@code/core`, `@specforge/checks`, the three
`@rust/*` collectors and the two `@javascript/*` collectors. The peers are read from the `peer` and `optional_peer`
calls of each builtin's `src/lib.rs` and of `gsd/core/src/lib.rs`, at `f61370cb9`. The seven keywords the spike's
`@gsd/core` shared were all shared with `@specforge/governance` or `@specforge/product`, which are in the old set
only, so neither set held two declarers of one keyword.

Each of the 11 `load_builtins(` call sites loads one set, or states why it loads all. E026 arises when one registry is
built from two declarers, not when components are loaded into a runtime, so a site that only loads components may load
all, and says so. Moving the sites is the vocabulary change's work. No test that builds a registry from one set reports
E026.

## D4. Enhancement fields stay credited to one declarer

A field two builtins add to one kind is still refused at generation, as ADR 0122 D5 says, and an enhancement field
stays credited to its one declarer (ADR 0096 D2): the list is for kind keywords only.

## Rejected

- **Deleting the old builtins in the same change as the `@gsd/core` row.** `examples/`, `integrations/rust/spec` and
  the test fixtures still load them (ADR 0367), and `absorb` reads an old project with them.
- **A precedence order where the first builtin wins.** It hides the second declarer from E024's suggestion, and ADR 0122
  rejected "first in load order wins" for a duplicate already.
- **Renaming gsd's keywords.** They are the destination names.
- **Making revendor skip a shared keyword.** The vocabulary would then leave out a keyword a project can be told to
  install, and E024 would suggest nothing for it.

## Consequences

- The vocabulary's committed form changes to a list per keyword, so its readers change in the same commit: the
  suggestions of E024, W151 and I210, `UnloadedPeers::declare`, and I004 and E022 through
  `GraphInputs.target_owners`. That map's type in the graph crate (`specforge-graph`) changes from one owner for a kind
  to a list, so the crate's public type changes. The test that asserts two declarers are refused,
  `two_candidates_declaring_one_keyword_are_refused`, is rewritten to assert the list.
- The vocabulary change lands before `@gsd/core` is vendored (ADR 0366 D6): without it revendor refuses the row, and the
  vendoring change cannot be committed.
- A project that loads both families still gets E026, by design. No project in this repository does: the root loads no
  `@gsd/core` before the swap, and after it none of the builtins that are in the old set only (ADR 0364 D5, ADR 0367
  D3).
- The generated file grows: each `@gsd/core` kind no old builtin declares adds an entry, and each of the shared
  keywords gains a second name.

## Would reopen it

A project that must load two declarers of one keyword at once (namespaced keywords).

## Review findings

A review of ADRs 0364 to 0369 on 2026-11-16 raised 25 findings: CR-01, WR-01 to WR-19 and IN-01 to IN-05. Each has one
row, in the ADR it concerns most; these are this ADR's. Phases are those of milestone v4.1: 2 the vocabulary change, 3
the review of `@gsd/core`'s kinds and fields, 5 the conversion, 6 the vendoring, 7 the swap.

| Finding | What it said | Outcome | Delivered in |
|---|---|---|---|
| WR-09 | D3 relied on two families that the builtins do not divide into, and did not say which call sites load which | Fixed in D3: both sets are listed by name, each builtin in the old set, the gsd set or both, with the rule for the 11 `load_builtins(` call sites | phase 2 |
| WR-10 | The list of readers of the single-owner answer was incomplete | Fixed in D2 and Consequences: every reader is named, I004 and E022 through `GraphInputs.target_owners` among them, and the graph crate's type changes | phase 2 |
| IN-04 | The status paragraph gave ADR 0096 the refusal of a field two builtins add to one kind, and Consequences quoted a line count measured on the single-owner form | Fixed in the status paragraph (the refusal is ADR 0122 D5's) and in Consequences (the line-count figure is gone) | this ADR |

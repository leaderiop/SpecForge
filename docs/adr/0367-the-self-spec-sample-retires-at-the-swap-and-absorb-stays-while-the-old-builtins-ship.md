# The self-spec sample retires at the swap, and absorb stays while the old builtins ship

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D1 (its last sentence: absorb is "deleted with the old builtins once the root project has moved"; the root project's
move deletes neither, and the old builtins outlive the swap) and D14 to D17 (the `self-spec-sample` check, its goldens
and its CI step retire at the swap, not later). Builds on
[ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md) (absorb
covers the software kinds, and the swap's evidence is a whole-spec run) and
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) (the vendored blob removes the
sample's gsd build). No protocol or SDK change comes with this decision.

## Context

ADR 0360 D1 ends: "It is a one-time migration, deleted with the old builtins once the root project has moved." Read
with the sample (D14 to D17) it says two things that do not hold together. The root project's swap (its spec in
`@gsd/core` and `@code/core` kinds, ADR 0360 D9) does not delete the old builtins. They stay in `BUILTINS` because six
projects still load them: the five examples `examples/product-launch`, `examples/regulated-process`, `examples/shop`,
`examples/spec-driven-method` and `examples/todo-app`, and `integrations/rust/spec` (each `specforge.json` names
`@specforge/*`; `git grep -l '@specforge/' -- 'examples/*/specforge.json' integrations/rust/spec/specforge.json` lists
exactly those six at `f61370cb9`). So "deleted once the root has moved" has no moment at which it becomes true.

The sample has a second problem. `cargo xtask absorb` compiles its source with the source's own builtins
(`xtask/src/absorb/read.rs`), and the sample copies the tracked tree and absorbs `spec/` from it
(`gate/src/self_spec_sample/steps.rs`). After the swap `spec/` is written in gsd kinds, so the sample would feed a gsd
project to a converter that reads old kinds: it would read a project already converted as an old one, and it would
convert nothing it did not hold already.

What the sample costs, and what replaces its evidence:

- The build job's `self-spec-sample` step took 432 s in the cold run 43828995410 (job wall time 1,319 s) and 339 s in
  the cold main-push run 43850086577 (job wall time 1,168 s); ADR 0360 measured 391 s for the check alone on a warm
  local target, with 41 cited tests (`../SpecForge-planning/r4b/BASELINE.md`, `gh run view <id> --log --job <job>`,
  the line `cargo gate: self-spec-sample passed in <n>s`).
- A whole-spec `absorb` over the root spec at `f61370cb9` took 66.47 s on a warm compile cache, exit 0, writing 841
  creates and a ledger of 1,758 entries, and the converted project checked in 11.35 s (the full record is in ADR 0364).
  The swap's own evidence is that run, repeated at the swap with the root's own extensions, not a sample of a spec that
  has already moved.

## D1. `self-spec-sample` retires in the swap change

The check, its `ci.yml` step, its goldens under `gate/golden/self-spec-sample/` and what documents it are removed in
the swap change, so `cargo gate list` still matches `ci.yml`'s shard list (the `sharded=` list and the build job's
`cargo gate ci --only self-spec-sample` step both lose the name). Until then the sample runs as ADR 0360 D14 to D17
say: it checks the sample the old `spec/` yields, which is what it was built to do.

## D2. `cargo xtask absorb` stays whole while any old builtin ships

`cargo xtask absorb` stays with all its parts, `--only sample:<name>` and the `select::SAMPLES` rule included, with its
unit tests, while any old `@specforge/*` builtin ships. It is deleted with the old builtins, in a later move that
removes the last project loading them. The sample rule stays with the command because it is a value form of `--only`,
and a reviewer reproducing a conversion on a part of a project needs it; only the gate's check around it goes.

## D3. The old builtins stay in `BUILTINS` after the swap

At the swap the root stops loading them: `specforge.json` and the `spec` block of `spec/specforge.spec` no longer name
`@specforge/product`, `@specforge/governance`, `@specforge/software`, `@specforge/testing`, `@specforge/formal` or
`@specforge/cargo-test`. Nothing else changes for them: the blobs, the vocabulary row of each (ADR 0369) and the
examples that load them stay as they are.

## D4. The swap's evidence is its whole-spec run

The swap is judged by a whole-spec `absorb` of the root, with `--to` and `--ledger` outside the repository, and by the
converted project checking clean. The ledger is archived outside the repository, never at the root, and the swap's
description quotes its counts by class against the baseline of ADR 0364. No sample, no golden and no fixture project
holds a second copy of what the swap converts.

## Rejected

- **A frozen pre-swap copy as a fixture project.** It is a project `specs` and `spec-format` would judge forever, a
  second old tree to keep green with the old builtins, and the sample would keep converting a copy that no longer moves.
- **Keeping the sample against a gsd `spec/`.** Absorb compiles its source with the source's own builtins, so it would
  compile a gsd project as an old one (Context), and the check would pass on a conversion that reads nothing.
- **Deleting `cargo xtask absorb` at the swap, as ADR 0360 D1 reads.** The examples and `integrations/rust/spec` are
  old projects the command still reads, and ADR 0364's residual census test reads the old kinds with it.

## Consequences

- The CI build job loses the sample step's time: 339 s of the 1,168 s cold main-push run (432 s of the 1,319 s pull
  request run). The `gsd` shard keeps its own `gsd-core` wasm build until ADR 0366's vendoring change lands, and then
  loses it too.
- A `spec/` edit no longer pays the sample. A spec edit that moves what absorb reads (an old project the old builtins
  still hold) is covered by absorb's own tests, as before the sample existed.
- `absorb` stays a repository task with tests until the old builtins go; ADR 0360's table and its measure test are not
  edited by this decision.
- The swap's archived ledger is what a reviewer reads, as ADR 0360 says of the ledger in general.
- ADR 0360 D1's last sentence is settled by this ADR; D14 to D17 stop applying at the swap. ADR 0360 is not edited: a
  later change reads the pair.

## Would reopen it

The old builtins being deleted (absorb goes with them, and the examples would have moved first), or a second old
project to convert, which would need its own table rows and a check of its own.

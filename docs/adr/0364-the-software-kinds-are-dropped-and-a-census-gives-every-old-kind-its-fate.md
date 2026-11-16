# The software kinds are dropped, and a census gives every old kind its fate

**Status:** accepted (2026-11-16). Amended by
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) (its D3 and D8): the census rows left to a later
ADR, and D3's fields a milestone keeps, are decided there. **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D8 (behavior values and release values wait for no kind: the first are dropped, and a release becomes a version
milestone) and D2 ("No row drops a value" stops holding: a value with no home in a kind gsd-core defines is dropped
and ledgered with its reason, and absorb reads the four software kinds too). Builds on
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) and
[ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) (`@code/core` holds collected facts, and gains no kind).
No protocol or SDK change comes with this decision.

## Context

**The rule.** On 2026-11-16 the owner decided how the self-spec moves to `@gsd/core`: with entities, fields and
formats exactly as gsd-core defines them (gsd-core is the upstream definition that `@gsd/core` tracks). SpecForge adds
only computed values, collected facts, graph logic and guards. No kind is added to port the old spec, and what has no
home is dropped with a recorded reason.

SpecForge's own spec is written in the kinds of the old builtins. The four kinds of `@specforge/software` (`behavior`,
`event`, `type` and `port`) are most of it, and gsd-core defines none of them. The graph still sees the code they
described: the collectors `@rust/cargo` and `@rust/source` already record it as `@code/core` facts, 40,286 `item` and
78 `package` facts on the scratch copy of `f61370cb9` described below.

Every figure here was measured at `f61370cb9` and is quoted with its command or its run. The full command list is
kept in `../SpecForge-planning/r4b/BASELINE.md`, a local folder this repository does not track.

**The census of the root spec.** Entities whose file is under `spec/` in the old graph (`specforge export --format
graph` of a scratch copy of `f61370cb9`, binary built with the test profile), counted with
`jq '[.nodes[]|select(.file|startswith("spec/"))|select(.kind=="<kind>")]|length'`. The third column says what absorb
(ADR 0360) did with the kind in the whole-spec trial below:

| Kind | Entities | At `f61370cb9` | Fate | Decided by |
|---|---|---|---|---|
| `behavior` | 706 | not read by absorb | dropped | this ADR (D1, D2) |
| `event` | 123 | not read by absorb | dropped | this ADR (D1, D2) |
| `type` | 678 | not read by absorb | dropped | this ADR (D1, D2) |
| `port` | 26 | not read by absorb | dropped | this ADR (D1, D2) |
| `module` | 54 | read, no entity created | dropped | [ADR 0365](0365-a-module-is-dropped-and-no-kind-is-added-to-hold-it.md) |
| `release` | 3 | read, no entity created | becomes a version milestone | this ADR (D3) |
| `milestone` | 18 | converted | becomes a phase under a version milestone | this ADR (D3) |
| `feature` | 141 | converted | becomes a requirement | ADR 0360 |
| `constraint` | 63 | converted | becomes a requirement | ADR 0360 |
| `deliverable` | 12 | converted | a deliverable, where gsd-core's kind takes it | the conversion change |
| `decision` | 117 | converted | a decision when still in force, otherwise dropped with `docs/adr/` as its record | decided in a later ADR |
| `invariant` | 149 | converted | becomes a requirement | decided in a later ADR |
| `persona` | 6 | converted | dropped | decided in a later ADR |
| `journey` | 54 | converted | dropped | decided in a later ADR |
| `term` | 100 | converted | dropped | decided in a later ADR |
| `failure_mode` | 35 | converted | dropped | decided in a later ADR |
| `channel` | 5 | converted as a term ([ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md) D2) | dropped, as `term` is | decided in a later ADR |

The sums. At the baseline 700 entities of eleven kinds were converted. The 54 modules and the 3 releases created no
entity, so the 757 entities of the 13 kinds absorb reads is not the converted count. The four software kinds hold
1,533. With the one `spec` block the spec holds 2,291.

By fate: 1,587 are dropped by this ADR and ADR 0365 (1,533 and 54), 21 become version milestones and phases (3 and
18), 682 follow the conversion and the later ADRs, and 1 is the `spec` block. That is 2,291, with 0 unaccounted.

**What leaves with the four kinds**, same graph and expression family:

| Obligation | Count | Where |
|---|---|---|
| `verify` statements, all spec entities | 7,001 | `behavior` 5,632, `type` 617, `invariant` 379, `event` 174, `constraint` 139, `failure_mode` 34, `port` 26 |
| `requires` entries | 502 | on 358 behaviors |
| `ensures` entries | 2,261 | on 602 behaviors |
| `maintains` entries | 26 | `behavior` 12, `type` 14 |
| limit links (`constrains` edges) | 388 | constraint to behavior |
| lines holding `behavior = ` in tracked `*.rs` | 5,845 | in 900 files, by `git grep` at `f61370cb9` (textual) |
| strings `verify = "` in tracked `*.rs` | 6,168 | in 933 files, by `git grep` at `f61370cb9` (textual) |

The four software kinds hold 6,449 of the 7,001 `verify` statements.

**The whole-spec trial.** `absorb` ran over the whole `spec/` of a scratch copy (`git archive f61370cb9`), with
`--extension @rust/cargo`, `@rust/source` and `@rust/test`, from binaries built with the test profile (`cargo build
--profile test -p specforge-cli -p xtask`, 82 s, fresh) and `gsd_core.wasm` built in `gsd/` with the release profile
(5,663,652 B). Nothing was collected or absorbed in the checkout.

- The first run, spec unmodified, stopped before any write: exit 2 after 5.87 s, `term 'blocker'` refused as a reserved
  word (E013), the only id refused. The rename (`blocker` to `blocker_term`, in `terms.spec` of the scratch copy
  only) is a typed edit for a later change; the checkout's `spec/` is untouched.
- The second run, on a warm compile cache from the first: exit 0, outcome WRITTEN, 66.47 s wall, 931,960 kB peak
  resident (`/usr/bin/time -v`), 700 entities converted, 141 created, 841 creates in the plan, the target compiled with
  0 errors and 304 `code/W102` findings (its cited tests are not collected there). `specforge check` of the converted
  target took 11.35 s warm, exit 0.
- The ledger holds 1,758 entries: Later 536, filled 384, collapsed 224, computed 208, unproven 197, created 141,
  unconverted 60, target_later 8, out_of_selection 0 (a whole-spec run selects everything). Later waits for a package
  (260), a truth (182), a behavior (63) and a release (31). The 60 unconverted are 52 constraint `metric` lines that
  are not one bound, and 8 scaffold entries (the target's own `spec` block and `specforge.json`). The 197 unproven
  are all `invariant.verify`, each with an empty `tests` list. The 8 target_later entries are `failure_mode` values
  whose target behavior does not exist yet.

The run converted 700 of the 2,291 entities. A dropped entity is a ledger entry, not a create, so the plan does not
grow by the four kinds.

**Why the swap cannot leave them in `spec/`.** A swap that unloads `@specforge/software` with these entities still
written fails E024 (unknown entity kind) on each of the 1,533. Keeping it loaded keeps the root on an old builtin,
which is the milestone's goal reversed.

## D1. No software kind is added

No `behavior`, `event`, `type` or `port` kind is added to `@code/core` or to `@gsd/core`. `@code/core` keeps holding
collected facts only (ADR 0327, ADR 0329), and it is not re-vendored for this decision.

## D2. The conversion drops the four kinds and every value that names one

The conversion drops the 1,533 entities, and with them every value that names one:

- a constraint's `constrains`, 388 links in 63 ledger entries, one per constraint;
- a milestone's behaviors, a field that holds no value in the spec;
- the 8 `target_later` failure-mode values.

Each is ledgered with its reason. absorb reads the four kinds so that none is skipped unrecorded: it reads 17 old
kinds, where it read 13 at the baseline. How the table and the ledger record a dropped value is the conversion
change's design.

## D3. A release becomes a version milestone

A release becomes a version milestone, and an old milestone becomes a phase under it. The owner decided so on
2026-11-16. No `release` kind is added. Each of the 31 release-pending ledger entries gets a field gsd-core's
milestone defines, or is dropped with its reason; which fields a milestone keeps is decided in a later ADR. The
mapping is the conversion change's design.

## D4. The census holds per old kind and per entity

Every old entity is converted, dropped, or ledgered whole. A ledger entry counts an entity and a field, so entries are
never summed against entities. The table above is the census's first reading.

Its test lands with the conversion. Before the swap it reads the whole-spec run. After the swap it reads absorb's
fixture projects under `xtask/tests/fixtures/`, which must hold every old kind.

## D5. At the swap the root stops loading the old builtins

At the swap the root stops loading its six old builtins, among them `@specforge/software`, `@specforge/testing`,
`@specforge/formal` and `@specforge/cargo-test`, which declare the dropped software kinds and their tests
([ADR 0367](0367-the-self-spec-sample-retires-at-the-swap-and-absorb-stays-while-the-old-builtins-ship.md) D3 has the
whole list). The root loads no `@gsd/core` before the swap, so it never loads two declarers of one keyword. No other
project in the repository does either: no tracked `specforge.json` names `@gsd/core` at `f61370cb9`
(`git grep -l '@gsd/core' -- '*specforge.json'` lists nothing).

## Rejected

- **The four kinds as authored kinds of `@code/core`.** This ADR first recorded it. They are four kinds gsd-core does
  not define. Their keywords are ones `@specforge/software` declares while the root loads both extensions. And
  references would run both ways between `@gsd/core` and `@code/core`.
- **The four kinds in `@gsd/core`.** gsd-core does not define them either.
- **Keeping `@specforge/software` loaded in the root.** It is two stacks in one project, against the milestone's goal.
- **A kernel kind map.** ADR 0360 rejected it already: the kernel is domain-free.

## Consequences

- The root spec loses its behavior-level statements with their 6,449 `verify` statements. The ledger, with the tagged
  pre-swap commit, is their record.
- The test attributes that name a behavior, an event, a type or a port lose their target (5,845 lines hold
  `behavior = ` in 900 files). What they become belongs to the swap, and is decided in a later ADR.
- `@code/core` is unchanged.
- No protocol or SDK change comes with this ADR.

## Would reopen it

- gsd-core defining a kind for one of the four.
- A requirement that cannot be proved without a behavior-level statement the collectors do not record.

## Review findings

A review of ADRs 0364 to 0369 on 2026-11-16 raised 25 findings: CR-01, WR-01 to WR-19 and IN-01 to IN-05. Each has one
row, in the ADR it concerns most; these are this ADR's. Phases are those of milestone v4.1: 2 the vocabulary change, 3
the review of `@gsd/core`'s kinds and fields, 5 the conversion, 6 the vendoring, 7 the swap.

| Finding | What it said | Outcome | Delivered in |
|---|---|---|---|
| CR-01 | Landing the four kinds in `@code/core` puts E026 on the root project before the swap, and no ADR orders it | Moot. D1 adds no kind to `@code/core`, so it shares no keyword with `@specforge/software` and the root has no window with E026. D5's rule is what remains: the root loads no `@gsd/core` before the swap | phases 6 and 7 |
| WR-01 | The census marked `module` and `release` as converted, and the text said 757 were converted where the trial converted 700 | Fixed in Context: the census says a module and a release created no entity, and 700 were converted | this ADR |
| WR-02 | The obligations of `type` and `port` had no home | Moot. D1 and D2: no obligation home is added for any of the four kinds, they are dropped | - |
| WR-03 | The census equation mixed entities with ledger entries, and the project its test reads after the swap was not named | Fixed in D4: the census is per entity, and the projects its test reads before and after the swap are named | phase 5 |
| WR-05 | The direction of the references between `@gsd/core` and `@code/core` was not stated and looked circular | Moot. D1: no reference runs between `@gsd/core` and `@code/core` for these kinds | - |

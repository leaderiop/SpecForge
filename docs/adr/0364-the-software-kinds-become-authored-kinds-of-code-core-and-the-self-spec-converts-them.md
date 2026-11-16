# The software kinds become authored kinds of @code/core, and the self-spec converts them

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D8 (the behavior values that Later waits for have a home: authored `behavior`, `event`, `type` and `port` kinds of
`@code/core`, not an unnamed one) and D2 (the old kinds the table measures grow by those four). Builds on
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) (these four are authored, not
collected facts) and [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) (`@code/core` already holds the test
facts a spec-link attribute names). No protocol or SDK change comes with this decision.

## Context

SpecForge's own spec is written in the kinds of the old builtins. Absorb (ADR 0360) converts the 13 kinds of
`@specforge/governance` and `@specforge/product`. The four kinds of `@specforge/software` (`behavior`, `event`, `type`
and `port`) have no home in the target, and they are most of the spec. The figures below were measured at `f61370cb9`
and are the baseline later changes quote; they supersede the counts the research quoted, which are named where they
differ. Every figure carries its command in `../SpecForge-planning/r4b/BASELINE.md`.

**The census of the root spec.** Entities whose file is under `spec/` in the old graph (`specforge export --format
graph` of a scratch copy of `f61370cb9`, binary built with the test profile; the graph also holds 40,286 `item` and 78
`package` facts that collect added, which are not spec entities). Counted with
`jq '[.nodes[]|select(.file|startswith("spec/"))|select(.kind=="<kind>")]|length'`:

| Kind | Entities | Converted by absorb at f61370cb9 |
|---|---|---|
| `behavior` | 706 | no |
| `event` | 123 | no |
| `type` | 678 | no |
| `port` | 26 | no |
| `feature` | 141 | yes |
| `invariant` | 149 | yes |
| `decision` | 117 | yes |
| `term` | 100 | yes |
| `constraint` | 63 | yes |
| `journey` | 54 | yes |
| `module` | 54 | yes |
| `failure_mode` | 35 | yes |
| `milestone` | 18 | yes |
| `deliverable` | 12 | yes |
| `persona` | 6 | yes |
| `channel` | 5 | yes |
| `release` | 3 | yes |

That is 1,533 entities of the four software kinds against 757 of the 13 absorb kinds, and one `spec` block, 2,291 in
all. The research's 706, 123, 678 and 26 stand; its second grep's 833 behaviors and 162 events are refuted.

**The obligations they carry**, same graph and expression family:

| Obligation | Count | Where |
|---|---|---|
| `verify` statements, all spec entities | 7,001 | `behavior` 5,632, `type` 617, `invariant` 379, `event` 174, `constraint` 139, `failure_mode` 34, `port` 26 |
| `requires` entries | 502 | on 358 behaviors |
| `ensures` entries | 2,261 | on 602 behaviors |
| `maintains` entries | 26 | `behavior` 12, `type` 14 |
| limit links (`constrains` edges) | 388 | constraint to behavior |
| lines holding `behavior = ` in tracked `*.rs` | 5,845 | in 900 files, by `git grep` at `f61370cb9` (textual) |
| strings `verify = "` in tracked `*.rs` | 6,168 | in 933 files, by `git grep` at `f61370cb9` (textual) |

The four software kinds hold 6,449 of the 7,001 `verify` statements. The research's 5,632 to 7,023 becomes 7,001, and
its 5,818 spec-link attributes becomes 5,845 (5,826 in the string form). Its 984 `requires` and `ensures` is not
reproduced: 2,763 entries on behaviors (358 behaviors hold `requires`, 602 hold `ensures`) is the unit a home must be
able to represent.

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

The run converts none of the 1,533 software entities, and it ran with the old builtins loaded: the 66.47 s covers
about a third of the spec's entities. The engine is superlinear in the plan's size (ADR 0360 Consequences), so the
whole-spec run with the four kinds added is a step to be measured, not extrapolated.

**Why the swap cannot leave them.** A swap that unloads `@specforge/software` with these entities unconverted fails
E024 (unknown entity kind) on each of the 1,533. Keeping it loaded keeps the root on an old builtin, which is the
milestone's goal reversed: the root would hold two stacks, and `behavior`, `event`, `type` and `port` would be the one
domain of the old spec that never moved.

## D1. `@code/core` declares authored `behavior`, `event`, `type` and `port`

The four kinds are authored kinds of `@code/core`, holding the fields the old software kinds hold. `@code/core` is their
home because it already owns the test facts an attribute names (ADR 0329) and the code a behavior describes, and keeps
`@gsd/core` free of software semantics. They are authored: a collector's facts stay read-only (ADR 0327), and a
`behavior` is not a fact. Their field-level design is the change that lands them, with its own ADR.

## D2. Their obligations have a home and keep their proofs

A behavior's and an event's obligations (`verify`, `requires`, `ensures`, `maintains`) have a home in `@code/core` and
keep their proofs: a `verify` statement that a test proves today is proven by the same test after the move. The shape
of that home follows the obligation shape of decision E (ADR 0368, an `obligations` block list on requirement and
risk), so one idea of an obligation serves all of them.

## D3. absorb converts them, and a residual census test holds

The old kinds absorb reads grow from 13 to 17. Each of their fields gets a row in the table that is To, Computed or
Refused, never silently dropped, and a residual census test holds: for every old kind, the old entities equal those
created plus those ledgered. The census above is its first reading.

## D4. A requirement and a milestone name behaviors that resolve

A requirement names the behaviors it limits and enforces (the old constraint's `constrains`, 388 links, 63 of them
ledgered Later today), and a milestone names its behaviors, each reference resolving to an authored `behavior`. These
are the homes of the two Later behavior rows of ADR 0360 D8 and of the 8 target_later entries. The behavior pending
(63 Later entries and the 8 target_later) clears when this lands; Later reaches 0 once the release, package and truth
homes have landed too.

## D5. At the swap the root stops loading four old builtins

At the swap the root project stops loading `@specforge/software`, `@specforge/testing`, `@specforge/formal` and
`@specforge/cargo-test`. The crates stay builtins while examples and `integrations/rust/spec` use them (decision D, ADR
0367, named here without a link because it is recorded after this one).

## Rejected

- **Staging**: keeping the four software builtins loaded in the root for this milestone. It is two stacks in one
  project, against the milestone's goal, and the behaviors whose `features` field names the `feature` kind that is gone
  still need retargeting, so the work is only postponed.
- **The four kinds in `@gsd/core`**: it gives gsd the software semantics it is kept free of, and `@code/core` already
  owns the test facts the attributes name.
- **A kernel kind map**: ADR 0360 Rejected already; the kernel is domain-free.

## Consequences

- `@code/core` is re-vendored where the kinds land, with its declaration snapshots.
- The whole-spec plan grows by the four kinds' entities, 1,533 beside the 757 now converted. The engine is superlinear
  (ADR 0360 Consequences), so each whole-spec run stays a measured step and quotes this census.
- The `links` check (A016), which reads the spec-link attributes (`behavior = "..."`, `verify = "..."`) against the
  old builtins' entities, gets a replacement that resolves them against authored behaviors. The fate of the
  `verify = "..."` attribute texts (6,168 in 933 files) is decided with that replacement, not here.
- No protocol or SDK change comes with this ADR; the kinds are declared by an extension in its own change.

## Would reopen it

- A software kind that cannot be authored in `@code/core` without breaking its collected facts (ADR 0327).
- A whole-spec conversion too slow for the gate once the four kinds are in the plan.

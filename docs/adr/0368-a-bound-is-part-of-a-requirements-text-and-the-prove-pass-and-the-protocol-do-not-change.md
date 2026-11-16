# A bound is part of a requirement's text, and the prove pass and the protocol do not change

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D7 (the converter writes every `metric` line of an old constraint into the requirement's text; it writes no `metric`
value, collapses no strict bound and ledgers no line `unconverted` for not being one bound) D8 (truth values wait
for no kind and no field) and D11 (the nine WASM-host constraints keep their bound line as text; none becomes a
`metric` record and none is collapsed). Builds on
[ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md) (the rule that no kind
is added to port the old spec, and the whole-spec run the figures come from). No protocol or SDK change comes with
this decision: the protocol stays 2.2.0, unreleased, and no builtin is re-vendored for it.

## Context

On 2026-11-16 the owner decided that a bound lives in the requirement's text: gsd-core's requirement has no field for
a measurable bound, and no field is added to port the old spec.

The figures were measured at `f61370cb9`, from the old graph and from the ledger of the whole-spec `absorb` run that
ADR 0364 describes. The full command list is kept in `../SpecForge-planning/r4b/BASELINE.md`, a local folder this
repository does not track.

- **Unproven.** The ledger holds 197 `unproven` entries, all `invariant.verify` obligations of the old invariants,
  each with an empty `tests` list at conversion time: no test item states them. Counted by
  `jq '[.entries[]|select(.why.unproven!=null)]|length' ledger.json`.
- **Unconverted.** 60 entries (`jq '[.entries[]|select(.why.unconverted!=null)]|length' ledger.json`), of which 52 are
  constraint `metric` lines that are not one bound and 8 are scaffold entries of the target (its `spec` block and
  `specforge.json`), not entities of the old spec.
- **Metrics.** 63 old constraints hold a `metric` (`jq` over the old graph: kind `constraint`, `fields.metric` not
  null). 11 of them are one bound line (`<name> <op> <number><unit>`); the 52 others were ledgered `unconverted` (ADR
  0360 D7). The 11 are the 63 less the 52 constraint entries of
  `jq '[.entries[]|select(.kind=="constraint" and .field=="metric" and .why.unconverted!=null)]|length' ledger.json`.
- **Formal claims.** One old invariant holds an `expression`: `performance_claim_lattice` (`jq` over the old graph:
  kind `invariant`, `fields.expression` not null).
- **Truth-pending rows.** Six rows of `xtask/src/absorb/table.rs` wait for a truth (`grep -c 'later(Truth)'`): four
  field rows (`constraint.enforced_by`, `invariant.expression`, `feature.acceptance`, `feature.tests`) and two
  obligation rows (`constraint.verify`, `failure_mode.verify`). They carry 182 Later ledger entries.
- **What the prove pass reads today.** `crates/app/ops/src/analysis/prove.rs` reads the text fields an extension gives
  a proof role: `Bound`, a fact the solver assumes, or `Claim`, a statement the bounds must entail. It reads a string
  line or an expression (`conjuncts_from_field`).

## D1. No list of obligation blocks is added

Not to a requirement, not to a risk, not to any kind. The six truth-pending rows get no new home. The conversion
change gives each one outcome: a field gsd-core defines, or dropped with its reason. `constraint.enforced_by` holds no
value in the spec, so its row is dropped with nothing lost.

## D2. A bound is part of the requirement's text

Every `metric` line of an old constraint is written into the requirement's text as it stands, strict or not. The 11
single-bound constraints and the 52 others convert alike, so the 52 `unconverted` entries convert. A requirement holds
no `metric` for it. The field leaves the `@gsd/core` requirement with the review of its fields, decided in a later ADR.

## D3. The prove pass and the protocol do not change

No requirement field gets a proof role. There is no protocol bump and no re-vendor. A bound in a requirement's text is
prose, and the solver does not check it.

What becomes of an old invariant (its guarantee, its 197 unproven `verify` lines, the one formal claim) is decided in
a later ADR. No home for a claim is added here.

## D4. A gate that reads proofs fails when no requirement is proven

It is one predicate: no requirement's computed `proof` is `proven`. So a project with no requirement and a project
with none proven are both red. The dogfood check takes the predicate at the swap.

## Rejected

- **A list of obligation blocks on requirement and risk.** This ADR first recorded it. They are fields gsd-core does
  not define.
- **The prove pass reading a requirement's `metric`, with or without a protocol minor bump.** It is a kernel and wire
  change made to keep a field gsd-core does not write.
- **A bound string in a field of its own.** It is one more field gsd-core does not define.

## Consequences

- No builtin is re-vendored, and the vendoring of `@gsd/core` waits for no protocol change.
- After the swap the solver no longer checks the root spec's bounds. `@specforge/formal` keeps proving the old
  projects that load it.
- The 52 constraint `metric` lines convert.
- The absorb table's truth rows and its measure counts change with the conversion change.

## Would reopen it

- gsd-core gaining a field for a measurable bound.
- A bound of the root spec that must be machine-checked.

## Review findings

A review of ADRs 0364 to 0369 on 2026-11-16 raised 25 findings: CR-01, WR-01 to WR-19 and IN-01 to IN-05. Each has one
row, in the ADR it concerns most; these are this ADR's. Phases are those of milestone v4.1: 2 the vocabulary change, 3
the review of `@gsd/core`'s kinds and fields, 5 the conversion, 6 the vendoring, 7 the swap.

| Finding | What it said | Outcome | Delivered in |
|---|---|---|---|
| WR-04 | `constraint.enforced_by` had no home, and ADR 0364 had a requirement enforce behaviors | Fixed in D1: the `enforced_by` row is dropped, and it held no value. ADR 0364 no longer has a requirement enforce behaviors | phase 5 |
| WR-16 | The protocol was bumped a minor, against the rule that an unreleased minor takes additions | Moot. D3: no protocol change at all | - |
| WR-17 | The 11 converted bounds were said to be proved as they were, while a strict bound was collapsed | Moot. D2: the bound is the requirement's text as written, strict or not, and nothing is collapsed | - |
| WR-18 | What turns the proof check red was ambiguous: nothing proven, or nothing to prove | Fixed in D4: one predicate | phase 7 |

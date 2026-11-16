# A requirement and a risk hold obligations, and the prove pass reads a requirement's metric

**Status:** accepted (2026-11-16). **Amends** [ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md)
D8 (the homes of the truth values it leaves Later: an `obligations` block list on requirement and risk, and an
invariant's `expression`) and D7 (the `metric` record the converter writes is what the prove pass reads, where it read
the bound line). Builds on
[ADR 0359](0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md) (a requirement's `metric` is
a record, and a second store of one value is not wanted),
[ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) (a risk is rated, an invariant is judged) and
[ADR 0364](0364-the-software-kinds-become-authored-kinds-of-code-core-and-the-self-spec-converts-them.md) (a behavior's
obligations take the same shape). **Protocol 2.3.0**, as ADR 0363 states its protocol: the prove pass reading a record
is a wire change, because a record field has to declare the proof role the pass reads. It is the first commit of the
change that lands it, before `@gsd/core` is vendored
([ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) D6), and it re-vendors every
builtin, since each blob states the protocol it was built with.

## Context

The figures are the baseline's (`../SpecForge-planning/r4b/BASELINE.md`, measured at `f61370cb9` from the old graph
and from the ledger of the whole-spec `absorb` run described in ADR 0364); each row there carries its `jq` or `git
grep` command.

- **Unproven.** The ledger holds 197 `unproven` entries (research quoted 197: confirmed), all `invariant.verify`
  obligations of the old invariants, each with an empty `tests` list at conversion time: no test item states them.
  Counted by `jq '[.entries[]|select(.why.unproven!=null)]|length' ledger.json`.
- **Unconverted.** 60 entries (research quoted 61), of which 52 are constraint `metric` lines that are not one bound and
  8 are scaffold entries of the target (its `spec` block and `specforge.json`), not entities of the old spec.
- **Metrics.** 63 old constraints hold a `metric`. 11 of them are one bound line (`<name> <op> <number><unit>`) and
  convert to a record; the 52 others are ledgered `unconverted` (ADR 0360 D7). The 11 are the 63 less the 52 constraint
  entries of `jq '[.entries[]|select(.kind=="constraint" and .field=="metric" and .why.unconverted!=null)]|length'`.
- **Formal claims.** One old invariant holds an `expression`: `performance_claim_lattice`.
- **Later, truth.** 6 `later(Truth)` rows of `xtask/src/absorb/table.rs` (4 field rows and 2 obligation rows; counted by
  `grep -c 'later(Truth)'`), which carry 182 `later` ledger entries waiting for a truth.
- **What the prove pass reads.** `crates/app/ops/src/analysis/prove.rs` reads the fields an extension declares a proof
  role for, `Bound` (a fact the solver assumes) or `Claim` (a statement the bounds must entail), and reads them as
  text: a string line, or an expression (`conjuncts_from_field`). A `metric` record is neither, so it is not proved:
  the old constraint's bound was proved only because it was written as a line.
- **Why not `@gsd/core`'s `truth`.** Its `truth` kind is plan-scoped (`gsd/core/src/kinds/must_haves.rs`: a plan's
  `must_haves`, parented to a plan), and the milestones that converted deliverables and milestones have no plans (ADR
  0360 Consequences). A requirement's acceptance has no plan to hang a truth from.

## D1. A requirement and a risk hold an `obligations` block list

A requirement and a risk hold an `obligations` block list. Each obligation has a kind (`acceptance` is one), a text,
and a proof: the tests that prove it. A behavior's and an event's obligations (ADR 0364 D2) take the same shape in
`@code/core`, so one idea of an obligation serves every kind that has any. This is the home of an old feature's
acceptance and tests, and of an old constraint's and failure mode's `verify` obligations (ADR 0360 D8 left them Later).

## D2. An obligation's status is read from its proof's tests

An obligation is `proven` when every test its proof names passes, `failing` when one fails, and `unproven` when it names
no test. The status is computed, never written. A proof that names a test not collected yet is `code/W102`, as ADR 0360 D6
says of a proof of an uncollected test.

## D3. An invariant's `expression` carries a claim the prove pass reads

An invariant holds an `expression`, which the prove pass reads as a claim; a false claim fails the proof. The one
formal claim the old spec holds (`performance_claim_lattice`) has this home, and a conversion that writes it is no
longer a Later row.

## D4. The prove pass reads a requirement's `metric` record

The prove pass reads a requirement's `metric` record as a bound (protocol 2.3.0, above): the record's name, direction,
threshold and unit state the bound the line stated. So the 11 constraints whose single bound line converted to a record
are proved as they were, and no bound string is written beside the record. A constraint whose `metric` is not one
bound line stays ledgered `unconverted` (ADR 0360 D7); each of the 52 gets an outcome in the triage, not a silent loss
of its proof. The descriptor form that carries the role on a record field is the design of the change that lands it.

## D5. A gate that reads proofs fails when no obligation is proven

A check that reads the proof of the obligations fails when no obligation is proven, so an empty result is red and not a
pass: the dogfood check after the swap reads obligations through these kinds and fails closed on a project that
converted none.

The fate of the `verify = "..."` attribute texts in Rust test attributes is not decided here: it is decided with the
replacement of the `links` check (A016), in the change that does.

## Rejected

- **A bound string written beside `metric`.** It is a second store of one value, as ADR 0359 rejected for the
  requirement, and the two would drift.
- **The plan-scoped `truth` as the home.** A converted milestone has no plans (Context), so an obligation with no plan
  would have no parent.
- **One entity per obligation.** An obligation nothing else references is a value of its holder; an entity per
  obligation is outside this milestone: the milestone's requirements list it as a future requirement.

## Consequences

- Protocol 2.3.0 re-vendors every builtin in the commit group that introduces it, before `@gsd/core` is vendored (ADR
  0366 D6). A guest built at protocol 2.2.0 still loads under a 2.3.0 host (E088: an older minor loads, and
  `docs/extension-protocol.md` states each minor's guests the same way); a guest built at 2.3.0 is refused by a 2.2
  host with E088, "Extension speaks a protocol this specforge does not". The version table of that document gains a
  `2.3.0` row, and the handshake examples state it.
- Each of the 197 unproven obligations and the 52 unconverted metrics still needs an outcome in the triage (a test to
  cite, or a recorded decision); this ADR gives them a home, not a verdict.
- The dogfood check after the swap reads obligations through these kinds (D5), and the old `@specforge/formal` bound
  and claim roles keep proving the old projects the examples hold.
- The truth rows of ADR 0360's table move from Later to To when the kinds land, and the measure test's counts move
  with them (ADR 0360 Would reopen it).

## Would reopen it

An obligation that another entity must reference (it would then become an entity), or a metric the prove pass cannot
read as a record.

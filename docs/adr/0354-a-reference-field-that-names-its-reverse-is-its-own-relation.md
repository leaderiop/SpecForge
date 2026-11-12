# A reference field that names its reverse is its own relation

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0150](0150-a-relation-is-written-on-one-side.md) D4 (W162's kind-pair rule leaves out a field that names its
reverse). No protocol change: `inverse_of` is already on the wire (ADR 0150 U2).

## Context

- ADR 0150 D4 makes W162 the load warning for a relation an extension declares from both of its ends, by two rules:
  (a) a reference field whose `inverse_of` names a field its target kind registers; (b) two different kinds that each
  register a reference field targeting the other, reported once per kind pair.
- Rule (b) reads a kind pair as one relation. Two kinds can hold two relations: `@gsd/core`'s `decision.plan` names
  the quick plan whose context recorded a decision, and `plan.decisions` lists the context decisions a plan
  implements. Each is written on one side and read back on the other; rule (b) took them for one relation written
  twice, so every project loading `@gsd/core` would load with W162.
- An extension had no way to say "these are two relations": `inverse_of` already names the word the target kind reads
  a relation by, so a field that carries one has declared its own reverse.

## Decisions

- **D1. A reference field with an `inverse_of` pairs with nothing in rule (b).** It names its reverse, so it is its
  own relation; rule (b) pairs only fields that name no reverse. Two kinds linked both ways by fields that name no
  reverse stay W162, once per kind pair, as before.
- **D2. Rule (a) is unchanged.** An `inverse_of` naming a field its target kind registers is still W162 on that
  field: the name must be a word the target kind does not use, so a field cannot silence rule (b) by naming the field
  of the other side.
- **D3. Two relations between the same kinds are told apart by giving one an `inverse_of`.** The W162 explanation
  says so; `plan.decisions` is read back as `implemented_by`.

## Rejected

- **Moving one field into a block member**, as a block member is not paired: it changes the authored shape of a
  relation to dodge a rule that was the imprecise part.
- **An explicit "distinct relation" flag on the field.** A protocol addition for what `inverse_of` already says.
- **Dropping rule (b).** Two kinds linking each other with no reverse named is still the common double declaration
  ADR 0150 removed.

## Consequences

- The trade-off: an `inverse_of` silences rule (b) even for a genuine double declaration. `parent.children` with
  `inverse_of "kids"` beside `child.parent` is one relation declared from both ends, and W162 no longer says so; the
  `inverse_of` is the extension author's statement that the fields are two relations, and the warning trusts it.
- The builtin extensions are unaffected: they declare every relation from one side.

## What would reopen it

A double declaration that an `inverse_of` hides reaching a project, where the graph holds two edges for one fact.

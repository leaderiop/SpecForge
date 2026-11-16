# A code's findings may wait for its owner state, and a write that reveals one adds no error

**Status:** accepted (2026-11-15). **Amends** [ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md)
(E080 counts a finding a write revealed as no error it adds, beside a judged rule's) and
[ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md) (a `codes` entry may declare `reported_while`).
Extends [ADR 0362](0362-a-judged-target-may-wait-for-its-owner-state.md) from a judged target to a code, on the owner
state of [ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D4. Protocol 2.2.0, unreleased: the addition goes
into the 2.2.0 row, no bump. The wire and the SDK change, so the builtins are re-vendored.

## Context

- E080 refuses a typed edit when the project it leaves reports an error the project does not (ADR 0167, keyed by code,
  severity, message shape, file and, for an extension's finding, its subject, ADR 0302). Only a judged rule's finding is
  exempt (ADR 0297): its judge settles it, not the edit.
- `@gsd/core`'s invariant is pending until its `holds_from` phase has run (ADR 0358 D10), and owes nothing then: its
  verdict waits for it to be in force (ADR 0362), and so does gsd/E531, the error that a phase changed what it guards
  after its last verdict. A verdict may be recorded early, while it is pending.
- An invariant judged early, whose `holds_from` phase then changes what it guards, is due the moment it is in force.
  The write that brings it into force (that phase's verification, a status, a `holds_from` moved to a phase already
  run) left a project with an E531 the project before it could not report, since nothing pending owes one. E080
  refused that write, the only way forward: the invariant could never come into force, so it could never be judged
  again, and the phase could not be verified. A deadlock.
- The error is not the write's. The change that makes the invariant due was committed before; the write only moved its
  owner into the state where the finding speaks. Treating it as added confuses "now visible" with "now caused".

## D1. `reported_while`, a property of the code

A `codes` entry (`CodeDescriptor.reported_while`) may declare `complete` or `pending`, an `OwnerState` word kept a
string, the words of a field's `checks_while` (ADR 0350) and a judged target's `judged_while` (ADR 0362). The SDK
writes it with `CodeBuilder::once_complete()` (`complete`) and `while_open()` (`pending`). Absent, a finding of the
code speaks in either state. A word that is neither is E030, and the entry is dropped from the codebook, its code still
the extension's to report, waiting for nothing. A first-party extension declares no `codes` (ADR 0286), so no host code
waits.

## D2. The host keeps such a finding only in its owner state

A finding of the code, reported by its extension (its origin) on an entity (its subject), is kept only when that
entity's owner state is the one its code waits for: ADR 0329 D4's, read on the snapshot the compute passes' values are
merged into (`EntitySnapshot::is_pending`), as ADR 0362 D2 reads it. A finding on no entity, or on an entity the
snapshot does not hold, is kept. The extension may still report in every state; the host decides, so a pass need not
compute the owner state the kernel already computes.

## D3. E080: a finding the write revealed is no error it adds

A typed edit and the under-lock re-judge count an unmatched finding as no added error when the write revealed it: its
code waits for an owner state, and the write moved its entity from the other state into it. The entity must be the one
the project held, judged from the write's plan and not from the ids the project held: an entity the plan creates,
deletes or moves (at either id), or one under such an entity, reveals nothing, so a delete and a create of the same id
in one write make another entity, not a revealed one. A migration, the rule's other caller, has no such plan and
reveals nothing. Both states must also be known: a project whose checks were skipped (the typing fast path, no
computed value says its state) reveals nothing. Every other unmatched error is counted as before, the waiting code's
included: on an entity whose state the write did not move, a finding the write makes appear is added (a widening of
what a judged invariant in force guards, for one), and on a pending entity a code that waits for nothing is added. The
finding is not hidden: it is reported with the write's other findings, `check` and every gate read it, and it stays
until what it names is settled (for E531, judging the invariant again).

## Rejected

- A gsd-only fix (report E531 on a pending invariant, or never on the write that brings it into force): the project
  before the write cannot report what a pending invariant does not owe, and a pass cannot tell which write it runs for.
- Exempting every finding on an entity whose owner state a write moved: it would also exempt a check that waits for
  an owner (`checks_while`, ADR 0350) and any extension error on that entity, weakening E080 for all of them.
- Refusing an early verdict, or the write that changed the guarded code: the first loses the judgement made, the
  second refuses a commit the gate has no say over.
- Keying the finding without its subject's state: a key that changes with the state would make every finding on an
  entity whose state moved a new one, the opposite of what is needed.

## Consequences

- The write that brings a due invariant into force is accepted; gsd/E531 is reported once it is written, and judging the
  invariant again clears it. A `holds_from` moved to a phase already run is such a write; widening what an invariant
  in force guards over a changed phase is still refused.
- A pass that reports a waiting code on a pending entity reports nothing the host keeps: the code's findings never
  speak in the other state, in `check`, in a gate or in a write's output.

## Would reopen it

- A code whose findings must wait for a state other than its subject's owner state.
- An error that a write reveals some other way than by moving an owner state, and that must not refuse it either.

# Invariant is governance's, and formal needs no software

**Status:** accepted (2026-10-26, architecture round 29, plan 04). Amends [ADR 0002](0002-test-runner-extensions.md)
(invariant's owner is governance; `init` enables governance with software).

`invariant`, a rule that must always hold, was declared by `@specforge/software`, so a product, marketing or regulated
project that wanted rules had to load the software extension (`E024 … install it with: specforge add
@specforge/software`). `@specforge/formal` declared software a required peer (E027), so a project of rules, limits and
lifecycles (governance + formal) could not load formal at all, and W123 (properties) and W128 (protocols) warned on
every entity because the only kinds that can reference them, behavior and event, are software's.

## D1. Governance declares invariant

`@specforge/governance` declares `invariant` (its `risk` vocabulary and W003, untyped: any reference silences it) beside
the decisions, constraints and failure modes that already reference it. Software keeps behavior, event, type and port and
the `BehaviorEnforcesInvariant` link (its writer is `behavior.invariants`), with governance an optional peer.
`@specforge/testing` names governance as invariant's owner.

## D2. Formal peers software and governance optionally

A formal rule whose only possible reference comes from another extension's kind names that edge (`edge_type`): W123
`BehaviorSatisfiesProperty`, W128 `EventFollowsProtocol`. Where the kind does not load, no field writes the edge and the
rule is inert ([ADR 0020](0020-rules-are-a-typed-set.md) D5, per writer). W134 stays untyped: a sub-process is
referenced by its parent, so a root process in a project without events still warns.

## D3. `init` enables governance with software

A project created with software keeps the invariants it could write before (U4 = a). No `migrate` step: the E024 help
names `specforge add @specforge/governance` (U5 = a).

## D4. Behavior and event stay software's

They carry software's design contract (ports, types, payloads, category routing). What would reopen it: a domain whose
step or record needs a contract shape software's behavior and event do not have.

```text
BEFORE                                         AFTER
software: behavior invariant event type port   software: behavior event type port (peers: product, governance)
governance: decision constraint failure_mode   governance: decision constraint failure_mode invariant · W003
formal ──REQUIRED──▶ software                  formal ──optional──▶ software, governance
W123/W128 untyped                              W123/W128 typed by the edge their message names
init: software ⇒ testing                       init: software ⇒ testing, governance
```

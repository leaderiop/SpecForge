# decision

> **Declared by `@specforge/governance`**, one of the builtin extensions. `decision` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/governance`. The fields, edges and rules below are read from its declaration.

## Purpose

A `decision` records a significant choice and why it was made: the context, the choice, the alternatives and the
consequences. In software it is an Architecture Decision Record (ADR); the same shape fits a pricing decision, a
supplier choice or a change to a procedure. It answers: **"Why was this done this way?"**

Decisions keep knowledge from living only in someone's memory. They can point at what they protect (invariants),
what they impose (constraints) and the features they affect.

## Syntax

```spec
decision use_postgresql "PostgreSQL over MongoDB" {
  status   accepted
  date     2025-03-01
  deciders ["platform-team"]

  context """
    We need a primary datastore. The team has SQL expertise.
    The data is relational; a document model is not needed.
  """

  decision "Use PostgreSQL with typed schemas."

  alternatives ["MongoDB", "DynamoDB"]

  consequences [
    "Migrations are required for schema changes",
    "Strong ACID guarantees",
  ]

  invariants [data_persistence]
}
```

## Fields

Every `decision` has an id (the word after `decision`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `status` | enum: proposed, accepted, deprecated, superseded | Current lifecycle status of the decision. |
| `context` | string | Background and circumstances that motivated this decision. |
| `decision` | string | The decision statement itself. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `consequences` | string list | Expected outcomes and trade-offs resulting from this decision. |
| `superseded_by` | reference → `decision` | Reference to a newer decision that replaces this one. |
| `invariants` | reference list → `invariant` | Invariants that this decision protects. |
| `refs` | string list | External references (URLs, documents, issue links). |
| `description` | string | Free-form description of the decision. |
| `date` | date (`YYYY-MM-DD`) | The day the decision was made or last updated. |
| `authors` | string list | People who authored this decision record. |
| `tags` | string list | Categorization tags for filtering and grouping. |
| `alternatives` | string list | Alternative options that were considered. |
| `reason` | string | Justification for why this option was chosen over alternatives. |
| `deciders` | string list | People who approved or signed off on this decision. |
| `affects_features` | reference list → `feature` | Product features affected by this decision (requires @specforge/product). |
| `constraints` | reference list → `constraint` | Constraints imposed by this decision. |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [feature](feature.md) | `DecisionAffectsFeature` | `affects_features` on `decision` | Decision affects a product feature |
| outgoing | [constraint](constraint.md) | `DecisionImposesConstraint` | `constraints` on `decision` | Decision imposes a constraint on the system |
| outgoing | [invariant](invariant.md) | `DecisionProtectsInvariant` | `invariants` on `decision` | Decision protects an invariant |
| outgoing | [decision](decision.md) | `DecisionSupersedesDecision` | `superseded_by` on `decision` | Decision supersedes a previous decision |
| incoming | [decision](decision.md) | `DecisionSupersedesDecision` | `superseded_by` on `decision` | Decision supersedes a previous decision |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.


## Status lifecycle

```
proposed -> accepted -> deprecated
               |
           superseded (name the replacement in superseded_by)
```

## Design guidance

### Good decisions

Capture the **context** (what forced the choice), the **alternatives** considered, the **reason** this one won,
and the **consequences**, both benefits and trade-offs.

### When to write one

Write a decision when choosing between technologies or suppliers, when setting a convention that affects many
parts, or when a choice would be expensive to reverse. Skip trivial, standard or easily reversible choices.

## Examples

### Architectural pattern

```spec
decision event_sourcing_audit "Event Sourcing for Audit Trail" {
  status accepted
  date   2025-06-15

  context """
    Regulatory requirements demand a complete, immutable audit trail
    of all state changes.
  """

  decision "Use event sourcing for the billing domain: state changes are an append-only event log."

  consequences [
    "Complete audit trail by construction",
    "Increased storage requirements",
  ]

  invariants  [audit_integrity]
  constraints [audit_retention]
}
```

### Superseded

```spec
decision rest_internal "REST for Internal Services" {
  status        superseded
  date          2024-01-10
  context       "We needed a simple protocol between internal services."
  decision      "Use REST/JSON between internal services."
  superseded_by migrate_grpc
}

decision migrate_grpc "gRPC for Internal Services" {
  status   accepted
  date     2026-02-15
  context  "Serialization overhead is more than 15% of p99 latency."
  decision "Use gRPC with Protocol Buffers between internal services; the external API stays REST."
}
```

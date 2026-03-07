---
name: specforge-decisions-dsl
description: "Write decision (ADR) blocks in .spec DSL files (@specforge/governance plugin). Each decision declares an Architecture Decision Record with free-form snake_case IDs, status lifecycle, context/decision/consequences structure, and invariant protection links. Use when documenting why the system is built a certain way."
---

# SpecForge Decisions DSL

Rules and conventions for authoring **`decision` blocks** in `.spec` files. Decisions are Architecture Decision Records (ADRs) -- they capture the *why* behind significant technical choices.

**Requires:** `@specforge/governance` plugin.

## When to Use

- Documenting why a technology was chosen (database, framework, language)
- Recording architectural pattern choices (sync vs async, monolith vs microservices)
- Establishing conventions that affect multiple components
- Preserving rationale for choices that would be expensive to reverse

## Block Syntax

```spec
decision postgres_over_mongodb "PostgreSQL over MongoDB" {
  status   accepted
  date     2025-03-01

  context """
    We need a primary datastore. Team has SQL expertise.
    Document model not needed -- data is relational.
  """

  decision """
    Use PostgreSQL with typed schemas.
  """

  consequences [
    "Migrations required for schema changes",
    "Strong ACID guarantees",
    "Team can leverage existing SQL knowledge",
  ]

  invariants [data_persistence]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `status` | enum | `proposed`, `accepted`, `deprecated`, `superseded`. |
| `context` | string / triple-string | What situation motivated this decision. |
| `decision` | string / triple-string | What was chosen. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `date` | date | When the decision was made (YYYY-MM-DD). |
| `consequences` | string list | Known consequences -- both positive and negative. |
| `invariants` | reference list | Invariants this decision protects. |
| `refs` | reference list | External references linked to this decision. |

### Status Lifecycle

| Status | Meaning |
|--------|---------|
| `proposed` | Under discussion, not yet accepted. |
| `accepted` | Approved and in effect. |
| `deprecated` | Superseded by a newer decision. |
| `superseded` | Replaced by another ADR. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `invariant` | `protects` | Decision protects these invariants |
| `ref` | `links_to` | External references linked to this decision |

### Incoming edges

None. Decisions are referenced informally from behavior contracts.

## Writing Rules

1. **Context explains forces** -- what problem, constraints, and trade-offs led to this decision.
2. **Decision is the choice** -- not the rationale (that is in context and consequences).
3. **Consequences include negatives** -- honest trade-offs, not just benefits.
4. **Link to invariants** -- decisions should protect invariants when applicable.
5. **Status transitions** -- `proposed -> accepted -> deprecated/superseded`.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `invariants` must resolve to an existing invariant. |
| E002 | No duplicate decision IDs. |
| I001 | Stale proposal -- `proposed` status with `date` older than 30 days. |

## Examples

### Technology Choice

```spec
decision postgres_over_mongodb "PostgreSQL over MongoDB" {
  status   accepted
  date     2025-03-01

  context """
    We need a primary datastore. Team has SQL expertise.
    Document model not needed -- data is relational.
    ACID transactions are required for financial data integrity.
  """

  decision """
    Use PostgreSQL 15+ with typed schemas and row-level security.
  """

  consequences [
    "Migrations required for schema changes",
    "Strong ACID guarantees",
    "Team can leverage existing SQL knowledge",
    "No native document storage -- complex JSON queries are slower",
  ]

  invariants [data_persistence]
  refs [gh.discussion:12]
}
```

### Architectural Pattern

```spec
decision event_sourcing_for_audit "Event Sourcing for Audit Trail" {
  status   accepted
  date     2025-06-15

  context """
    Regulatory requirements demand a complete, immutable audit trail.
    A traditional update-in-place model would require separate audit
    logging with risk of drift.
  """

  decision """
    Use event sourcing for the billing domain.
    All state changes stored as an append-only event log.
  """

  consequences [
    "Complete audit trail by construction",
    "Event replay enables point-in-time recovery",
    "Increased storage requirements",
    "Eventually consistent read models required",
  ]

  invariants [audit_trail_integrity, event_ordering]
}
```

### Proposed (Not Yet Accepted)

```spec
decision grpc_for_internal_services "Migrate from REST to gRPC for Internal Services" {
  status   proposed
  date     2026-02-15

  context """
    Internal service-to-service communication uses REST/JSON.
    Serialization overhead is measurable at current scale.
  """

  decision """
    Adopt gRPC with Protocol Buffers for all internal service communication.
    External API remains REST/JSON.
  """

  consequences [
    "~40% reduction in serialization latency (estimated)",
    "Compile-time type safety between services",
    "Requires protobuf tooling in CI",
    "Two API paradigms to maintain",
  ]
}
```

## What NOT to Do

- Do not write decisions without the `@specforge/governance` plugin installed
- Do not write ADRs for trivial, easily reversible choices
- Do not omit consequences -- especially negative trade-offs
- Do not leave `proposed` decisions for more than 30 days without resolving (I001 warning)

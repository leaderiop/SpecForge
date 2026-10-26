# constraint

> **Declared by `@specforge/governance`**, one of the builtin extensions. `constraint` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/governance`. The fields, edges and rules below are read from its declaration.

## Purpose

A `constraint` states a measurable quality or limit something must meet: a latency, an encryption rule, a data
residency requirement, a regulatory threshold. It answers: **"What quality must be achieved, and how is it
measured?"**

An invariant is binary (it holds or it is broken); a constraint has a metric and a threshold. It can name the
behaviors it constrains, the behaviors that enforce it, the invariants it protects and the features it governs.

## Syntax

```spec
constraint api_latency "API Latency Under Load" {
  description "The public API stays fast under sustained load."
  category    performance
  priority    high
  metric      "response_time_p99 at 1000 concurrent users for 5 minutes"
  threshold   "< 200ms"
  scope       "per-request"

  constrains [create_user, read_user]
  protects   [data_persistence]

  verify load "k6 load test with 1000 virtual users for 5 minutes, assert p99 < 200ms"
}
```

### Categories

`category` is one bare word from an open set. Common ones:

| Category | Covers | Example |
|----------|--------|---------|
| `performance` | Latency, throughput, resource use | "p99 < 200ms" |
| `security` | Authentication, authorization, encryption | "All PII encrypted at rest" |
| `reliability` | Availability, fault tolerance, recovery | "99.9% uptime" |
| `scalability` | Load capacity, data volume | "100k concurrent users" |
| `compatibility` | Platform support, interoperability | "Node.js 18+" |
| `usability` | Accessibility, learnability | "WCAG 2.1 AA" |
| `legal` | Regulation, licensing, data residency | "EU data stays in the EU" |
| `operational` | Deployment, monitoring, backup | "Daily backups" |

## Fields

Every `constraint` has an id (the word after `constraint`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Free-form description of the constraint. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `category` | identifier (one bare word) | Classification of the constraint (e.g. performance, security, regulatory). |
| `priority` | enum: critical, high, medium, low | Relative importance of this constraint. |
| `metric` | string | Measurable quantity used to evaluate compliance. |
| `threshold` | string | Acceptable limit or target value for the metric. |
| `refs` | string list | External references (URLs, documents, issue links). |
| `enforced_by` | reference list → `behavior` | Behaviors that enforce this constraint at runtime. |
| `scope` | string | Applicability scope (e.g. system-wide, per-module, per-request). |
| `constrains` | reference list → `behavior` | Behaviors that are limited or governed by this constraint. |
| `protects` | reference list → `invariant` | Invariants that this constraint helps protect. |
| `governs_features` | reference list → `feature` | Product features this constraint governs (requires @specforge/product). |

## Verify kinds

`@specforge/testing` makes `constraint` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `unit`, `integration`, `property`, `load`, `contract`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [behavior](behavior.md) | `ConstraintConstrainsBehavior` | `constrains` on `constraint` | Constraint constrains a behavior |
| outgoing | [behavior](behavior.md) | `ConstraintEnforcedByBehavior` | `enforced_by` on `constraint` | Constraint is enforced by a behavior |
| outgoing | [feature](feature.md) | `ConstraintGovernsFeature` | `governs_features` on `constraint` | Constraint governs a product feature |
| outgoing | [invariant](invariant.md) | `ConstraintProtectsInvariant` | `protects` on `constraint` | Constraint protects an invariant |
| incoming | [decision](decision.md) | `DecisionImposesConstraint` | `constraints` on `decision` | Decision imposes a constraint on the system |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### Good constraints

Constraints should be **measurable** (a check passes or fails against `metric` and `threshold`), **scoped**
(they name what they apply to) and **categorized**.

### Constraint vs. invariant

| Invariant | Constraint |
|-----------|------------|
| Binary: holds or is broken | Measured against a threshold |
| "Email addresses are unique" | "The API responds in under 200ms" |
| Broken means *wrong* | Missed means *degraded* or *non-compliant* |

### Constraint vs. behavior

| Behavior | Constraint |
|----------|------------|
| What the system *does* | *How well* it does it |
| One operation | Cuts across many |

## Examples

### Security

```spec
constraint pii_encryption_constraint "PII Encryption at Rest" {
  description "Personal data is encrypted in the database."
  category    security
  priority    critical
  metric      "Share of PII columns encrypted with AES-256"
  threshold   "100%"

  constrains [create_user, update_email]

  verify integration "column inspection confirms every PII field is encrypted"
}
```

### Regulatory

```spec
constraint gdpr_residency "GDPR Data Residency" {
  description "All EU user data is stored in the EU-West-1 region, with no cross-region replication of PII."
  category    legal
  priority    critical
  refs        ["https://gdpr-info.eu/art-44-gdpr/"]

  governs_features [user_management]
}
```

# invariant

> **Declared by `@specforge/software`**, one of the builtin extensions. `invariant` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/software`. The fields, edges and rules below are read from its declaration.

## Purpose

An `invariant` states a guarantee that must never be broken, whatever the input, load or failure. It answers:
**"What must ALWAYS be true?"**

An invariant is not a feature or a behavior. When one is violated, the system is not degraded or slow: it is
wrong. Behaviors enforce invariants; decisions, constraints and failure modes point at them.

## Syntax

```spec
invariant data_persistence "Data Persistence" {
  guarantee """
    All committed writes survive process restarts.
    No acknowledged write may be silently dropped.
  """
  risk high
}
```

An invariant lists no behaviors. The behaviors that enforce it name it in their `invariants` field
(`invariants [data_persistence]`), and `specforge trace data_persistence` lists them.

## Fields

Every `invariant` has an id (the word after `invariant`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists and the extensions that enhance it add. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `guarantee` | string | The constraint this invariant guarantees holds at all times. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `risk` | enum: critical, high, medium, low | Impact level if this invariant is violated. |
| `description` | string | Human-readable summary of this invariant. |
| `refs` | string list | External references such as issue or document URIs. |
| `expression` | string | Machine-checkable claim the prove pass must entail from the declared bounds (added by `@specforge/formal`). |

## Verify kinds

`@specforge/testing` makes `invariant` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `unit`, `integration`, `property`, `performance`, `mutation`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| incoming | `axiom` | `AxiomAssumesInvariant` | `assumes` on `axiom` | Axiom assumes an invariant as foundational truth |
| incoming | [behavior](behavior.md) | `BehaviorEnforcesInvariant` | `invariants` on `behavior` | Invariants this behavior enforces |
| incoming | [constraint](constraint.md) | `ConstraintProtectsInvariant` | `protects` on `constraint` | Constraint protects an invariant |
| incoming | [decision](decision.md) | `DecisionProtectsInvariant` | `invariants` on `decision` | Decision protects an invariant |
| incoming | [failure_mode](failure-mode.md) | `FailureModeTargetsInvariant` | `invariant` on `failure_mode` | Failure mode targets an invariant |
| incoming | `property` | `PropertyDependsOnInvariant` | no field writes it | Property depends on an invariant |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W003 | warning | `@specforge/software` | Unenforced invariant |
| W004 | warning | `@specforge/testing` | Untested testable entity |
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### Good invariants

Invariants should be:
- **Falsifiable**: you can write a test that would catch a violation
- **Universal**: they hold under all conditions, not only the happy path
- **Implementation-independent**: they say what is true, not how it is achieved

Examples: "No two active users share the same email address"; "All committed writes survive process restarts";
"The audit log is append-only: no entry may be modified or deleted".

### Anti-patterns

- "The system is fast": not measurable; write a `constraint`
- "Users can log in": an operation; write a `behavior`
- "We use PostgreSQL": a choice; write a `decision`

## Examples

### Simple

```spec
invariant email_uniqueness "Email Uniqueness" {
  guarantee "No two active users share the same email address."
  risk medium
}
```

### Audit trail

```spec
invariant audit_integrity "Audit Trail Integrity" {
  guarantee """
    The audit log is append-only.
    No audit entry may be modified or deleted after creation.
    Every state-changing operation MUST produce an audit entry
    before returning success to the caller.
  """
  risk high
  refs [gh.issue:15]
  verify integration "an update writes its audit entry before it returns"
}
```

### Low risk

```spec
invariant display_name_length "Display Name Length" {
  guarantee "User display names MUST be between 1 and 100 characters."
  risk low
}
```

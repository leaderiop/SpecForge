# behavior

> **Declared by `@specforge/software`**, one of the builtin extensions. `behavior` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/software`. The fields, edges and rules below are read from its declaration.

## Purpose

A `behavior` states one thing a system does in one situation, as a contract you can test. It answers:
**"What exactly does the system do?"**

Write the contract with RFC 2119 words (MUST, MUST NOT, SHOULD, MAY) so the obligation level is clear. A
behavior sits between a feature (the value delivered) and an invariant (what must always stay true).

## Syntax

```spec
behavior create_user "Create User" {
  invariants [data_persistence, email_uniqueness]
  category   users

  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with a unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  refs ["gh.issue:42"]

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"
  verify property    "email uniqueness holds under concurrent inserts"
}
```

Tests are linked from the tests themselves, not from the spec: see [Rust test tracing](../guides/rust-test-tracing.md)
and [Vitest test tracing](../guides/vitest-test-tracing.md).

## Fields

Every `behavior` has an id (the word after `behavior`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists and the extensions that enhance it add. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `contract` | string | The behavioral contract this behavior guarantees. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `invariants` | reference list → `invariant` | Invariants this behavior enforces. |
| `types` | reference list → `type` | Type definitions used by this behavior. |
| `ports` | reference list → `port` | Port interfaces this behavior interacts with. |
| `produces` | reference list → `event` | Events produced as a result of this behavior. |
| `consumes` | reference list → `event` | Events this behavior reacts to. |
| `category` | identifier (one bare word) | Classification tag for agent task routing. |
| `features` | reference list → `feature` | Product features this behavior implements. |
| `description` | string | Human-readable summary of this behavior. |
| `status` | identifier (one bare word) | Current lifecycle status of this behavior. |
| `refs` | string list | External references such as issue or document URIs. |
| `severity` | identifier (one bare word) | Impact level if this behavior fails. |
| `diagnostic` | identifier (one bare word) | Diagnostic message for validation tooling. |
| `requires` | block | Named preconditions: each condition's name and what it requires (added by `@specforge/formal`). |
| `ensures` | block | Named postconditions: each condition's name and what it guarantees (added by `@specforge/formal`). |
| `maintains` | block | Named frame conditions that hold throughout execution (added by `@specforge/formal`). |
| `satisfies` | reference list → `property` | Temporal properties this behavior satisfies (added by `@specforge/formal`). |
| `sync` | block | Synchronization bound: barrier, timeout or delivery (added by `@specforge/formal`). |
| `abstract` | bool | Marks a specification-only behavior that concrete behaviors refine; it carries no verify obligations of its own (added by `@specforge/formal`). |
| `refines` | reference → `behavior` | Abstract behavior this concrete behavior refines; its ensures conditions must be kept (E031) (added by `@specforge/formal`). |

## Verify kinds

`@specforge/testing` makes `behavior` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `unit`, `contract`, `integration`, `property`, `performance`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [event](event.md) | `BehaviorConsumesEvent` | `consumes` on `behavior` | Events this behavior reacts to |
| outgoing | [invariant](invariant.md) | `BehaviorEnforcesInvariant` | `invariants` on `behavior` | Invariants this behavior enforces |
| outgoing | [feature](feature.md) | `BehaviorImplementsFeature` | `features` on `behavior` | Behavior implements a feature (cross-extension via peer_dependency @specforge/product) |
| outgoing | [event](event.md) | `BehaviorProducesEvent` | `produces` on `behavior` | Events produced as a result of this behavior |
| outgoing | [type](type.md) | `BehaviorReferencesType` | `types` on `behavior` | Type definitions used by this behavior |
| outgoing | [behavior](behavior.md) | `BehaviorRefinesBehavior` | `refines` on `behavior` | Concrete behavior refines an abstract behavior (specification layering) |
| outgoing | `property` | `BehaviorSatisfiesProperty` | `satisfies` on `behavior` | Behavior satisfies a temporal property |
| outgoing | [port](port.md) | `BehaviorUsesPort` | `ports` on `behavior` | Port interfaces this behavior interacts with |
| incoming | [behavior](behavior.md) | `BehaviorRefinesBehavior` | `refines` on `behavior` | Concrete behavior refines an abstract behavior (specification layering) |
| incoming | [constraint](constraint.md) | `ConstraintConstrainsBehavior` | `constrains` on `constraint` | Constraint constrains a behavior |
| incoming | [constraint](constraint.md) | `ConstraintEnforcedByBehavior` | `enforced_by` on `constraint` | Constraint is enforced by a behavior |
| incoming | [failure_mode](failure-mode.md) | `FailureModeAffectsBehavior` | `affected_behaviors` on `failure_mode` | Failure mode identifies a behavior as a failure vector |
| incoming | [milestone](milestone.md) | `MilestoneIncludesBehavior` | `behaviors` on `milestone` | Milestone delivers a behavior (cross-extension enhancement edge) |
| incoming | `refinement` | `RefinementRefinesAbstract` | `abstract_entity` on `refinement` | Refinement maps from this abstract behavior |
| incoming | `refinement` | `RefinementRefinesConcrete` | `concrete_entity` on `refinement` | Refinement maps to this concrete behavior |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W001 | warning | `@specforge/software` | Behavior implements no feature |
| W004 | warning | `@specforge/testing` | Untested testable entity |
| W006 | warning | `@specforge/software` | Behavior missing category |
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### Writing good contracts

A contract should say:
1. **Trigger**: "When X happens" or "Given Y"
2. **Action**: what the system does
3. **Guarantee**: what MUST be true after the action
4. **Error cases**: what happens when preconditions are not met

Use RFC 2119 words precisely:
- **MUST**: absolute requirement; a violation is a bug
- **MUST NOT**: absolute prohibition
- **SHOULD**: recommended; a deviation needs a reason
- **MAY**: truly optional

### Contract patterns

**Command (state change):**
```
When a valid {Command} is received,
the system MUST {perform action}
and MUST return Result<{Success}, {Error}>.
```

**Query (read):**
```
Given {precondition},
the system MUST return {result type}.
MUST NOT return stale data after a successful write.
```

**Validation (rejection):**
```
When {invalid input} is received,
the system MUST reject the request
and MUST return {ErrorType} with {details}.
```

### Behavior vs. invariant

| Behavior | Invariant |
|----------|-----------|
| Describes an operation | Describes a property |
| Has a trigger (when X, do Y) | Has no trigger (always true) |
| "When a user is created, assign a unique ID" | "No two users share the same ID" |
| Scoped to one operation | Holds across all operations |

### Granularity

Each behavior should describe **one logical operation**. If a behavior has MUST clauses for different
operations, split it. If two behaviors always happen together and cannot be tested apart, merge them.

## Examples

### Simple query

```spec
behavior read_user "Read User by ID" {
  invariants [data_persistence]

  contract """
    Given a valid user ID,
    the system MUST return the user record or UserNotFoundError.
    MUST NOT return stale data after a successful write.
  """

  verify unit "insert then get by ID"
}
```

### Event-producing behavior

```spec
behavior place_order "Place Order" {
  invariants [data_persistence, idempotent_orders]
  produces   [order_placed]

  contract """
    When a valid PlaceOrderCommand is received,
    the system MUST validate inventory availability,
    reserve items, create an order record,
    and MUST emit an OrderPlaced event.
    MUST return Result<Order, InsufficientInventoryError>.
  """

  verify unit        "place order with available items"
  verify unit        "reject order when items unavailable"
  verify integration "concurrent orders for last item: exactly one succeeds"
}
```

### Design by contract (with `@specforge/formal`)

`@specforge/formal` adds `requires`, `ensures` and `maintains` blocks to `behavior`; each line names a
condition and states it. See [Formal verification](../guides/formal-verification.md).

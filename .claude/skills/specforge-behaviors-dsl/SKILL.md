---
name: specforge-behaviors-dsl
description: "Write behavior blocks in .spec DSL files. Each behavior declares a behavioral contract with BEH-{infix}-{n} IDs, a contract statement using RFC 2119 keywords, verify statements (unit/integration/property/load/e2e), and cross-references to invariants, types, ports, and decisions. Use when specifying what exactly the system does in a specific situation."
---

# SpecForge Behaviors DSL

Rules and conventions for authoring **`behavior` blocks** in `.spec` files. Behaviors are the atomic unit of specification — each one describes a single operation with preconditions, guarantees, and verification strategy.

## When to Use

- Specifying what the system does in a specific situation
- Writing testable contracts with RFC 2119 keywords
- Defining verification strategies (unit, integration, property, load, e2e)
- Linking behaviors to invariants, types, ports, and events
- Creating the behaviors that features will compose

## Block Syntax

```spec
use invariants/data
use types/user
use decisions/ADR-001

behavior BEH-MS-001 "Create User" {
  invariants [INV-MS-1, INV-MS-2]
  adrs       [ADR-001]
  types      [User, CreateUserCommand, DuplicateEmailError]
  ports      [UserRepository, EmailService]

  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  refs [gh.issue:42, jira.epic:PROJ-123]

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"
  verify property    "email uniqueness holds under concurrent inserts"

  tests [
    "tests/user_test.go::TestCreateUser",
    "tests/user.test.ts:45",
  ]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `contract` | string / triple-string | Behavioral contract using RFC 2119 keywords (MUST, MUST NOT, SHOULD, MAY). |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `invariants` | reference list | Invariants this behavior depends on or upholds. |
| `adrs` | reference list | Architecture decisions that shaped this behavior (soft ref to governance). |
| `types` | reference list | Type definitions used by this behavior. |
| `ports` | reference list | Port interfaces used by this behavior. |
| `verify` | verify statement(s) | Test specifications: `verify {type} "{description}"`. Multiple allowed. |
| `tests` | string list | Paths to existing test files/functions that exercise this behavior. |
| `refs` | reference list | External references linked to this behavior. |

### Verify Statement Types

| Type | Meaning |
|------|---------|
| `unit` | Isolated test, no external dependencies |
| `integration` | Tests with real dependencies (database, network) |
| `property` | Property-based / fuzz testing |
| `load` | Performance / load testing |
| `e2e` | End-to-end through the full system |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `invariant` | `references` | Behavior depends on these invariants |
| `decision` | `shaped_by` | Behavior shaped by these decisions (soft ref) |
| `type` | `uses_type` | Behavior uses these type definitions |
| `port` | `uses_port` | Behavior uses these port interfaces |
| `event` | `produces` | Behavior emits these events |
| `ref` | `links_to` | External references linked to this behavior |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `feature` | `implements` | Feature includes this behavior |
| `constraint` | `constrains` | Quality requirement applies to this behavior |
| `event` | `consumes` | Event triggers this behavior |

## Writing Rules

1. **One logical operation per behavior** — if it has multiple MUST clauses for different operations, split it.
2. **Use RFC 2119 keywords precisely** — MUST (absolute requirement), MUST NOT (prohibition), SHOULD (recommended), MAY (optional).
3. **Contract pattern**: Trigger ("When X") → Action (what the system does) → Guarantee (what MUST be true after) → Error cases.
4. **Always add at least one `verify` statement** — or `W004` fires.
5. **Reference invariants** — every invariant a behavior depends on should be in the `invariants` list.
6. **Reference types and ports** — name the data shapes and interfaces the behavior uses.
7. **`adrs` is a soft reference** — if `@specforge/governance` is not installed, these emit `I004` info, not errors.
8. **`tests` paths are strings** — they point to actual test files, not entity IDs.
9. **Import required files** — `use` the files that declare referenced invariants, types, and ports.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `invariants`, `adrs`, `types`, `ports` must resolve. |
| E002 | No duplicate behavior IDs across all `.spec` files. |
| W001 | Orphan behavior — not referenced by any feature. |
| W004 | Unverified behavior — no `verify` statement. |

## Examples

### Simple Query

```spec
behavior BEH-MS-002 "Read User by ID" {
  invariants [INV-MS-1]

  contract """
    Given a valid user ID,
    the system MUST return the user record or UserNotFoundError.
    MUST NOT return stale data after a successful write.
  """

  verify unit "insert then get by ID"
}
```

### Validation Behavior

```spec
behavior BEH-MS-003 "Update User Email" {
  invariants [INV-MS-1, INV-MS-2]

  contract """
    When an UpdateEmailCommand is received,
    the system MUST validate the new email is unique
    before committing the change.
    MUST return DuplicateEmailError if the email is already taken.
    MUST NOT modify the user record if validation fails.
  """

  verify unit        "update to unique email succeeds"
  verify unit        "update to taken email fails with DuplicateEmailError"
  verify integration "concurrent updates to same email — exactly one wins"
}
```

### Event-Producing Behavior

```spec
behavior BEH-MS-010 "Place Order" {
  invariants [INV-MS-1, INV-MS-7]

  contract """
    When a valid PlaceOrderCommand is received,
    the system MUST validate inventory availability,
    reserve items, create an order record,
    and MUST emit an OrderPlaced event.
    MUST return Result<Order, InsufficientInventoryError>.
  """

  verify unit        "place order with available items"
  verify unit        "reject order when items unavailable"
  verify integration "concurrent orders for last item — exactly one succeeds"
  verify e2e         "order placed via API triggers fulfillment workflow"

  tests [
    "tests/order.test.ts:12",
    "tests/order_test.go::TestPlaceOrder",
  ]
}
```

## What NOT to Do

- Do not write a behavior for a property ("No two users share the same ID" is an invariant)
- Do not skip `verify` statements — every behavior should describe how to test it
- Do not use `adrs` without understanding it is a soft reference (I004 if governance not installed)
- Do not reference entities from other files without a `use` import
- Do not put implementation details in the `contract` — describe what, not how

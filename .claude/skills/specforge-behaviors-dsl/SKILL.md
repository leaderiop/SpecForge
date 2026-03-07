---
name: specforge-behaviors-dsl
description: "Write behavior blocks in .spec DSL files. Each behavior declares a behavioral contract with free-form snake_case IDs, a contract statement using RFC 2119 keywords, Design-by-Contract blocks (requires/ensures/maintains), verify statements (unit/integration/property/load/e2e/contract/refinement/trace/deadlock_free/liveness/mutation), and cross-references to invariants, types, ports, and events. Use when specifying what exactly the system does in a specific situation."
---

# SpecForge Behaviors DSL

Rules and conventions for authoring **`behavior` blocks** in `.spec` files. Behaviors are the atomic unit of specification -- each one describes a single operation with preconditions, guarantees, and verification strategy.

**Requires:** `@specforge/software` plugin.

## When to Use

- Specifying what the system does in a specific situation
- Writing testable contracts with RFC 2119 keywords
- Defining Design-by-Contract preconditions (requires) and postconditions (ensures)
- Defining verification strategies (unit, integration, property, load, e2e, contract, refinement, trace, mutation)
- Linking behaviors to invariants, types, ports, and events
- Creating abstract behaviors and refinement chains (B-Method)
- Creating the behaviors that features will compose

## Block Syntax

```spec
use invariants/data
use types/user

behavior create_user "Create User" {
  category command
  invariants [data_persistence, email_uniqueness]
  types      [User, CreateUserCommand, DuplicateEmailError]
  ports      [UserRepository, EmailService]

  contract """
    When a valid CreateUserCommand is received,
    the system MUST create a user record with unique email
    and MUST return Result<User, DuplicateEmailError>.
  """

  requires {
    valid_command      "CreateUserCommand passes schema validation"
    email_available    "no active user exists with the same email"
  }

  ensures {
    user_created       "a new User record exists in the datastore"
    email_unique       "email uniqueness invariant holds after insertion"
    event_emitted      "UserCreated event is produced"
  }

  refs [gh.issue:42, jira.epic:PROJ-123]

  verify unit        "insert user, retrieve by ID, assert equal"
  verify integration "insert user, restart process, retrieve persists"
  verify property    "email uniqueness holds under concurrent inserts"
  verify contract    "requires/ensures consistency verified"

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
| `category` | enum | Behavior category: `command`, `query`, `handler`, `saga`, `projection`. |
| `invariants` | reference list | Invariants this behavior depends on or upholds. |
| `types` | reference list | Type definitions used by this behavior. |
| `ports` | reference list | Port interfaces used by this behavior. |
| `produces` | reference list | Events this behavior emits. |
| `consumers` | reference list | Events this behavior consumes. |
| `requires` | block | Named preconditions (Design-by-Contract). Each entry: `name "description"`. |
| `ensures` | block | Named postconditions (Design-by-Contract). Each entry: `name "description"`. |
| `maintains` | block | Frame invariants that must hold before AND after execution. |
| `abstract` | boolean | Marks this as a specification-only behavior (no implementation). |
| `refines` | reference | Reference to an abstract behavior this concrete behavior refines. |
| `verify` | verify statement(s) | Test specifications: `verify {kind} "{description}"`. Multiple allowed. |
| `tests` | string list | Paths to existing test files/functions that exercise this behavior. |
| `gherkin` | string list | Paths to Gherkin (.feature) files for BDD scenarios. |
| `refs` | reference list | External references linked to this behavior. |

### Verify Statement Kinds

| Kind | Meaning |
|------|---------|
| `unit` | Isolated test, no external dependencies |
| `integration` | Tests with real dependencies (database, network) |
| `property` | Property-based / fuzz testing |
| `load` | Performance / load testing |
| `e2e` | End-to-end through the full system |
| `contract` | Design-by-Contract consistency verification |
| `refinement` | Refinement correctness (concrete satisfies abstract) |
| `trace` | Traceability verification |
| `deadlock_free` | Deadlock freedom analysis |
| `liveness` | Liveness property verification |
| `mutation` | Mutation testing |

### Behavior Categories

| Category | Meaning |
|----------|---------|
| `command` | State-changing operation |
| `query` | Read-only operation |
| `handler` | Event handler / message processor |
| `saga` | Multi-step orchestration / compensation flow |
| `projection` | Read model / view builder from event stream |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `invariant` | `references` | Behavior depends on these invariants |
| `type` | `uses_type` | Behavior uses these type definitions |
| `port` | `uses_port` | Behavior uses these port interfaces |
| `event` | `produces` | Behavior emits these events |
| `event` | `consumes` | Behavior reacts to these events |
| `behavior` | `refines` | This behavior refines an abstract behavior |
| `ref` | `links_to` | External references linked to this behavior |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `feature` | `implements` | Feature includes this behavior |
| `constraint` | `constrains` | Quality requirement applies to this behavior |
| `behavior` | `refines` | Another behavior refines this one |

## Writing Rules

1. **One logical operation per behavior** -- if it has multiple MUST clauses for different operations, split it.
2. **Use RFC 2119 keywords precisely** -- MUST (absolute requirement), MUST NOT (prohibition), SHOULD (recommended), MAY (optional).
3. **Contract pattern**: Trigger ("When X") -> Action (what the system does) -> Guarantee (what MUST be true after) -> Error cases.
4. **Always add at least one `verify` statement** -- or `W004` fires.
5. **Reference invariants** -- every invariant a behavior depends on should be in the `invariants` list.
6. **Reference types and ports** -- name the data shapes and interfaces the behavior uses.
7. **`tests` paths are strings** -- they point to actual test files, not entity IDs.
8. **Import required files** -- `use` the files that declare referenced invariants, types, and ports.
9. **Use `requires`/`ensures` for formal contracts** -- named preconditions and postconditions enable automated consistency checking.
10. **Use `abstract`/`refines` for B-Method refinement** -- abstract behaviors are specification-only; concrete behaviors must satisfy the abstract's postconditions.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `invariants`, `types`, `ports`, `produces`, `consumers`, `refines` must resolve. |
| E002 | No duplicate behavior IDs across all `.spec` files. |
| E030 | Always-false precondition in `requires` block. |
| E031 | Liskov violation -- precondition strengthening or postcondition weakening in refinement. |
| W001 | Orphan behavior -- not referenced by any feature. |
| W004 | Unverified behavior -- no `verify` statement. |
| W037 | Unverifiable contract condition -- ambiguous or references external state. |
| W038 | Unreachable postcondition -- contradicts preconditions. |
| W039 | Redundant precondition -- implied by another precondition. |

## Examples

### Simple Query

```spec
behavior read_user_by_id "Read User by ID" {
  category query
  invariants [data_persistence]

  contract """
    Given a valid user ID,
    the system MUST return the user record or UserNotFoundError.
    MUST NOT return stale data after a successful write.
  """

  verify unit "insert then get by ID"
}
```

### Behavior with Design-by-Contract

```spec
behavior update_user_email "Update User Email" {
  category command
  invariants [data_persistence, email_uniqueness]
  types      [User, UpdateEmailCommand, DuplicateEmailError]
  ports      [UserRepository]

  contract """
    When an UpdateEmailCommand is received,
    the system MUST validate the new email is unique
    before committing the change.
    MUST return DuplicateEmailError if the email is already taken.
    MUST NOT modify the user record if validation fails.
  """

  requires {
    user_exists        "target user exists in the datastore"
    email_valid        "new email passes format validation"
  }

  ensures {
    email_updated      "user record reflects the new email"
    old_email_released "old email is available for other users"
    uniqueness_held    "email uniqueness invariant holds"
  }

  verify unit        "update to unique email succeeds"
  verify unit        "update to taken email fails with DuplicateEmailError"
  verify integration "concurrent updates to same email -- exactly one wins"
  verify contract    "requires/ensures block consistency"
}
```

### Abstract Behavior with Refinement

```spec
behavior process_payment "Process Payment" {
  abstract true

  contract """
    The system MUST process a payment and return a receipt
    or a PaymentFailedError.
  """

  requires {
    valid_amount       "payment amount is positive"
    valid_method       "payment method is supported"
  }

  ensures {
    receipt_returned   "a valid receipt is returned on success"
    idempotent         "retrying the same payment ID is safe"
  }
}

behavior process_stripe_payment "Process Stripe Payment" {
  category command
  refines process_payment
  ports   [StripeGateway]

  contract """
    The system MUST process a payment through the Stripe API.
  """

  verify unit        "successful Stripe charge returns receipt"
  verify unit        "Stripe decline returns PaymentFailedError"
  verify refinement  "concrete satisfies abstract postconditions"
}
```

### Event-Producing Behavior

```spec
behavior place_order "Place Order" {
  category command
  invariants [data_persistence, inventory_accuracy]
  produces   [order_placed]

  contract """
    When a valid PlaceOrderCommand is received,
    the system MUST validate inventory availability,
    reserve items, create an order record,
    and MUST emit an OrderPlaced event.
    MUST return Result<Order, InsufficientInventoryError>.
  """

  requires {
    items_in_stock     "all order items have sufficient inventory"
  }

  ensures {
    order_created      "order record persisted with correct total"
    inventory_reserved "inventory decremented for all items"
    event_emitted      "OrderPlaced event emitted with order details"
  }

  verify unit        "place order with available items"
  verify unit        "reject order when items unavailable"
  verify integration "concurrent orders for last item -- exactly one succeeds"
  verify e2e         "order placed via API triggers fulfillment workflow"

  tests [
    "tests/order.test.ts:12",
    "tests/order_test.go::TestPlaceOrder",
  ]
}
```

## What NOT to Do

- Do not write a behavior for a property ("No two users share the same ID" is an invariant)
- Do not skip `verify` statements -- every behavior should describe how to test it
- Do not reference entities from other files without a `use` import
- Do not put implementation details in the `contract` -- describe what, not how
- Do not strengthen preconditions or weaken postconditions in a refinement (Liskov violation -- E031)

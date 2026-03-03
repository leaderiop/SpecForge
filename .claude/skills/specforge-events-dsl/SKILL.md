---
name: specforge-events-dsl
description: "Write event blocks in .spec DSL files. Each event declares a domain or system event with EVT-{infix}-{n} IDs, a trigger behavior, payload shape, channel name, and consumer behaviors. Use when making the system's reactive behavior explicit and traceable across bounded contexts."
---

# SpecForge Events DSL

Rules and conventions for authoring **`event` blocks** in `.spec` files. Events make the system's reactive behavior explicit — a behavior emits an event, and other behaviors consume it.

## When to Use

- Declaring what the system announces when a state change occurs
- Defining event payload contracts between producers and consumers
- Tracing reactive chains across bounded contexts or services
- Linking behaviors via publish-subscribe relationships

## Block Syntax

```spec
use types/user
use behaviors/user-crud

event EVT-MS-001 "User Created" {
  trigger   BEH-MS-001
  channel   "users.created"

  payload {
    userId    string
    email     string
    role      UserRole
    timestamp timestamp
  }

  consumers [BEH-NOTIF-001, BEH-AUDIT-001]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `trigger` | reference | The behavior that produces this event. Must reference an existing behavior. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `payload` | block | Event data shape — fields with types. Defines the contract between producer and consumers. |
| `channel` | string | Logical channel, topic, or queue name. Used for documentation and AsyncAPI generation. |
| `consumers` | reference list | Behaviors that react to this event. Enables cross-context traceability. |
| `refs` | reference list | External references linked to this event. |

### Payload Fields

Follow the same syntax as `type` fields:

```spec
payload {
  fieldName  fieldType
  fieldName  fieldType  @optional
}
```

## Relationships

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `behavior` | `produces` | Behavior emits this event (via `trigger`) |

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `behavior` | `consumes` | Event is consumed by these behaviors |
| `ref` | `links_to` | External references linked to this event |

## Writing Rules

1. **Past tense names** — "UserCreated", "OrderPlaced", "PaymentProcessed" — events describe what happened.
2. **One trigger per event** — an event is produced by exactly one behavior.
3. **Payload carries sufficient data** — consumers should not need to call back to the producer.
4. **Include timestamps** — event payloads should include when the event occurred.
5. **Import trigger and consumer files** — `use` the files that declare referenced behaviors.
6. **Empty consumers is OK during development** — `W007` warning will remind you to add them later.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | `trigger` must resolve to an existing behavior. |
| E001 | Every ID in `consumers` must resolve to an existing behavior. |
| E002 | No duplicate event IDs across all `.spec` files. |
| E006 | Trigger behavior must be valid. |
| W007 | Orphan event — no consumers. |

## Examples

### Simple Event

```spec
event EVT-MS-001 "User Created" {
  trigger   BEH-MS-001
  channel   "users.created"

  payload {
    userId    string
    email     string
    role      UserRole
    timestamp timestamp
  }

  consumers [BEH-NOTIF-001, BEH-AUDIT-001]
}
```

### Event Without Consumers

```spec
event EVT-MS-002 "User Email Changed" {
  trigger   BEH-MS-003
  channel   "users.email-changed"

  payload {
    userId    string
    oldEmail  string
    newEmail  string
    timestamp timestamp
  }

  // No consumers yet — W007 warning is fine during early development.
}
```

### Cross-Service Event

```spec
use behaviors/order-processing

event EVT-MS-010 "Order Placed" {
  trigger   BEH-MS-050
  channel   "orders.placed"

  payload {
    orderId     string
    customerId  string
    items       OrderItem[]
    totalAmount number
    currency    string
    timestamp   timestamp
  }

  consumers [
    BEH-FULFILL-001,   // Fulfillment service
    BEH-BILLING-010,   // Billing service
    BEH-NOTIF-005,     // Notification service
    BEH-AUDIT-003,     // Audit service
  ]
}
```

## What NOT to Do

- Do not name events as commands ("SendWelcomeEmail" — that is a behavior, not an event)
- Do not use future tense ("UserWillBeCreated" — events describe what happened)
- Do not create events without a trigger behavior
- Do not put the entire entity in the payload — include only what consumers need
- Do not reference behaviors from other files without a `use` import

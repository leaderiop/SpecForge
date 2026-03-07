---
name: specforge-events-dsl
description: "Write event blocks in .spec DSL files. Each event declares a domain or system event with free-form snake_case IDs, a trigger behavior, payload type reference, channel name, consumer behaviors, sync blocks for CSP concurrency, and verify statements (integration/deadlock_free/liveness). Use when making the system's reactive behavior explicit and traceable across bounded contexts."
---

# SpecForge Events DSL

Rules and conventions for authoring **`event` blocks** in `.spec` files. Events make the system's reactive behavior explicit -- a behavior emits an event, and other behaviors consume it.

**Requires:** `@specforge/software` plugin.

## When to Use

- Declaring what the system announces when a state change occurs
- Defining event payload contracts between producers and consumers (via type references)
- Tracing reactive chains across bounded contexts or services
- Linking behaviors via publish-subscribe relationships
- Defining CSP synchronization constraints with `sync` blocks

## Block Syntax

```spec
use types/user
use behaviors/user-crud

event user_created "User Created" {
  trigger   create_user
  channel   "users.created"
  payload   UserCreatedPayload

  consumers [send_welcome_email, log_audit_entry]

  verify integration "event emitted after user creation"
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
| `payload` | reference | Type reference for the event data shape (e.g., `payload UserCreatedPayload`). |
| `channel` | string | Logical channel, topic, or queue name. |
| `consumers` | reference list | Behaviors that react to this event. Enables cross-context traceability. |
| `sync` | block | CSP synchronization constraints: `barrier` (behavior references) and `timeout` (duration). |
| `verify` | verify statement(s) | Test specifications: `verify {kind} "{description}"`. Kinds: integration, deadlock_free, liveness. |
| `refs` | reference list | External references linked to this event. |

### Sync Block

The `sync` block defines CSP (Communicating Sequential Processes) synchronization constraints:

```spec
sync {
  barrier [behavior_a, behavior_b, behavior_c]
  timeout 30s "all sub-analyses must complete within 30 seconds"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `barrier` | reference list | Behaviors that must all complete before this event fires. |
| `timeout` | string | Duration with description -- maximum wait time for barrier completion. |

### Verify Kinds for Events

| Kind | Meaning |
|------|---------|
| `integration` | Event emitted correctly with proper payload |
| `deadlock_free` | No circular dependency between event participants |
| `liveness` | Event processing eventually completes |

## Relationships

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `behavior` | `produces` | Behavior emits this event (via `trigger`) |

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `behavior` | `consumes` | Event is consumed by these behaviors |
| `type` | `uses_type` | Event carries data shaped like this type (via `payload`) |
| `ref` | `links_to` | External references linked to this event |

## Writing Rules

1. **Past tense names** -- "user_created", "order_placed", "payment_processed" -- events describe what happened.
2. **One trigger per event** -- an event is produced by exactly one behavior.
3. **Payload is a type reference** -- use `payload UserCreatedPayload`, not inline field declarations. Define the payload shape as a `type` block.
4. **Use `sync` for concurrency constraints** -- when multiple behaviors must complete before the event fires.
5. **Import trigger and consumer files** -- `use` the files that declare referenced behaviors.
6. **Empty consumers is OK during development** -- `W029` warning will remind you to add them later.
7. **Add verify statements** -- events are testable entities (integration, deadlock_free, liveness).

## Validation Rules

| Code | Rule |
|------|------|
| E001 | `trigger` must resolve to an existing behavior. |
| E001 | Every ID in `consumers` must resolve to an existing behavior. |
| E002 | No duplicate event IDs across all `.spec` files. |
| E006 | Trigger behavior must be valid. |
| E034 | Circular event dependency detected (deadlock). |
| E035 | Channel type mismatch -- producer and consumer disagree on payload type. |
| W007 | Orphan event -- no consumers. |
| W029 | Unmatched producers -- event has producers but no consumers. |

## Examples

### Simple Event

```spec
event user_created "User Created" {
  trigger   create_user
  channel   "users.created"
  payload   UserCreatedPayload

  consumers [send_welcome_email, log_audit_entry]

  verify integration "event emitted after user creation"
}
```

### Event Without Consumers

```spec
event user_email_changed "User Email Changed" {
  trigger   update_user_email
  channel   "users.email-changed"
  payload   EmailChangedPayload

  // No consumers yet -- W029 warning is fine during early development.

  verify integration "event emitted after email update"
}
```

### Event with Sync Block

```spec
event concurrency_analysis_complete "Concurrency Analysis Complete" {
  trigger   process_analyze
  channel   "analysis.concurrency"
  payload   ConcurrencyAnalysisReport

  sync {
    barrier [detect_deadlocks, detect_channel_mismatch, detect_unmatched_producers, detect_livelock_risk]
    timeout 30s "all concurrency sub-analyses must complete within 30 seconds"
  }

  verify integration "event emitted after process analyze pass completes"
  verify deadlock_free "no circular dependency between concurrency sub-analyses"
}
```

### Cross-Service Event

```spec
use behaviors/order-processing

event order_placed "Order Placed" {
  trigger   place_order
  channel   "orders.placed"
  payload   OrderPlacedPayload

  consumers [
    start_fulfillment,       // Fulfillment service
    process_billing,         // Billing service
    send_order_confirmation, // Notification service
    log_order_audit,         // Audit service
  ]

  verify integration "event emitted with correct order details"
  verify liveness "all consumers eventually process the event"
}
```

## What NOT to Do

- Do not name events as commands ("send_welcome_email" -- that is a behavior, not an event)
- Do not use future tense ("user_will_be_created" -- events describe what happened)
- Do not create events without a trigger behavior
- Do not define payload fields inline -- use a type reference (`payload MyPayloadType`)
- Do not reference behaviors from other files without a `use` import
- Do not ignore sync constraints for events with multiple concurrent dependencies

# event

> **Declared by `@specforge/software`**, one of the builtin extensions. `event` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/software`. The fields, edges and rules below are read from its declaration.

## Purpose

An `event` is something the system announces when a significant state change happens. It answers:
**"What does the system announce?"**

Events make the reactive links explicit. Without them, the link between "user created" and "welcome email
sent" is hidden in code. With them, the graph holds the chain: a behavior produces the event, other behaviors
consume it, and `specforge trace` follows it.

## Syntax

```spec
type UserCreatedPayload "User Created Payload" {
  userId    string
  email     string
  timestamp timestamp
}

event user_created "User Created" {
  channel  "users.created"
  payload  UserCreatedPayload
  category users
}
```

An event lists no behaviors. The behaviors name it, in `produces` (they emit it) and `consumes` (they react to
it). Its data shape is a [type](type.md), named in `payload`.

## Fields

Every `event` has an id (the word after `event`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists and the extensions that enhance it add. A field no loaded extension declares is reported (W020).

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `channel` | string | Communication channel this event is published on. |
| `channel_type` | identifier (one bare word) | Transport mechanism for the channel (e.g. queue, topic, stream). |
| `category` | identifier (one bare word) | Classification of this event (e.g. domain, integration, system). |
| `payload` | reference → `type` | Type definition describing this event's data shape. |
| `description` | string | Human-readable summary of this event. |
| `refs` | string list | External references such as issue or document URIs. |
| `contract` | string | Delivery or ordering guarantees for this event. |
| `follows_protocol` | reference list → `protocol` | Synchronization protocol this event follows (added by `@specforge/formal`). |
| `participates_in` | reference list → `process` | Processes this event participates in (added by `@specforge/formal`). |
| `sync` | block | Synchronization bound: barrier, timeout or delivery (added by `@specforge/formal`). |

## Verify kinds

`@specforge/testing` makes `event` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `integration`, `unit`, `deadlock_free`, `liveness`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [type](type.md) | `EventCarriesPayloadType` | `payload` on `event` | Type definition describing this event's data shape |
| outgoing | `protocol` | `EventFollowsProtocol` | `follows_protocol` on `event` | Event follows a synchronization protocol |
| outgoing | `process` | `EventParticipatesInProcess` | `participates_in` on `event` | Event participates in a process |
| incoming | [behavior](behavior.md) | `BehaviorConsumesEvent` | `consumes` on `behavior` | Events this behavior reacts to |
| incoming | [behavior](behavior.md) | `BehaviorProducesEvent` | `produces` on `behavior` | Events produced as a result of this behavior |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W004 | warning | `@specforge/testing` | Untested testable entity |
| W007 | warning | `@specforge/software` | Event never produced |
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### Good events

Events should:
- **Describe what happened, not what to do**: "UserCreated", not "SendWelcomeEmail"
- **Be past tense**: "OrderPlaced", "PaymentProcessed", "SessionExpired"
- **Carry enough data**: consumers should not need to call back to the producer
- **Be immutable**: once published, the payload never changes

### Event vs. behavior

| Event | Behavior |
|-------|----------|
| Describes what *happened* | Describes what the system *does* |
| Past tense ("UserCreated") | Imperative ("Create User") |
| Published after a state change | Defines the operation that causes it |
| Consumed by behaviors | Implements features |

### Payload design

A payload type should hold the identity of the affected entity (`userId`), the data consumers need without a
round trip, and a timestamp. Keep it minimal but sufficient.

## Examples

### Event chain

```spec
type OrderPlacedPayload "Order Placed Payload" {
  orderId     string
  customerId  string
  totalAmount number
  timestamp   timestamp
}

event order_placed "Order Placed" {
  channel "orders.placed"
  payload OrderPlacedPayload
}

event payment_processed "Payment Processed" {
  channel "billing.payment-processed"
}

behavior charge_payment "Charge Payment" {
  consumes [order_placed]
  produces [payment_processed]
  contract "When an order is placed, the system MUST charge the customer's payment method once."
  verify integration "one order_placed yields exactly one charge"
}
```

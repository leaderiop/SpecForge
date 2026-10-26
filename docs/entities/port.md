# port

> **Declared by `@specforge/software`**, one of the builtin extensions. `port` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/software`. The fields, edges and rules below are read from its declaration.

## Purpose

A `port` states an interface contract: the operations a component offers or needs. Ports mark the boundary
between the domain and the outside world (databases, APIs, message brokers, external services). In hexagonal
architecture they are the sockets adapters plug into. It answers: **"What contracts exist between components?"**

## Syntax

```spec
type User "User" {
  id    string @readonly
  email string
}

type CreateUserCommand "Create User Command" {
  email string
}

type DuplicateEmailError "Duplicate Email Error" {
  email string
}

type UserNotFoundError "User Not Found Error" {
  id string
}

port UserRepository "User Repository" {
  direction outbound
  category  "persistence/user"
  types     [User]

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method findById(id: string) -> Result<User, UserNotFoundError>
  method findByEmail(email: string) -> Result<Option<User>, never>
  method delete(id: string) -> Result<void, UserNotFoundError>
}
```

### Methods

A port's body holds `method name(param: Type, ...) -> ReturnType` lines. Every type a parameter or return names
must be a declared [type](type.md) or a primitive (E004). Generic arguments (`Result<A, B>`, `Option<T>`) and lists
(`T[]`) are checked inside. `never` marks an error channel that cannot happen.

### Direction

| Direction | Meaning | Examples |
|-----------|---------|----------|
| `inbound` | The outside world calls in; the port is what the system offers ("driving" port) | `UserAPI`, `WebhookHandler` |
| `outbound` | The system calls out; the port is what the system needs ("driven" port) | `UserRepository`, `PaymentGateway` |
| `bidirectional` | Calls go both ways | a socket session |

## Fields

Every `port` has an id (the word after `port`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `direction` | enum: inbound, outbound, bidirectional | Whether this port is inbound, outbound, or bidirectional. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `category` | string | Classification of this port, a word or a path (e.g. http, io/filesystem). |
| `methods` | block | Method signatures exposed by this port interface. |
| `types` | reference list → `type` | Types named in this port's method signatures (derived from its methods). |
| `description` | string | Human-readable summary of this port. |
| `refs` | string list | External references such as issue or document URIs. |

## Verify kinds

`@specforge/testing` makes `port` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `integration`, `unit`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [type](type.md) | `PortReferencesType` | `types` on `port` | Port names a type in a method parameter or return type |
| incoming | [behavior](behavior.md) | `BehaviorUsesPort` | `ports` on `behavior` | Port interfaces this behavior interacts with |
| incoming | [module](module.md) | `ModuleConsumesPort` | `ports` on `module` | Port interfaces this module consumes |
| incoming | [module](module.md) | `ModuleDefinesPort` | `ports_defined` on `module` | Port interfaces this module defines |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| E004 | error | `@specforge/software` | Unknown type in port method |
| W004 | warning | `@specforge/testing` | Untested testable entity |
| W005 | warning | `@specforge/software` | Unreferenced port |
| W009 | warning | `@specforge/testing` | Disallowed verify kind |


## Design guidance

### Naming

| Pattern | Convention | Examples |
|---------|-----------|----------|
| Repository | `{Entity}Repository` | `UserRepository`, `OrderRepository` |
| External service | `{Service}Service` or `{Service}Gateway` | `EmailService`, `PaymentGateway` |
| API surface | `{Domain}API` | `UserAPI`, `SearchAPI` |
| Event bus | `{Domain}EventBus` | `OrderEventBus` |

### Result types

Return `Result<Success, Error>` rather than throwing: the error cases become part of the contract.

### Port vs. behavior

| Port | Behavior |
|------|----------|
| The interface contract (signature) | The behavioral contract (meaning) |
| "This method exists with this signature" | "When this method is called, the system MUST do X" |
| Type signatures only | MUST/SHOULD/MAY obligations |

The port defines the shape; the behaviors that list it in `ports` define the rules.

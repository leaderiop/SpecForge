---
name: specforge-ports-dsl
description: "Write port interface blocks in .spec DSL files. Each port declares an interface contract with direction (inbound/outbound), method signatures with Result types, and category grouping. Use when defining boundaries between the domain and external systems in hexagonal architecture."
---

# SpecForge Ports DSL

Rules and conventions for authoring **`port` blocks** in `.spec` files. Ports define the boundaries in hexagonal architecture — the "holes" that adapters plug into. They generate language-specific interfaces.

## When to Use

- Defining interface contracts between domain and external systems
- Specifying method signatures with typed errors (Result types)
- Distinguishing inbound (driving) and outbound (driven) ports
- Creating interfaces for code generation (TypeScript, Python, Go)

## Block Syntax

```spec
use types/user

port UserRepository {
  direction outbound
  category  "persistence/user"

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method findById(id: string) -> Result<User, UserNotFoundError>
  method findByEmail(email: string) -> Result<User?, never>
  method update(id: string, cmd: UpdateUserCommand) -> Result<User, DuplicateEmailError | UserNotFoundError>
  method delete(id: string) -> Result<void, UserNotFoundError>
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `name` | identifier | Port name (identifier after `port`). |
| `direction` | enum | `inbound` or `outbound`. |
| `methods` | method list | One or more method signatures. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `category` | string | Organizational category (e.g., `"persistence/user"`, `"external/payment"`). |
| `refs` | reference list | External references linked to this port. |

### Direction

| Direction | Meaning | Examples |
|-----------|---------|----------|
| `inbound` | Outside world calls into the system (driving port) | `UserAPI`, `WebhookHandler` |
| `outbound` | System calls out to the world (driven port) | `UserRepository`, `EmailService` |

### Method Signatures

```ebnf
method_decl = "method" , identifier , "(" , [param_list] , ")" , "->" , return_type ;
```

Common return patterns:
- `Result<Success, Error>` — operation that can fail
- `Result<Success, ErrorA | ErrorB>` — multiple error types
- `Result<Type?, never>` — nullable return, cannot fail
- `Result<void, Error>` — no return value, can fail

## Relationships

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `behavior` | `uses_port` | Behavior uses this port interface |
| `library` | `defines_port` | Library defines this port interface |
| `invariant` | `enforces` | Invariant enforced by this port's implementation |

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `ref` | `links_to` | External references linked to this port |

## Writing Rules

1. **PascalCase names recommended** — by convention, ports typically use PascalCase (e.g., `UserRepository`, `EmailService`, `PaymentGateway`), but any valid identifier is accepted.
2. **Naming conventions** — `{Entity}Repository` for persistence, `{Service}Service` or `{Service}Gateway` for external, `{Domain}API` for inbound.
3. **All methods return Result types** — explicit error handling, no thrown exceptions.
4. **Import types used in signatures** — `use types/user` before referencing `User`, `CreateUserCommand`.
5. **Choose direction carefully** — inbound = system offers, outbound = system requires.
6. **One port per concern** — don't mix persistence and messaging in the same port.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | No two ports may share the same identifier. |
| E001 | Every type name in method signatures must resolve to a declared type or primitive. |

## Examples

### Outbound: Database

```spec
use types/user

port UserRepository {
  direction outbound
  category  "persistence/user"

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method findById(id: string) -> Result<User, UserNotFoundError>
  method findByEmail(email: string) -> Result<User?, never>
  method update(id: string, cmd: UpdateUserCommand) -> Result<User, DuplicateEmailError | UserNotFoundError>
  method delete(id: string) -> Result<void, UserNotFoundError>
}
```

### Outbound: External Service

```spec
use types/email

port EmailService {
  direction outbound
  category  "external/email"

  method send(to: string, template: EmailTemplate, data: EmailData) -> Result<void, EmailDeliveryError>
  method sendBulk(recipients: BulkEmailRequest) -> Result<BulkEmailResult, EmailDeliveryError>
}
```

### Inbound: API Surface

```spec
use types/user

port UserAPI {
  direction inbound
  category  "api/user"

  method createUser(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
  method getUser(id: string) -> Result<User, UserNotFoundError>
  method listUsers(query: UserQuery) -> Result<UserPage, never>
}
```

## What NOT to Do

- Do not use `PREFIX-INFIX-N` IDs for ports — ports use plain PascalCase identifiers
- Do not define behavioral contracts in ports — ports define signatures, behaviors define semantics
- Do not mix inbound and outbound concerns in a single port
- Do not throw exceptions in method signatures — always use Result types
- Do not forget to import types from other files used in method signatures

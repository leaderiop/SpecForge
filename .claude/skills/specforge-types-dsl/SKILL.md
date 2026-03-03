---
name: specforge-types-dsl
description: "Write type definitions in .spec DSL files. Supports three syntax variants: struct types with fields and annotations (@readonly, @unique, @literal, @optional), union types (type X = a | b | c), and error types with _tag pattern. Use when defining data shapes for domain entities, commands, errors, and value objects."
---

# SpecForge Types DSL

Rules and conventions for authoring **`type` blocks** in `.spec` files. Types define the data vocabulary — the shape of domain entities, commands, errors, and value objects. They drive code generation for TypeScript interfaces, Python dataclasses, Go structs, and JSON Schema.

## When to Use

- Defining domain entity shapes (User, Order, Product)
- Defining command types (CreateUserCommand, PlaceOrderCommand)
- Defining error types with discriminated unions (DuplicateEmailError)
- Defining union/enum types (UserRole, OrderStatus)
- Defining value objects (EmailAddress, Money, DateRange)

## Block Syntax

### Struct Type

```spec
type User {
  id        string      @readonly
  email     string      @unique
  name      string
  role      UserRole
  createdAt timestamp   @readonly
  updatedAt timestamp   @readonly
}
```

### Union Type (Enum)

```spec
type UserRole = admin | editor | viewer
```

### Error Type (Tagged)

```spec
type DuplicateEmailError {
  _tag    "DuplicateEmailError"   @literal
  email   string
  message string
}
```

### Command Type

```spec
type CreateUserCommand {
  email  string
  name   string
  role   UserRole
}
```

## Fields Reference

### Type-Level

| Field | Type | Description |
|-------|------|-------------|
| `name` | identifier | The type name (identifier after `type`). |
| `fields` | field list | For struct types: fields with types and annotations. |
| `variants` | identifier list | For union types: `= a \| b \| c`. |
| `refs` | reference list | External references linked to this type. |

### Field-Level

| Part | Description |
|------|-------------|
| `name` | Field identifier. |
| `type` | Primitive, another type name, or array (`TypeName[]`). |
| `annotations` | Zero or more: `@readonly`, `@unique`, `@literal`, `@optional`. |

### Field Annotations

| Annotation | Meaning | Code Generation |
|------------|---------|-----------------|
| `@readonly` | Set at creation, never modified | TS: `readonly`. Python: frozen. Go: no setter. |
| `@unique` | Value unique across all instances | Generates unique constraint hints. |
| `@literal` | Fixed literal value (for tagged unions) | TS: literal type. Python: `Literal["..."]`. |
| `@optional` | Field may be absent | TS: `?`. Python: `Optional`. Go: pointer. |

### Primitive Types

| Type | Meaning |
|------|---------|
| `string` | Text |
| `number` | Numeric value |
| `integer` | Integer value |
| `boolean` | True/false |
| `timestamp` | Date/time |
| `void` | No value (return types only) |

## Relationships

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `behavior` | `uses_type` | Behavior uses this type definition |
| `event` | payload reference | Event carries data shaped like this type |
| `port` | method signatures | Port uses this type in method signatures |

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `ref` | `links_to` | External references linked to this type |

## Writing Rules

1. **PascalCase names recommended** — by convention, types typically use PascalCase (e.g., `User`, `CreateUserCommand`, `DuplicateEmailError`), but any valid identifier is accepted.
2. **Suffix conventions** — `*Command` for commands, `*Error` for errors, no suffix for domain entities.
3. **Use `_tag` + `@literal` for discriminated unions** — enables type-safe error handling.
4. **Use `@readonly` for identity and audit fields** — `id`, `createdAt`, `updatedAt`.
5. **Use `@unique` for business uniqueness constraints** — `email`, `slug`.
6. **Union types for finite sets** — `type UserRole = admin | editor | viewer`.
7. **Array fields use `TypeName[]`** — `items OrderItem[]`.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | No two types may share the same identifier. |
| E001 | Every type name used in a field type must resolve to a declared type or be a primitive. |

## Examples

### Domain Entity

```spec
type Order {
  id         string      @readonly
  customerId string
  items      OrderItem[]
  status     OrderStatus
  total      number
  currency   string
  createdAt  timestamp   @readonly
  updatedAt  timestamp   @readonly
}
```

### Value Object

```spec
type OrderItem {
  productId  string
  name       string
  quantity   integer
  unitPrice  number
  total      number
}
```

### Union / Enum

```spec
type OrderStatus = pending | confirmed | shipped | delivered | cancelled
```

### Error with Tag

```spec
type InsufficientInventoryError {
  _tag       "InsufficientInventoryError"   @literal
  productId  string
  requested  integer
  available  integer
  message    string
}
```

### Command with Nested Type

```spec
type PlaceOrderCommand {
  customerId string
  items      OrderItemInput[]
  currency   string
}

type OrderItemInput {
  productId  string
  quantity   integer
}
```

## What NOT to Do

- Do not use `PREFIX-INFIX-N` IDs for types — types use plain PascalCase identifiers
- Do not define operations on types — use `port` blocks for methods
- Do not mix type definitions with behavior contracts — types are data shapes, behaviors are operations
- Do not forget to import types referenced in field definitions from other files
- Do not use lowercase for type names — they must be PascalCase to map to code

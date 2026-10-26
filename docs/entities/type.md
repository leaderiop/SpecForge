# type

> **Declared by `@specforge/software`**, one of the builtin extensions. `type` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/software`. The fields, edges and rules below are read from its declaration.

## Purpose

A `type` states the shape of some data: a domain record, a command, an error, a value object. It answers:
**"What shape does the data have?"**

Types are the data vocabulary of a spec. Behaviors, events and ports name them, so a change to a shape shows up
everywhere it is used.

## Syntax

A struct type lists its members, one per line: a name, a type, and annotations.

```spec
type User "User" {
  id        string    @readonly
  email     string    @unique
  name      string
  role      UserRole
  createdAt timestamp @readonly
}
```

A union type lists its values after `=`; its title comes before the `=`.

```spec
type UserRole "User Role" = admin | editor | viewer
```

The `type` kind accepts open fields: any member name is valid in its body. The declared fields below
(`extends`, `composed_types`, ...) are the ones the extension gives a meaning to.

### Member annotations

| Annotation | Meaning |
|------------|---------|
| `@readonly` | Set at creation and never modified |
| `@unique` | Unique across all instances |
| `@literal` | A fixed literal value (for tagged unions) |
| `@optional` | May be absent |

Any other annotation is W010.

### Primitive types

Besides declared types, a member or a port method may use `string`, `number`, `integer`, `boolean`, `timestamp`,
`date`, `void`, `never`, `any`, the Rust-style primitives (`bool`, `i8`…`i64`, `u8`…`u64`, `f32`, `f64`, `usize`,
`isize`) and the containers `Result`, `Option`, `Vec`, `Box`, `Arc`, `Rc`, `HashMap`, `HashSet`, `BTreeMap`,
`BTreeSet`, `String`. `TypeName[]` is a list.

## Fields

Every `type` has an id (the word after `type`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `composed_types` | reference list → `type` | Types composed or referenced by this type (derived from its field types). |
| `extends` | reference → `type` | Parent type this type extends or inherits from. |
| `description` | string | Human-readable summary of this type. |
| `refs` | string list | External references such as issue or document URIs. |

## Verify kinds

`@specforge/testing` makes `type` testable: write `verify <kind> "..."` lines in its body. The kinds it accepts are `unit`, `property`. A bare `verify "..."` has no kind. A kind outside this list is W009.

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [type](type.md) | `TypeComposesType` | `composed_types` on `type` | Types composed or referenced by this type (derived from its field types) |
| outgoing | [type](type.md) | `TypeExtendsType` | `extends` on `type` | Parent type this type extends or inherits from |
| incoming | [behavior](behavior.md) | `BehaviorReferencesType` | `types` on `behavior` | Type definitions used by this behavior |
| incoming | [event](event.md) | `EventCarriesPayloadType` | `payload` on `event` | Type definition describing this event's data shape |
| incoming | [port](port.md) | `PortReferencesType` | `types` on `port` | Port names a type in a method parameter or return type |
| incoming | [type](type.md) | `TypeComposesType` | `composed_types` on `type` | Types composed or referenced by this type (derived from its field types) |
| incoming | [type](type.md) | `TypeExtendsType` | `extends` on `type` | Parent type this type extends or inherits from |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| W002 | warning | `@specforge/software` | Unreferenced type |
| W004 | warning | `@specforge/testing` | Untested testable entity |
| W009 | warning | `@specforge/testing` | Disallowed verify kind |
| W010 | warning | `@specforge/software` | Unknown field annotation |


## Design guidance

### Naming

| Kind | Convention | Examples |
|------|-----------|----------|
| Domain record | PascalCase noun | `User`, `Order`, `Product` |
| Command | `Command` suffix | `CreateUserCommand`, `PlaceOrderCommand` |
| Error | `Error` suffix | `DuplicateEmailError`, `UserNotFoundError` |
| Enum | PascalCase | `UserRole`, `OrderStatus` |
| Value object | PascalCase noun | `EmailAddress`, `Money`, `DateRange` |

Any valid identifier is accepted; PascalCase is a convention.

### Tagged unions for errors

Use a `_tag` member with `@literal` so each error is told apart by a fixed value:

```spec
type DuplicateEmailError "Duplicate Email Error" {
  _tag    "DuplicateEmailError" @literal
  email   string
  message string
}
```

### Type vs. port

| Type | Port |
|------|------|
| Data shape (what the data looks like) | Interface contract (what operations exist) |
| `User { id, email, name }` | `UserRepository { create(), findById() }` |
| Passive | Defines operations |

## Examples

### Domain record and value object

```spec
type OrderItem "Order Item" {
  productId string
  quantity  integer
  unitPrice number
}

type Order "Order" {
  id         string      @readonly
  customerId string
  items      OrderItem[]
  status     OrderStatus
  total      number
  createdAt  timestamp   @readonly
}

type OrderStatus "Order Status" = pending | confirmed | shipped | delivered | cancelled
```

### Command

```spec
type PlaceOrderCommand "Place Order Command" {
  customerId string
  items      OrderItem[]
  currency   string
}
```

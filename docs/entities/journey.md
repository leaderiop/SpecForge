# journey

> **Declared by `@specforge/product`**, one of the builtin extensions. `journey` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `journey` is one path a real person takes: a persona, on some channels, using some features, step by step.
It answers: **"How does the user experience this?"**

A feature says *what* is delivered; a journey says *how* one kind of person meets it. The same feature can surface
as several journeys: an admin creates users in a web dashboard, an API consumer through a REST endpoint, a
customer at a shop counter. The `flow` steps are prose on purpose: a journey documents the experience; the
checked links are its persona, channels and features.

## Syntax

```spec
persona admin "Administrator" {
  description "Manages accounts and permissions"
}

channel web "Web App" {
  description "Browser-based application"
}

journey create_user_web "Create a New User" {
  persona  admin
  channels [web]
  features [user_management]

  flow [
    "Admin opens the user management page",
    "Clicks \"New User\"",
    "Fills the form (name, email, role)",
    "Submits; the system checks the email is unique",
    "Success: the user appears in the list",
    "Failure: an inline error on the email field",
  ]
}
```

## Fields

Every `journey` has an id (the word after `journey`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `flow` | string list | Ordered steps the user takes through the journey. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `persona` | reference → `persona` | The user archetype who undertakes this journey. |
| `description` | string | Human-readable summary of the journey. |
| `channels` | reference list → `channel` | Communication or distribution channels used in this journey. |
| `features` | reference list → `feature` | Features exercised during this journey. |
| `priority` | enum: critical, high, medium, low | Importance level. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [feature](feature.md) | `JourneyExercisesFeature` | `features` on `journey` | Features exercised during this journey |
| outgoing | [persona](persona.md) | `JourneyTargetsPersona` | `persona` on `journey` | The user archetype who undertakes this journey |
| outgoing | [channel](channel.md) | `JourneyUsesChannel` | `channels` on `journey` | Communication or distribution channels used in this journey |
| incoming | [deliverable](deliverable.md) | `DeliverableSupportsJourney` | `journeys` on `deliverable` | User journeys this deliverable supports |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I050 | info | `@specforge/product` | Journey with empty flow |
| I054 | info | `@specforge/product` | Journey without persona |
| I055 | info | `@specforge/product` | Journey without channels |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| W042 | warning | `@specforge/product` | Unreferenced journey |


## Design guidance

### Naming

Name a journey as the user's action: "Create a New User" (not "User Creation"), "Export Monthly Report" (not
"Report Export").

### Writing flows

A flow should alternate user actions and system responses, cover both success and failure, and be concrete
enough to sketch a screen or an API sequence from it.

### Journey vs. feature

| Journey | Feature |
|---------|---------|
| User's view | Product's view |
| "Admin creates a new user in the web dashboard" | "User Management" |
| Persona, channels, steps | Problem, solution, acceptance |
| Exercises one or more features | Serves many journeys |

## Examples

### API journey

```spec
journey create_user_api "Create User via API" {
  persona  api_consumer
  channels [api]
  features [user_management]
  priority high

  flow [
    "Consumer sends POST /api/v1/users with a JSON body",
    "System validates the body and the email's uniqueness",
    "Success: 201 Created with the user object",
    "Duplicate email: 409 Conflict with an error message",
  ]
}
```

### Outside software

```spec
journey redeem_points_in_store "Redeem Points in Store" {
  persona  loyal_customer
  channels [shop_counter]
  features [loyalty_programme]

  flow [
    "Customer shows the loyalty card at the till",
    "Cashier scans it; the balance appears",
    "Customer chooses how many points to spend",
    "Receipt shows points used and the new balance",
  ]
}
```

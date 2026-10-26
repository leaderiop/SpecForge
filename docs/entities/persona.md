# persona

> **Declared by `@specforge/product`**, one of the builtin extensions. `persona` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `persona` is a kind of person who uses what you build: a role, not an individual. It answers:
**"Who uses this?"**

Journeys name the persona who takes them, and a persona can name the features it cares most about
(`key_features`). Because personas are entities, the graph can answer "which features reach this persona?"
(`specforge product persona-features`) and flag a persona no journey serves (I046).

## Syntax

```spec
persona admin "System Administrator" {
  description """
    A user with full system access who manages accounts,
    configures settings and monitors operational health.
  """
  technical_level expert
  goals       ["manage user accounts", "configure system settings"]
  pain_points ["permission changes take effect too slowly"]
  tags        ["internal"]
}
```

## Fields

Every `persona` has an id (the word after `persona`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of this persona. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `technical_level` | enum: expert, advanced, intermediate, beginner, non_technical | Technical proficiency of this persona. |
| `goals` | string list | What this persona wants to achieve with the product. |
| `pain_points` | string list | Frustrations or problems this persona currently faces. |
| `status` | enum: active, deprecated | Lifecycle state; default `active`. |
| `reason` | string | Justification for the current status or a status change. |
| `key_features` | reference list → `feature` | Features this persona cares about most. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [feature](feature.md) | `PersonaPrioritizesFeature` | `key_features` on `persona` | Persona prioritizes a feature |
| incoming | [journey](journey.md) | `JourneyTargetsPersona` | `persona` on `journey` | The user archetype who undertakes this journey |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I046 | info | `@specforge/product` | Unreferenced persona |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I069 | info | `@specforge/product` | Deprecated persona missing reason |


## Design guidance

### When to declare a persona

Declare one when a distinct type of person takes journeys, when a role has different permissions, goals or
skills, or when several journeys share the same kind of user.

### Granularity

One persona per distinct role (`admin`, `viewer`, `operator`), never per individual (`alice`, `bob`). Two roles
with the same goals and permissions are one persona.

### Technical level

| Level | Meaning |
|-------|---------|
| `expert` | Power user, comfortable with CLIs, APIs, advanced features |
| `advanced` | Technically proficient, prefers efficient workflows |
| `intermediate` | Knows the domain, needs clear guidance |
| `beginner` | New to the product, needs onboarding |
| `non_technical` | No technical background, needs a fully guided experience |

## Examples

### Deprecated

```spec
persona legacy_admin "Legacy Admin" {
  description "Former admin role before the RBAC migration."
  technical_level expert
  status deprecated
  reason "Replaced by role-based personas (admin, operator) in v2"
}
```

### Outside software

```spec
persona patient "Healthcare Patient" {
  description "A person receiving care, who books appointments and reads their records."
  technical_level beginner
  goals ["schedule appointments", "view medical records"]
}

persona loyal_customer "Loyal Customer" {
  description "Shops at least monthly and holds a loyalty card."
  technical_level non_technical
  goals       ["get rewarded for repeat purchases"]
  key_features [loyalty_programme]
}
```

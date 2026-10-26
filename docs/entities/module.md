# module

> **Declared by `@specforge/product`**, one of the builtin extensions. `module` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `module` is a structural unit that builds features: a code package, a hardware subsystem, a consulting
workstream, a programme office. It answers: **"What component delivers this?"**

A module lists the features it builds and the modules it depends on. The dependencies form a graph that must
have no cycle (E007).

## Syntax

```spec
module core_mod "@myservice/core" {
  description "Core platform component handling user management and profiles"
  family      core
  features    [user_management, user_profile]
  depends_on  [utils_mod]
}
```

With `@specforge/software` loaded, a module can also list the ports it consumes (`ports`) and defines
(`ports_defined`).

### Families

`family` is one bare word. Five are standard; any other is accepted with an I062 note.

| Value | Meaning |
|-------|---------|
| `core` | Fundamental library or framework component |
| `platform` | User-facing binary (CLI, server, app) |
| `extension` | Plugin or extension package |
| `integration` | Integration adapter or bridge |
| `advisory` | Non-code module (docs, process, governance) |

## Fields

Every `module` has an id (the word after `module`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists and the extensions that enhance it add. A field no loaded extension declares is reported (W020).

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `family` | identifier (one bare word) | Category or family this module belongs to; the standard ones are core, platform, extension, integration, advisory. |
| `description` | string | Human-readable summary of the module. |
| `features` | reference list → `feature` | Features contained within this module. |
| `depends_on` | reference list → `module` | Other modules this module depends on. |
| `reason` | string | Justification for this module's existence or grouping. |
| `ports` | reference list → `port` | Port interfaces this module consumes (added by `@specforge/software`). |
| `ports_defined` | reference list → `port` | Port interfaces this module defines (added by `@specforge/software`). |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [port](port.md) | `ModuleConsumesPort` | `ports` on `module` | Port interfaces this module consumes |
| outgoing | [feature](feature.md) | `ModuleContainsFeature` | `features` on `module` | Features contained within this module |
| outgoing | [port](port.md) | `ModuleDefinesPort` | `ports_defined` on `module` | Port interfaces this module defines |
| outgoing | [module](module.md) | `ModuleDependsOn` | `depends_on` on `module` | Other modules this module depends on |
| incoming | [deliverable](deliverable.md) | `DeliverableContainsModule` | `modules` on `deliverable` | Modules included in this deliverable |
| incoming | [milestone](milestone.md) | `MilestoneScopesModule` | `modules` on `milestone` | Modules scoped to this milestone |
| incoming | [module](module.md) | `ModuleDependsOn` | `depends_on` on `module` | Other modules this module depends on |
| incoming | [term](term.md) | `TermBelongsToModule` | `module` on `term` | Term is defined within a module's bounded context |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| E007 | error | `@specforge/product` | Module dependency cycle |
| I062 | info | `@specforge/product` | Non-standard module family |
| I067 | info | `@specforge/product` | Module without features |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| W044 | warning | `@specforge/product` | Unreferenced module |


## Design guidance

### Granularity

A module should map to a **real unit** of the organisation (a package, a subsystem, a workstream), be
**cohesive** (a related set of features) and **declare its dependencies**.

### Module vs. feature

| Module | Feature |
|--------|---------|
| "Core Auth Package" / "Mechanical Subsystem" | "Password Authentication" / "Load Bearing" |
| A structural unit | A unit of value |
| Has dependencies | Has a problem and a solution |

### Module vs. port

| Module | Port |
|--------|------|
| "Core Auth Package" | "UserRepository" |
| The component that builds features | The interface contract itself |

### Scope

A module records links: the features it builds and the modules it depends on. Package names, part numbers and
budget codes belong in other tools.

## Examples

### Integration module (software)

```spec
module email_mod "@myservice/email" {
  description "Email delivery integration layer"
  family      integration
  features    [email_notifications]
  depends_on  [core_mod]
}
```

### Hardware subsystem

```spec
module mechanical_subsystem "Mechanical Subsystem" {
  description "Primary structural and damping components"
  family      mechanical
  features    [load_bearing, vibration_damping]
  depends_on  [electronics_subsystem]
}
```

### Consulting workstream

```spec
module advisory_workstream "Advisory Workstream" {
  description "Client-facing advisory and policy services"
  family      advisory
  features    [stakeholder_alignment, policy_drafting]
}
```

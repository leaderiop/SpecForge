# module

> **Module:** `@specforge/product`

## Purpose

A `module` declares a **structural component** that maps features to deliverables. Modules represent the organizational units that implement features — whether code packages, hardware subsystems, consulting workstreams, or program offices. They form a dependency DAG that the compiler validates for cycles.

It answers: **"What component delivers this?"**

Modules bridge the gap between abstract features and concrete organizational structure. A module groups the features it implements and declares its dependencies, making the relationship between spec-level concepts and implementation units explicit and compiler-checked.

## ID Pattern

```
identifier
```

Examples: `core_mod`, `email_mod`, `search_mod`, `mechanical_subsystem`, `advisory_workstream`

## Syntax

```spec
use "features/user-management"
use "features/auth"

module core_mod "@myservice/core" {
  description  "Core platform component handling user management and profiles"
  family       core
  features     [user_management, user_profile]
  depends_on   [utils_mod]
}
```

## Fields

All fields are optional at the type level. The compiler emits info diagnostics (I067) when a module has no features.

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). |
| `description` | string | Brief description of the module's purpose and scope. |
| `family` | ModuleFamily | Logical grouping from the `ModuleFamily` enum: `core`, `platform`, `extension`, `integration`, `advisory`. Non-standard values produce I062 with fuzzy-match suggestion. |
| `features` | EntityId[] | The features this module implements. Creates `ModuleFeature` edges. Omission emits I067. |
| `depends_on` | EntityId[] | Other modules this module depends on. Creates `ModuleDependsOn` edges. Cycles detected by E007. |
| `reason` | string | Rationale for module state (e.g., deprecation justification). Documentation-only — no validation rule in v1. |
| `tags` | string[] | Free-form tags for categorization. Format validated by I068 (lowercase hyphen-separated, 2-50 chars). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this module. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `ModuleFeature` | "This module implements these features" |
| `module` | `ModuleDependsOn` | "This module depends on that module" |
| `ref` | `links_to` | "This module links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `module` | `ModuleDependsOn` | "Another module depends on this one" |
| `deliverable` | `DeliverableModule` | "A deliverable uses this module" |
| `milestone` | `MilestoneModule` | "A milestone includes this module" |

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| E007 | error | Circular module dependency — `depends_on` edges between modules form a cycle. |
| W044 | warning | Module not referenced by any deliverable or milestone (orphan module). |
| I062 | info | Non-standard `family` value (not in `ModuleFamily` enum: `core`, `platform`, `extension`, `integration`, `advisory`). Includes fuzzy-match suggestion. |
| I067 | info | Module with no features. |

## ModuleFamily Enum

The `family` field uses an open enum with five standard values. Non-standard values are valid but produce an I062 info diagnostic.

| Value | Meaning |
|-------|---------|
| `core` | Fundamental library or framework component |
| `platform` | User-facing binary (CLI, LSP, MCP server) |
| `extension` | Plugin or extension package |
| `integration` | Integration adapter or bridge |
| `advisory` | Non-code module (docs, process, governance) |

## Design Guidance

### Module Granularity

A module should be:
- **Deliverable** — maps to a concrete organizational unit (code package, hardware subsystem, consulting workstream, program office)
- **Cohesive** — implements a related set of features
- **Dependency-aware** — explicitly declares its module dependencies

### Module vs. Feature

| Module | Feature |
|--------|---------|
| "Core Auth Package" / "Mechanical Subsystem" | "Password Authentication" / "Load Bearing" |
| Structural organization unit | User value unit |
| Has dependencies and description | Has behaviors and problem/solution |
| Maps to a deliverable component | Maps to a milestone item |

### Module vs. Port

| Module | Port |
|--------|------|
| "Core Auth Package" / "Advisory Workstream" | "UserRepository" / "ClientIntakePort" |
| The component that implements features | The interface contract itself |
| Has a dependency DAG | Has a direction (inbound/outbound) |

### DSL Scope

The module block models references and relationships — which features it implements and which other modules it depends on. Concrete implementation details (npm names, part numbers, budget codes) belong in external tooling or a `specforge verify` plugin, not in the DSL.

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `ModuleFeature` | Features this module implements |
| outgoing | [module](module.md) | `ModuleDependsOn` | Modules this module depends on |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this module |
| incoming | [module](module.md) | `ModuleDependsOn` | Modules that depend on this one |
| incoming | [deliverable](deliverable.md) | `DeliverableModule` | Deliverables that use this module |
| incoming | [milestone](milestone.md) | `MilestoneModule` | Milestones that include this module |

## Examples

### Platform Module (Software)

```spec
use "features/user-management"
use "features/auth"

module core_mod "@myservice/core" {
  description  "Core platform services for user and auth management"
  family       platform
  features     [user_management, password_auth]
  depends_on   [search_mod]
  refs         [gh.pr:187]
}
```

### Integration Module (Software)

```spec
use "features/email-notifications"

module email_mod "@myservice/email" {
  description  "Email delivery integration layer"
  family       integration
  features     [email_notifications]
  depends_on   [core_mod]
}
```

### Hardware Subsystem

```spec
use "features/load-bearing"
use "features/vibration-damping"

module mechanical_subsystem "Mechanical Subsystem" {
  description  "Primary structural and damping components"
  family       mechanical
  features     [load_bearing, vibration_damping]
  depends_on   [electronics_subsystem]
}
```

### Consulting Workstream

```spec
use "features/stakeholder-alignment"
use "features/policy-drafting"

module advisory_workstream "Advisory Workstream" {
  description  "Client-facing advisory and policy services"
  family       advisory
  features     [stakeholder_alignment, policy_drafting]
  depends_on   [research_workstream]
}
```

### Minimal Module

```spec
use "features/search"

module search_mod "@myservice/search" {
  features     [full_text_search]
}
```

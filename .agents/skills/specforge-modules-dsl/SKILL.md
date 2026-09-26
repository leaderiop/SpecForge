---
name: specforge-modules-dsl
description: "Write module blocks in .spec DSL files (@specforge/product plugin). Each module declares a structural component with free-form snake_case IDs, mapping features to dependencies with a validated dependency DAG. Use when making the relationship between components and spec-level features explicit."
---

# SpecForge Modules DSL

Rules and conventions for authoring **`module` blocks** in `.spec` files. Modules represent structural components -- they map features to dependencies and form a dependency DAG the compiler validates.

**Requires:** `@specforge/product` plugin.

## When to Use

- Mapping structural components to the features they implement
- Establishing module-to-module dependencies (validated DAG)
- Connecting the implementation layer to the product layer for deliverable coverage validation
- Describing what component delivers a set of features

## Block Syntax

```spec
use "features/user-management"
use "features/auth"

module core_mod "@myservice/core" {
  family       core
  features     [user_management, password_authentication]
  depends_on   [crypto_mod]
  description  "Core domain logic and user management"
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `features` | reference list | Features this module implements. Omission emits I067. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `depends_on` | reference list | Other modules this module depends on. Validated DAG (no cycles). |
| `description` | string @optional | Free-text explanation of the module's purpose. |
| `family` | ModuleFamily @optional | Logical grouping: `core`, `platform`, `extension`, `integration`, `advisory`. Non-standard values produce I062 with fuzzy-match suggestion. |
| `reason` | string @optional | Rationale for module state (e.g., deprecation justification). Documentation-only -- no validation rule in v1. |
| `tags` | string[] @optional | Faceted filtering tags. |
| `refs` | reference list | External references linked to this module. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `ModuleFeature` | Module implements these features |
| `module` | `ModuleDependsOn` | Module depends on these modules |
| `ref` | `links_to` | External references linked to this module |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `module` | `ModuleDependsOn` | Other modules depend on this one |
| `deliverable` | `DeliverableModule` | Deliverable uses this module |
| `milestone` | `MilestoneModule` | Milestone includes this module |

**Note:** If `@specforge/software` is installed, modules gain `ports_defined` (reference list -> port) via entity_enhancement.

## Writing Rules

1. **Maps to structural components** -- each module corresponds to a package, subsystem, workstream, or program area.
2. **Features are what it implements** -- the spec-level features this component delivers.
3. **`depends_on` forms a DAG** -- circular module dependencies are a compile error (E007).
4. **Import feature files** -- `use` the files declaring referenced entities.
5. **DSL scope is references, not implementation details** -- internal structure belongs in external tooling.

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| E007 | error | Circular module dependency -- `depends_on` edges must form a DAG. |
| W044 | warning | Orphan module -- not referenced by any deliverable or milestone. |
| I062 | info | Non-standard `family` value (not in ModuleFamily enum). Includes fuzzy-match suggestion. |
| I067 | info | Module with no features. |

## Examples

### Software Module

```spec
module core_mod "@myservice/core" {
  family       platform
  features     [user_management, password_authentication]
  depends_on   [crypto_mod]
  description  "Core domain logic and authentication"
  refs         [gh.pr:187]
}
```

### Hardware Subsystem

```spec
module power_subsystem "Power Distribution Unit" {
  family       mechanical
  features     [power_regulation, thermal_protection]
  depends_on   [sensor_array]
  description  "Manages power distribution and voltage regulation across all subsystems"
}
```

### Consulting Workstream

```spec
module market_analysis "Market Analysis Workstream" {
  family       advisory
  features     [competitive_landscape, market_sizing]
  description  "Research and analysis of target market segments"
}
```

### Program Delivery Module

```spec
module community_outreach "Community Outreach Program" {
  family       program_delivery
  features     [volunteer_coordination, event_planning]
  depends_on   [communications_mod]
  description  "Coordinates community engagement and volunteer activities"
}
```

### Minimal Module

```spec
module search_mod "@myservice/search" {
  features     [full_text_search]
}
```

## What NOT to Do

- Do not write modules without the `@specforge/product` plugin installed
- Do not create circular dependencies between modules -- E007 error
- Do not confuse modules (structural components) with deliverables (shippable artifacts)
- Do not put implementation-specific details (version, file paths) in module blocks -- use external tooling
- Do not reference features or modules from other files without `use` imports

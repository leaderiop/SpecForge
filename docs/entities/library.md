# library

> **Module:** `@specforge/product`

## Purpose

A `library` declares a **code package** that maps features to ports. Libraries represent the structural code units that implement features and define the port interfaces through which they interact with external systems. They form a dependency DAG that the compiler validates for cycles.

It answers: **"What code package implements this?"**

Libraries bridge the gap between abstract features and concrete code organization. A library groups the features it implements and the ports it defines, making the relationship between spec-level concepts and code-level packages explicit and compiler-checked.

## ID Pattern

```
identifier
```

Examples: `core_lib`, `email_lib`, `search_lib`

## Syntax

```spec
use features/user-management
use features/auth

library core_lib "@myservice/core" {
  family       core
  features     [user_management, user_profile]
  depends_on   [utils_lib]
  ports_defined [UserRepository, EmailService]
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `features` | reference list | The features this library implements. Every feature referenced must exist. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). Typically the package name. |
| `depends_on` | reference list | Other libraries this library depends on. Forms a DAG validated for cycles. |
| `ports_defined` | reference list | Port interfaces defined by this library. |
| `family` | enum or string | Logical grouping (e.g., `core`, `platform`, `plugin`, `integration`). Used for organizational clarity. |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this library. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `provides` | "This library provides the code for these features" |
| `library` | `depends_on` | "This library depends on that library" |
| `port` | `defines_port` | "This library defines this port interface" |
| `ref` | `links_to` | "This library links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `library` | `depends_on` | "Another library depends on this one" |
| `deliverable` | `built_from` | "A deliverable uses this library" |

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` must resolve to an existing `feature`. |
| E001 | Every ID in `depends_on` must resolve to an existing `library`. |
| E001 | Every ID in `ports_defined` must resolve to an existing `port`. |
| E002 | No two libraries may share the same ID. |
| E007 | **No circular library dependencies** — `depends_on` edges between libraries must form a DAG. |
| W009 | If no `deliverable` references this library, emit "orphan library" warning. |

## Design Guidance

### Library Granularity

A library should be:
- **Deployable** — maps to an actual code package (npm package, Go module, Rust crate, Python package)
- **Cohesive** — implements a related set of features and defines related ports
- **Dependency-aware** — explicitly declares its library dependencies

### Library vs. Feature

| Library | Feature |
|---------|---------|
| "Core Auth Package" | "Password Authentication" |
| Code organization unit | User value unit |
| Has dependencies and ports | Has behaviors and problem/solution |
| Maps to a real package | Maps to a roadmap item |

### Library vs. Port

| Library | Port |
|---------|------|
| "Core Auth Package" | "UserRepository" |
| The package that defines and implements | The interface contract itself |
| Groups multiple ports | A single interface with methods |
| Has a dependency DAG | Has a direction (inbound/outbound) |

### DSL Scope

The library block models references and relationships — which features it implements, which ports it defines, which other libraries it depends on. Concrete package details (`npm_name`, `package.json` path, version numbers) belong in external tooling or a `specforge verify` plugin, not in the DSL.

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `provides` | Features this library implements |
| outgoing | [library](library.md) | `depends_on` | Libraries this library depends on |
| outgoing | [port](port.md) | `defines_port` | Port interfaces this library defines |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this library |
| incoming | [deliverable](deliverable.md) | `built_from` | Deliverables that use this library |
| incoming | [library](library.md) | `depends_on` | Libraries that depend on this one |

## Examples

### Platform Library

```spec
use features/user-management
use features/auth

library core_lib "@myservice/core" {
  family       platform
  features     [user_management, password_auth]
  depends_on   [search_lib]
  ports_defined [UserRepository, TokenService]
  refs         [gh.pr:187]
}
```

### Integration Library

```spec
use features/email-notifications

library email_lib "@myservice/email" {
  family       integration
  features     [email_notifications]
  depends_on   [core_lib]
  ports_defined [EmailService]
}
```

### Minimal Library

```spec
use features/search

library search_lib "@myservice/search" {
  features     [full_text_search]
  ports_defined [SearchIndex]
}
```

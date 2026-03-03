# deliverable

> **Module:** `@specforge/product`

## Purpose

A `deliverable` declares a **shippable artifact** that maps capabilities to libraries. Deliverables represent what actually ships to users — apps, services, CLIs, extensions — bridging the UX layer (capabilities) to the code layer (libraries).

It answers: **"What ships to users?"**

Deliverables complete the top of the traceability chain: `deliverable -> capability -> feature -> behavior -> invariant`. They make release planning explicit by bundling capabilities into named artifacts with clear library dependencies.

## ID Pattern

```
identifier
```

Examples: `web_dashboard`, `cli_tool`, `rest_api`

## Syntax

```spec
use capabilities/admin-users
use capabilities/developer-api
use libraries/core
use libraries/email

deliverable user_mgmt_mvp "User Management MVP" {
  type         app
  personas     [admin, developer]
  capabilities [create_user_web, create_user_api]
  libraries    [core_lib, email_lib]
  roadmap      mvp_phase
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `capabilities` | reference list | The UX capabilities this deliverable ships. Every capability referenced must exist. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). |
| `libraries` | reference list | The libraries this deliverable is built from. |
| `roadmap` | reference | The roadmap phase this deliverable is planned for. |
| `personas` | identifier list | The personas this deliverable targets. Validated against `persona` definitions in the `spec` root. |
| `type` | enum or string | The artifact type (e.g., `app`, `service`, `cli`, `extension`, `library`, `webapp`). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this deliverable. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `capability` | `bundles` | "This deliverable ships these capabilities" |
| `library` | `built_from` | "This deliverable uses these libraries" |
| `ref` | `links_to` | "This deliverable links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `roadmap` | `schedules` | "A roadmap phase schedules this deliverable" |

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `capabilities` must resolve to an existing `capability`. |
| E001 | Every ID in `libraries` must resolve to an existing `library`. |
| E002 | No two deliverables may share the same ID. |
| W008 | **Uncovered capability** — every capability in a deliverable should be reachable through its library chain. If a capability traces to features that are not implemented by any of the deliverable's libraries, emit this warning. |

## Design Guidance

### Deliverable Granularity

A deliverable should be:
- **Shippable** — something you can deploy, publish, or release to users
- **Self-contained** — bundles all capabilities needed for its use case
- **Persona-scoped** — targets specific user personas

### Deliverable vs. Capability

| Deliverable | Capability |
|-------------|------------|
| "Web Dashboard" | "Create a New User" |
| What ships | What a user can do |
| Groups capabilities by artifact | Groups features by persona + surface |
| Has library dependencies | Has feature dependencies |

### Deliverable vs. Library

| Deliverable | Library |
|-------------|---------|
| "User Management MVP" | "@myservice/core" |
| What users see | What developers build |
| Maps capabilities to libraries | Maps features to ports |
| Shippable artifact | Code package |

### Coverage Validation

The W008 warning validates that every capability in a deliverable is reachable through its library chain. The chain works like this:

```
deliverable.capabilities → capability.features → feature.behaviors
deliverable.libraries → library.features → same feature set

If any capability's features are not covered by the deliverable's libraries, W008 fires.
```

This catches cases where a deliverable claims to ship a capability but doesn't include the library that implements it.

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [capability](capability.md) | `bundles` | Capabilities this deliverable ships |
| outgoing | [library](library.md) | `built_from` | Libraries this deliverable uses |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this deliverable |
| incoming | [roadmap](roadmap.md) | `schedules` | Roadmap phases that schedule this deliverable |

## Examples

### Web Application

```spec
use capabilities/admin-users
use capabilities/admin-reports
use libraries/core
use libraries/reporting

deliverable web_dashboard "Web Dashboard" {
  type         webapp
  personas     [admin]
  capabilities [create_user_web, create_user_api, export_reports]
  libraries    [core_lib, reporting_lib]
  roadmap      mvp_phase
  refs         [jira.epic:PROJ-100]
}
```

### CLI Tool

```spec
use capabilities/developer-api
use libraries/core
use libraries/cli-utils

deliverable cli_tool "CLI Tool" {
  type         cli
  personas     [developer]
  capabilities [search_records_cli, lint_specs_cli]
  libraries    [core_lib, cli_utils_lib]
  roadmap      search_analytics
}
```

### Minimal Deliverable

```spec
use capabilities/api-integration

deliverable rest_api "REST API Service" {
  capabilities [api_integration]
}
```

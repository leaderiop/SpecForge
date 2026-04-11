# deliverable

> **Module:** `@specforge/product`

## Purpose

A `deliverable` declares a **shippable artifact** that maps journeys to modules. Deliverables represent what actually ships to users — apps, services, CLIs, extensions — bridging the UX layer (journeys) to the code layer (modules).

It answers: **"What ships to users?"**

Deliverables complete the top of the traceability chain: `deliverable -> journey -> feature -> behavior -> invariant`. They make release planning explicit by bundling journeys into named artifacts with clear module dependencies.

## ID Pattern

```
identifier
```

Examples: `web_dashboard`, `cli_tool`, `rest_api`

## Syntax

```spec
use "journeys/admin-users"
use "journeys/developer-api"
use "modules/core"
use "modules/email"
use "product/milestones"

deliverable user_mgmt_mvp "User Management MVP" {
  artifact_type cli
  status        in_progress
  journeys      [create_user_web, create_user_api]
  modules       [core_mod, email_mod]
  milestones    [mvp_phase]
  version       "0.1.0"
}
```

## Fields

All fields are optional at the type level. The compiler emits warnings (W043, W046) when key structural fields are absent.

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). |
| `artifact_type` | ArtifactType | The artifact kind: `cli`, `service`, `library`, `web_app`, `mobile_app`, `api`, `extension`, `documentation`, `package`. Validated by W081. |
| `status` | DeliverableStatus | Lifecycle state: `draft`, `in_progress`, `shipped`, `deprecated`. Validated by W085. |
| `journeys` | EntityId[] | The UX journeys this deliverable ships. Creates `DeliverableJourney` edges. Omission emits W043. |
| `modules` | EntityId[] | The modules this deliverable is built from. Creates `DeliverableModule` edges. Omission emits W046. |
| `milestones` | EntityId[] | Milestones this deliverable is tracked against. Creates `DeliverableMilestone` edges. |
| `depends_on` | EntityId[] | Other deliverables this one depends on. Creates `DeliverableDependsOn` edges. Cycles detected by E016. |
| `version` | string | Semantic Versioning 2.0.0 string (e.g., `1.0.0`, `1.0.0-alpha.1`, `1.0.0+build.42`). Format validated by I061. |
| `reason` | string | Rationale for current status (required context for `deprecated` status, checked by I066). |
| `tags` | string[] | Free-form tags for categorization. Format validated by I068 (lowercase hyphen-separated, 2-50 chars). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this deliverable. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `journey` | `DeliverableJourney` | "This deliverable ships these journeys" |
| `module` | `DeliverableModule` | "This deliverable uses these modules" |
| `milestone` | `DeliverableMilestone` | "This deliverable is tracked against these milestones" |
| `deliverable` | `DeliverableDependsOn` | "This deliverable depends on that deliverable" |
| `ref` | `links_to` | "This deliverable links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `deliverable` | `DeliverableDependsOn` | "Another deliverable depends on this one" |

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| E016 | error | Circular deliverable dependency — `depends_on` edges form a cycle. |
| W043 | warning | Deliverable with no journeys. |
| W046 | warning | Deliverable with no modules. |
| W085 | warning | Invalid `status` value (not in DeliverableStatus enum). |
| I049 | info | Journey features not a subset of module features (traceability gap). |
| I061 | info | Invalid `version` format (not Semantic Versioning 2.0.0). Accepts `MAJOR.MINOR.PATCH`, pre-release tags, and build metadata. |
| I065 | info | Shipped deliverable has incomplete milestones (not all completed). |
| I066 | info | Deprecated deliverable without a `reason`. |
| I073 | info | Deliverable references a journey that uses a deprecated persona (transitive deprecation). |
| I074 | info | Deliverable references a journey that uses a deprecated channel (transitive deprecation). |

## Queries

| Query | Method | Description |
|-------|--------|-------------|
| Deliverable traceability | `queryDeliverableTraceability` | Transitive features via journey and module paths. |
| Deliverable completion | `queryDeliverableCompletion` | Aggregate milestone completion across referenced milestones. |
| Deliverable dependents | `queryDeliverableDependents` | Other deliverables that depend on this one. |
| Deliverable priority | `queryDeliverablePriority` | Derived priority from constituent milestones and journeys (max-priority). |
| Deliverable personas | `queryDeliverablePersonas` | Deduplicated personas served by this deliverable, traversed via deliverable→journey→persona edges. |

## Design Guidance

### Deliverable Granularity

A deliverable should be:
- **Shippable** — something you can deploy, publish, or release to users
- **Self-contained** — bundles all journeys needed for its use case
- **Persona-scoped** — targets specific user personas

### Status Lifecycle

```
draft -> in_progress -> shipped
                |
            deprecated (requires reason)
```

When `status` is `shipped`, the compiler checks that all referenced milestones are `completed` (I065). When `status` is `deprecated`, omitting `reason` emits I066.

### Deliverable vs. Journey

| Deliverable | Journey |
|-------------|---------|
| "Web Dashboard" | "Create a New User" |
| What ships | What a user can do |
| Groups journeys by artifact | Groups features by persona + channel |
| Has module dependencies | Has feature dependencies |

### Deliverable vs. Module

| Deliverable | Module |
|-------------|--------|
| "User Management MVP" / "Housing Policy Framework" | "@myservice/core" / "Advisory Workstream" |
| What users or stakeholders receive | What implementers build |
| Maps journeys to modules | Maps features to dependencies |
| Shippable artifact or outcome | Structural component |

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [journey](journey.md) | `DeliverableJourney` | Journeys this deliverable ships |
| outgoing | [module](module.md) | `DeliverableModule` | Modules this deliverable uses |
| outgoing | [milestone](milestone.md) | `DeliverableMilestone` | Milestones this deliverable tracks |
| outgoing | [deliverable](deliverable.md) | `DeliverableDependsOn` | Deliverables this one depends on |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this deliverable |
| incoming | [deliverable](deliverable.md) | `DeliverableDependsOn` | Deliverables that depend on this one |

## Examples

### Web Application

```spec
use "journeys/admin-users"
use "journeys/admin-reports"
use "modules/core"
use "modules/reporting"
use "product/milestones"

deliverable web_dashboard "Web Dashboard" {
  artifact_type web_app
  status        in_progress
  journeys      [create_user_web, create_user_api, export_reports]
  modules       [core_mod, reporting_mod]
  milestones    [mvp_phase]
  refs          [jira.epic:PROJ-100]
}
```

### CLI Tool

```spec
use "journeys/developer-api"
use "modules/core"
use "modules/cli-utils"

deliverable cli_tool "CLI Tool" {
  artifact_type cli
  status        shipped
  version       "1.0.0"
  journeys      [search_records_cli, lint_specs_cli]
  modules       [core_mod, cli_utils_mod]
}
```

### Non-Software: Policy Framework

```spec
use "journeys/policy-review"
use "modules/advisory"

deliverable housing_policy "Housing Policy Framework" {
  artifact_type documentation
  status        draft
  journeys      [review_policy_draft, submit_public_comment]
  modules       [advisory_workstream]
}
```

### Minimal Deliverable

```spec
use "journeys/api-integration"

deliverable rest_api "REST API Service" {
  artifact_type api
  journeys [api_integration]
}
```

# deliverable

> **Declared by `@specforge/product`**, one of the builtin extensions. `deliverable` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `deliverable` is something that ships: an app, a service, a CLI, a document, a package. It bundles the
journeys it serves and the modules it is built from. It answers: **"What ships to users?"**

Deliverables make release planning explicit: `release -> deliverable -> journey -> feature`. The artifact need
not be code: `artifact_type documentation` covers a policy framework or a procedure manual.

## Syntax

```spec
deliverable user_mgmt_mvp "User Management MVP" {
  artifact_type web_app
  status        in_progress
  journeys      [create_user_web, create_user_api]
  modules       [core_mod]
  milestones    [mvp_phase]
  version       "0.1.0"
  owner         "platform-team"
}
```

## Fields

Every `deliverable` has an id (the word after `deliverable`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `artifact_type` | enum: cli, service, library, web_app, mobile_app, api, extension, documentation, package | Kind of artifact produced. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of the deliverable. |
| `status` | enum: draft, in_progress, shipped, deprecated | Lifecycle state; default `draft`. |
| `journeys` | reference list → `journey` | User journeys this deliverable supports. |
| `modules` | reference list → `module` | Modules included in this deliverable. |
| `version` | string | Semantic version of this deliverable. |
| `milestones` | reference list → `milestone` | Milestones this deliverable is tracked under. |
| `depends_on` | reference list → `deliverable` | Other deliverables that must ship before this one. |
| `reason` | string | Justification for the current status or a status change. |
| `owner` | string | Person or team responsible for this deliverable. |
| `contributors` | string list | Additional people or teams contributing to this deliverable. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [module](module.md) | `DeliverableContainsModule` | `modules` on `deliverable` | Modules included in this deliverable |
| outgoing | [deliverable](deliverable.md) | `DeliverableDependsOn` | `depends_on` on `deliverable` | Other deliverables that must ship before this one |
| outgoing | [journey](journey.md) | `DeliverableSupportsJourney` | `journeys` on `deliverable` | User journeys this deliverable supports |
| outgoing | [milestone](milestone.md) | `DeliverableTrackedByMilestone` | `milestones` on `deliverable` | Milestones this deliverable is tracked under |
| incoming | [deliverable](deliverable.md) | `DeliverableDependsOn` | `depends_on` on `deliverable` | Other deliverables that must ship before this one |
| incoming | [release](release.md) | `ReleaseIncludesDeliverable` | `deliverables` on `release` | Deliverables included in this release |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| E052 | error | `@specforge/product` | Deliverable dependency cycle |
| I061 | info | `@specforge/product` | Deliverable version not semver |
| I066 | info | `@specforge/product` | Deprecated deliverable missing reason |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I080 | info | `@specforge/product` | Entity without owner |
| W043 | warning | `@specforge/product` | Deliverable without journeys |
| W046 | warning | `@specforge/product` | Deliverable without modules |


## Queries

`@specforge/product` also declares commands that read deliverables from the graph, on the CLI and, under the same
names, as MCP tools:

| Command | Answers |
|---------|---------|
| `specforge product deliverable-traceability` | The features a deliverable reaches |
| `specforge product deliverable-completion` | Deliverable completion |
| `specforge product deliverable-dependents` | Deliverables that depend on a given deliverable |
| `specforge product deliverable-priority` | A deliverable's derived priority |
| `specforge product deliverable-personas` | The personas a deliverable serves (via journeys) |

## Design guidance

### Granularity

A deliverable should be **shippable** (you can deploy, publish or hand it over), **self-contained** (it bundles
the journeys its use case needs) and **aimed** at specific personas.

### Status lifecycle

```
draft -> in_progress -> shipped
                |
            deprecated (give a reason)
```

A `deprecated` deliverable without a `reason` is I066.

### Deliverable vs. module

| Deliverable | Module |
|-------------|--------|
| "User Management MVP" / "Housing Policy Framework" | "core" / "Advisory Workstream" |
| What users or stakeholders receive | What implementers build |
| Maps journeys to modules | Maps features to dependencies |

## Examples

### CLI tool

```spec
deliverable cli_tool "CLI Tool" {
  artifact_type cli
  status        shipped
  version       "1.0.0"
  journeys      [search_records_cli]
  modules       [core_mod, cli_utils_mod]
}
```

### Outside software: a policy framework

```spec
deliverable housing_policy "Housing Policy Framework" {
  artifact_type documentation
  status        draft
  journeys      [review_policy_draft, submit_public_comment]
  modules       [advisory_workstream]
}
```

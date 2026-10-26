# feature

> **Declared by `@specforge/product`**, one of the builtin extensions. `feature` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `feature` is a unit of value for the people a product serves: the thing you put on a roadmap. It answers:
**"What value does this deliver?"**

A feature frames a problem and a solution. In a software project, behaviors implement it (they name it in their
`features` field). Nothing in the kind is software-only: a feature can just as well be a service offer, a loyalty
programme or a policy change, tracked through journeys, milestones and releases.

## Syntax

```spec
feature user_management "User Management" {
  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and a full audit trail.
  """

  priority   high
  status     in_progress
  owner      "platform-team"
  effort     m
  depends_on [password_auth]
}
```

## Fields

Every `feature` has an id (the word after `feature`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `problem` | string | The user problem this feature addresses. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of the feature. |
| `solution` | string | How this feature solves the stated problem. |
| `priority` | enum: critical, high, medium, low | Importance level. |
| `status` | enum: proposed, accepted, in_progress, done, deferred, deprecated | Lifecycle state; default `proposed`. |
| `acceptance` | string list | Criteria that must be met for the feature to be considered complete. |
| `depends_on` | reference list → `feature` | Other features that must be completed before this one. |
| `features` | reference list → `feature` | Related features referenced by this feature. |
| `refs` | string list | External references such as issues, URLs, or documents. |
| `reason` | string | Justification for the current status or a status change. |
| `owner` | string | Person or team responsible for this feature. |
| `contributors` | string list | Additional people or teams contributing to this feature. |
| `effort` | enum: xs, s, m, l, xl | T-shirt size estimate. |
| `tests` | string list | Executable test files that verify this feature, relative to the project root (RES-15 linkage). |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [feature](feature.md) | `FeatureDependsOn` | `depends_on` on `feature` | Other features that must be completed before this one |
| outgoing | [feature](feature.md) | `FeatureRelatesTo` | `features` on `feature` | Feature has a non-dependency relationship to another feature |
| incoming | [behavior](behavior.md) | `BehaviorImplementsFeature` | `features` on `behavior` | Behavior implements a feature (cross-extension via peer_dependency @specforge/product) |
| incoming | [constraint](constraint.md) | `ConstraintGovernsFeature` | `governs_features` on `constraint` | Constraint governs a product feature |
| incoming | [decision](decision.md) | `DecisionAffectsFeature` | `affects_features` on `decision` | Decision affects a product feature |
| incoming | [failure_mode](failure-mode.md) | `FailureModeThreatensFeature` | `threatens_features` on `failure_mode` | Failure mode threatens a product feature |
| incoming | [feature](feature.md) | `FeatureDependsOn` | `depends_on` on `feature` | Other features that must be completed before this one |
| incoming | [feature](feature.md) | `FeatureRelatesTo` | `features` on `feature` | Feature has a non-dependency relationship to another feature |
| incoming | [journey](journey.md) | `JourneyExercisesFeature` | `features` on `journey` | Features exercised during this journey |
| incoming | [milestone](milestone.md) | `MilestoneDeliversFeature` | `features` on `milestone` | Features that must be delivered to complete this milestone |
| incoming | [module](module.md) | `ModuleContainsFeature` | `features` on `module` | Features contained within this module |
| incoming | [persona](persona.md) | `PersonaPrioritizesFeature` | `key_features` on `persona` | Persona prioritizes a feature |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I048 | info | `@specforge/product` | Feature without acceptance criteria |
| I059 | info | `@specforge/product` | Deferred feature missing reason |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I080 | info | `@specforge/product` | Entity without owner |
| I081 | info | `@specforge/product` | Feature without effort estimate |
| W008 | warning | `@specforge/software` | Unimplemented feature |
| W041 | warning | `@specforge/product` | Unreferenced feature |
| W045 | warning | `@specforge/product` | Feature dependency cycle |


## Design guidance

### Granularity

A feature should be:
- **Deliverable**: something you can ship, demo or release on its own
- **Valuable**: it solves a real problem for someone
- **Composed**: it holds several behaviors or journeys (one alone may be too small)

### Problem and solution

Write `problem` from the user's or stakeholder's side: the pain, not the fix, in words a non-specialist
understands. Write `solution` from the system's side: the approach, at a high level.

### Status lifecycle

```
proposed -> accepted -> in_progress -> done -> deprecated
                |
             deferred (give a reason)
```

`status` drives milestone completion: a feature counts as finished only when it is `done`. A `deferred` feature
without a `reason` is I059.

### Feature vs. behavior

| Feature | Behavior |
|---------|----------|
| "User Management" | "Create User" |
| Several operations | One operation |
| Value someone sees | Implementation contract |
| Goes on a roadmap | Goes in a test suite |

### Feature vs. journey

| Feature | Journey |
|---------|---------|
| "User Management" | "Create a New User" |
| What the product can do | How one persona experiences it |
| Product-centric | User-centric |

## Examples

### With acceptance criteria

```spec
feature full_text_search "Full-Text Search" {
  problem "Users with large datasets cannot find specific records quickly."
  solution "Full-text search with typo tolerance, field weighting and faceted filtering."

  priority   high
  status     in_progress
  depends_on [user_management]

  acceptance [
    "Search returns results within 200ms for datasets up to 1M records",
    "Typo tolerance handles single-character errors",
  ]
}
```

### Deferred

```spec
feature analytics_dashboard "Analytics Dashboard" {
  problem "Product managers need visibility into usage patterns."
  solution "A dashboard with date range filtering and CSV export."

  priority high
  status   deferred
  reason   "Deprioritized in favor of core features for the first release"
}
```

### Outside software

```spec
feature loyalty_programme "Loyalty Programme" {
  problem "Repeat customers get nothing for coming back, and churn to competitors."
  solution "Points on every purchase, redeemable in store and online."

  priority high
  status   accepted
  owner    "marketing"
  effort   l
  tags     ["retention", "q3-campaign"]
}
```

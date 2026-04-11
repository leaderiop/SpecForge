# feature

> **Module:** `@specforge/product`

## Purpose

A `feature` declares a **user-facing value unit** composed of one or more behaviors. Features bridge the gap between what users need (problem) and how the system delivers it (solution). They group behaviors into coherent units of value.

It answers: **"What value does this deliver?"**

Features are the natural unit for product planning, release scoping, and stakeholder communication. A feature is what you put on a roadmap. A behavior is how you implement it.

## ID Pattern

```
identifier
```

Examples: `user_management`, `password_auth`, `full_text_search`

## Syntax

```spec
feature user_management "User Management" {
  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and full audit trail.
  """

  priority   high
  status     in_progress
  depends_on [password_auth]
}
```

## Fields

All fields are optional at the type level. Features are intentionally lightweight — `problem` and `solution` are strongly recommended but not enforced by the compiler.

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the identifier). Auto-derived from identifier if omitted. |
| `problem` | string | What user need or pain point this feature addresses. Written from the user's perspective. |
| `solution` | string | How the system addresses the problem. Written from the system's perspective. |
| `priority` | Priority | Importance level: `critical`, `high`, `medium`, `low`. Validated by W078. |
| `status` | FeatureStatus | Lifecycle state: `proposed`, `accepted`, `in_progress`, `done`, `deferred`. Validated by W077. Used in completion calculations. |
| `acceptance` | string[] | Free-form acceptance criteria. Omission emits I048. |
| `depends_on` | EntityId[] | Other features this one depends on. Creates `FeatureDependsOn` edges. Cycles detected by W045. |
| `reason` | string | Rationale for current status (expected when `deferred`, checked by I059). |
| `tags` | string[] | Free-form tags for categorization. Format validated by I068 (lowercase hyphen-separated, 2-50 chars). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this feature. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `FeatureDependsOn` | "This feature depends on that feature" |
| `ref` | `links_to` | "This feature links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `behavior` | `Implements` | "A behavior implements this feature" (requires `@specforge/software`) |
| `journey` | `JourneyFeature` | "This UX journey delivers this feature" |
| `module` | `ModuleFeature` | "A module provides the code for this feature" |
| `milestone` | `MilestoneFeature` | "A milestone schedules this feature" |

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| W041 | warning | Feature not referenced by any journey, module, or milestone (orphan feature). |
| W045 | warning | Circular feature dependency — `depends_on` edges form a cycle. |
| W077 | warning | Invalid `status` value (not in FeatureStatus enum). |
| W078 | warning | Invalid `priority` value (not in Priority enum). |
| I048 | info | Feature has no acceptance criteria. |
| I059 | info | Deferred feature without a `reason`. |
| I063 | info | Done feature with incomplete dependencies (depends_on features not done). |

## Design Guidance

### Feature Granularity

A feature should be:
- **Deliverable** — something you can ship, demo, or release independently
- **Valuable** — it solves a real user problem
- **Composed** — it contains multiple behaviors (if only one behavior, it might be too granular)

### Problem/Solution Framing

The `problem` field should:
- Be written from the user's or stakeholder's perspective
- Describe the pain point, not the solution
- Be understandable by non-technical stakeholders

The `solution` field should:
- Be written from the system's perspective
- Describe the approach at a high level
- Reference specific technologies or patterns where relevant

### Status Lifecycle

```
proposed -> accepted -> in_progress -> done
                |
             deferred (requires reason)
```

The `status` field drives milestone completion calculations. A feature counts as "done" only when `status` is `done`. Deferred features should include a `reason`.

### Feature vs. Behavior

| Feature | Behavior |
|---------|----------|
| "User Management" | "Create User" |
| Composed of multiple operations | A single atomic operation |
| User-visible value | Implementation contract |
| Goes on a roadmap | Goes in a test suite |
| Problem/solution framing | Contract with MUST/SHOULD/MAY |

### Feature vs. Journey

| Feature | Journey |
|---------|---------|
| "User Management" | "Create a New User" |
| What the system can do | How a specific persona experiences it |
| Groups behaviors | Groups features by persona + channel |
| System-centric | User-centric |

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `FeatureDependsOn` | Features this feature depends on |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this feature |
| incoming | [behavior](behavior.md) | `Implements` | Behaviors that implement this feature |
| incoming | [journey](journey.md) | `JourneyFeature` | Journeys that deliver this feature |
| incoming | [module](module.md) | `ModuleFeature` | Modules that implement this feature |
| incoming | [milestone](milestone.md) | `MilestoneFeature` | Milestones that schedule this feature |

## Examples

### Simple Feature

```spec
feature password_auth "Password Authentication" {
  problem """
    Users need to securely authenticate to access their accounts.
    The system must prevent unauthorized access while keeping
    the login experience fast and simple.
  """

  solution """
    Password-based authentication with bcrypt hashing,
    rate-limited login attempts, and secure session tokens.
  """

  status accepted
}
```

### Feature with Dependencies

```spec
feature full_text_search "Full-Text Search" {
  problem """
    Users with large datasets cannot find specific records quickly.
    Current filtering by exact field match is insufficient for
    free-form queries across multiple fields.
  """

  solution """
    Elasticsearch-backed full-text search with typo tolerance,
    field weighting, and faceted filtering. Results ranked by
    relevance with highlighting of matched terms.
  """

  priority   high
  status     in_progress
  depends_on [user_management]

  acceptance [
    "Search returns results within 200ms for datasets up to 1M records",
    "Typo tolerance handles single-character errors",
    "Results include highlighted matched terms",
  ]

  refs [gh.issue:88, figma.frame:search-ui]
}
```

### Deferred Feature

```spec
feature analytics_dashboard "Analytics Dashboard" {
  problem """
    Product managers need visibility into usage patterns
    and adoption metrics.
  """

  solution """
    Real-time dashboard with configurable widgets,
    date range filtering, and CSV export.
  """

  priority high
  status   deferred
  reason   "Deprioritized in favor of core CRUD features for MVP"
}
```

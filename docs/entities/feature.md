# feature

> **Module:** `core`

## Purpose

A `feature` declares a **user-facing capability** composed of one or more behaviors. Features bridge the gap between what users need (problem) and how the system delivers it (solution). They group behaviors into coherent units of value.

It answers: **"What value does this deliver?"**

Features are the natural unit for product planning, release scoping, and stakeholder communication. A feature is what you put on a roadmap. A behavior is how you implement it.

## ID Pattern

```
identifier
```

Examples: `user_management`, `password_auth`, `full_text_search`

## Syntax

```spec
use behaviors/user-crud

feature user_management "User Management" {
  behaviors [create_user, read_user, update_email]

  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and full audit trail.
  """

  roadmap [mvp_phase]
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `behaviors` | reference list | The behaviors that compose this feature. Every behavior referenced must exist. |
| `problem` | string or triple-string | What user need or pain point this feature addresses. Written from the user's perspective. |
| `solution` | string or triple-string | How the system addresses the problem. Written from the system's perspective. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the identifier). Optional — auto-derived from identifier if omitted. |
| `roadmap` | reference list | Roadmap milestones this feature is planned for. |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this feature. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `behavior` | `implements` | "This feature is composed of these behaviors" |
| `ref` | `links_to` | "This feature links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `capability` | `traces_to` | "This UX capability delivers this feature" |
| `library` | `provides` | "A library provides the code for this feature" |
| `roadmap` | `schedules` | "A roadmap phase schedules this feature" |

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `behaviors` must resolve to an existing `behavior`. |
| E002 | No two features may share the same ID. |
| W002 | If no `capability` references this feature, emit "orphan feature" warning. |

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

### Feature vs. Behavior

| Feature | Behavior |
|---------|----------|
| "User Management" | "Create User" |
| Composed of multiple operations | A single atomic operation |
| User-visible value | Implementation contract |
| Goes on a roadmap | Goes in a test suite |
| Problem/solution framing | Contract with MUST/SHOULD/MAY |

### Feature vs. Capability

| Feature | Capability |
|---------|------------|
| "User Management" | "Create a New User" |
| What the system can do | How a specific persona experiences it |
| Groups behaviors | Groups features by persona + surface |
| System-centric | User-centric |

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [behavior](behavior.md) | `implements` | Behaviors that compose this feature |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this feature |
| incoming | [capability](capability.md) | `traces_to` | Capabilities that deliver this feature |
| incoming | [library](library.md) | `provides` | Libraries that implement this feature |
| incoming | [roadmap](roadmap.md) | `schedules` | Roadmap phases that schedule this feature |

## Examples

### Simple Feature

```spec
use behaviors/auth

feature password_auth "Password Authentication" {
  behaviors [login, logout, reset_password]

  problem """
    Users need to securely authenticate to access their accounts.
    The system must prevent unauthorized access while keeping
    the login experience fast and simple.
  """

  solution """
    Password-based authentication with bcrypt hashing,
    rate-limited login attempts, and secure session tokens.
  """
}
```

### Feature with Roadmap

```spec
use behaviors/search

feature full_text_search "Full-Text Search" {
  behaviors [search_query, search_suggest, search_facets, search_highlight]

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

  roadmap [v2_phase]
  refs [gh.issue:88, figma.frame:search-ui]
}
```

### Feature Composing Cross-Domain Behaviors

```spec
use behaviors/order-processing
use behaviors/inventory
use behaviors/billing

feature order_checkout "Order Checkout" {
  behaviors [place_order, validate_inventory, process_payment, confirm_order]

  problem """
    Customers need to complete purchases with confidence that
    their order will be fulfilled, their payment processed correctly,
    and inventory accurately reserved.
  """

  solution """
    Orchestrated checkout flow: validate inventory, reserve items,
    process payment via Stripe, create order record. Compensating
    transactions on failure (release inventory, refund payment).
    Event-driven notifications at each stage.
  """

  roadmap [mvp_phase]
}
```

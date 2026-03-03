---
name: specforge-features-dsl
description: "Write feature blocks in .spec DSL files. Each feature declares a user-facing capability composed of behaviors with FEAT-{infix}-{n} IDs, problem/solution framing, and behavior composition. Use when grouping behaviors into coherent units of value for product planning."
---

# SpecForge Features DSL

Rules and conventions for authoring **`feature` blocks** in `.spec` files. Features bridge what users need (problem) and how the system delivers it (solution) by composing behaviors.

## When to Use

- Grouping related behaviors into a deliverable unit of value
- Defining what a feature solves (problem) and how (solution)
- Connecting behaviors to the product planning chain
- Creating the features that capabilities will reference

## Block Syntax

```spec
use behaviors/user-crud

feature FEAT-MS-001 "User Management" {
  behaviors [BEH-MS-001, BEH-MS-002, BEH-MS-003]

  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and full audit trail.
  """

  roadmap [RM-01]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `behaviors` | reference list | Behaviors that compose this feature. Every one must exist. |
| `problem` | string / triple-string | User need or pain point. Written from the user's perspective. |
| `solution` | string / triple-string | How the system addresses the problem. Written from the system's perspective. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `roadmap` | reference list | Roadmap milestones this feature is planned for. |
| `refs` | reference list | External references linked to this feature. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `behavior` | `implements` | Behaviors that compose this feature |
| `ref` | `links_to` | External references linked to this feature |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `capability` | `traces_to` | UX capability delivers this feature |
| `library` | `provides` | Library implements this feature |
| `roadmap` | `schedules` | Roadmap phase schedules this feature |

## Writing Rules

1. **A feature groups multiple behaviors** — if it has only one behavior, it may be too granular.
2. **Problem is user-perspective** — describe the pain point, not the solution.
3. **Solution is system-perspective** — describe the approach at a high level.
4. **Import behavior files** — `use` the files that declare the referenced behaviors.
5. **`roadmap` is a soft reference** — if `@specforge/product` is not installed, emits `I004`.
6. **Features go on roadmaps, behaviors go in test suites** — keep the distinction clear.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `behaviors` must resolve to an existing behavior. |
| E002 | No duplicate feature IDs across all `.spec` files. |
| W002 | Orphan feature — not referenced by any capability. |

## Examples

### Simple Feature

```spec
use behaviors/auth

feature FEAT-MS-002 "Password Authentication" {
  behaviors [BEH-MS-010, BEH-MS-011, BEH-MS-012]

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

### Feature with Roadmap and Refs

```spec
use behaviors/search

feature FEAT-MS-005 "Full-Text Search" {
  behaviors [BEH-MS-030, BEH-MS-031, BEH-MS-032, BEH-MS-033]

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

  roadmap [RM-03]
  refs [gh.issue:88, figma.frame:search-ui]
}
```

### Cross-Domain Feature

```spec
use behaviors/order-processing
use behaviors/inventory
use behaviors/billing

feature FEAT-MS-010 "Order Checkout" {
  behaviors [BEH-MS-050, BEH-MS-051, BEH-MS-060, BEH-MS-070]

  problem """
    Customers need to complete purchases with confidence that
    their order will be fulfilled, their payment processed correctly,
    and inventory accurately reserved.
  """

  solution """
    Orchestrated checkout flow: validate inventory, reserve items,
    process payment via Stripe, create order record. Compensating
    transactions on failure (release inventory, refund payment).
  """

  roadmap [RM-02]
}
```

## What NOT to Do

- Do not write a feature with a single behavior — that is too granular
- Do not put MUST/SHOULD keywords in the problem/solution — save those for behavior contracts
- Do not confuse features (system-perspective value) with capabilities (user-perspective UX flow)
- Do not reference behaviors from other files without a `use` import
- Do not put implementation details in the `problem` field — it should be user-perspective

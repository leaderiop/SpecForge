---
name: specforge-features-dsl
description: "Write feature blocks in .spec DSL files. Each feature declares a user-facing capability composed of behaviors with free-form snake_case IDs, problem/solution framing, and behavior composition. Features are NOT testable (testable=false). Use when grouping behaviors into coherent units of value for product planning."
---

# SpecForge Features DSL

Rules and conventions for authoring **`feature` blocks** in `.spec` files. Features bridge what users need (problem) and how the system delivers it (solution) by composing behaviors.

**Requires:** `@specforge/software` plugin.

**Note:** Features are NOT testable entities (`testable=false` in the extension manifest). They do not support `verify` statements. Testing happens at the behavior level.

## When to Use

- Grouping related behaviors into a deliverable unit of value
- Defining what a feature solves (problem) and how (solution)
- Connecting behaviors to the product planning chain
- Creating the features that capabilities will reference

## Block Syntax

```spec
use behaviors/user-crud

feature user_management "User Management" {
  behaviors [create_user, read_user_by_id, update_user_email]

  problem """
    Administrators need to manage user accounts
    with guaranteed data integrity.
  """

  solution """
    CRUD operations backed by PostgreSQL with
    unique email constraints and full audit trail.
  """
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `behaviors` | reference list | Behaviors that compose this feature. Every one must exist. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `problem` | string / triple-string | User need or pain point. Written from the user's perspective. |
| `solution` | string / triple-string | How the system addresses the problem. Written from the system's perspective. |
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

1. **A feature groups multiple behaviors** -- if it has only one behavior, it may be too granular.
2. **Problem is user-perspective** -- describe the pain point, not the solution.
3. **Solution is system-perspective** -- describe the approach at a high level.
4. **Import behavior files** -- `use` the files that declare the referenced behaviors.
5. **Features go on roadmaps, behaviors go in test suites** -- keep the distinction clear.
6. **Features are not testable** -- do not add `verify` statements; testing is at the behavior level.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `behaviors` must resolve to an existing behavior. |
| E002 | No duplicate feature IDs across all `.spec` files. |
| W002 | Orphan feature -- not referenced by any capability. |
| W008 | Feature with empty behaviors list. |

## Examples

### Simple Feature

```spec
use behaviors/auth

feature password_authentication "Password Authentication" {
  behaviors [authenticate_user, hash_password, validate_session]

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

### Feature with Refs

```spec
use behaviors/search

feature full_text_search "Full-Text Search" {
  behaviors [index_documents, search_query, faceted_filter, highlight_matches]

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

  refs [gh.issue:88, figma.frame:search-ui]
}
```

### Cross-Domain Feature

```spec
use behaviors/order-processing
use behaviors/inventory
use behaviors/billing

feature order_checkout "Order Checkout" {
  behaviors [place_order, reserve_inventory, process_payment, confirm_order]

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
}
```

## What NOT to Do

- Do not write a feature with a single behavior -- that is too granular
- Do not put MUST/SHOULD keywords in the problem/solution -- save those for behavior contracts
- Do not confuse features (system-perspective value) with capabilities (user-perspective UX flow)
- Do not reference behaviors from other files without a `use` import
- Do not put implementation details in the `problem` field -- it should be user-perspective
- Do not add `verify` statements -- features are not testable entities

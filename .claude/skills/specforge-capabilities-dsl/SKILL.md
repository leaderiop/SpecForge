---
name: specforge-capabilities-dsl
description: "Write capability blocks in .spec DSL files (@specforge/product plugin). Each capability declares a UX flow with free-form snake_case IDs, mapping a persona on a surface to features with a step-by-step interaction flow. Use when describing how a specific user type experiences the system."
---

# SpecForge Capabilities DSL

Rules and conventions for authoring **`capability` blocks** in `.spec` files. Capabilities are the top of the traceability chain -- they represent what a real user actually does with the system.

**Requires:** `@specforge/product` plugin.

## When to Use

- Describing how a specific persona interacts with the system
- Mapping UX flows to features
- Defining interaction sequences for different surfaces (web, mobile, CLI, API)
- Creating the capabilities that deliverables will bundle

## Block Syntax

```spec
use features/user-management

capability create_new_user "Create a New User" {
  persona  admin
  surface  [web, cli, api]
  features [user_management]

  flow """
    1. Admin opens user management page
    2. Clicks "New User"
    3. Fills form (name, email, role)
    4. Submits -- system validates uniqueness
    5. Success: user appears in list
    6. Failure: inline error on email field
  """
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). Phrased as a user action. |
| `persona` | identifier | The user type performing this action (e.g., `admin`, `developer`, `api_consumer`). |
| `features` | reference list | Features this capability delivers. |
| `flow` | string / triple-string | Step-by-step interaction flow (user actions + system responses). |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `surface` | identifier list | Interaction surfaces (e.g., `web`, `mobile`, `cli`, `api`). |
| `refs` | reference list | External references linked to this capability. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `traces_to` | Capability delivers these features |
| `ref` | `links_to` | External references linked to this capability |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `deliverable` | `bundles` | Deliverable ships this capability |

## Writing Rules

1. **Name as user actions** -- "Create a New User" not "User Creation".
2. **Numbered flow steps** -- alternate between user actions and system responses.
3. **Include success and failure paths** -- both in the flow.
4. **Persona must match spec root** -- if `persona` definitions exist in `spec` block, this must match (E008).
5. **Surface must match spec root** -- if `surface` definitions exist, values must match (E009).
6. **One capability per persona-action** -- different personas doing the same thing = different capabilities.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` must resolve to an existing feature. |
| E002 | No duplicate capability IDs. |
| E008 | `persona` must match a persona defined in the spec root. |
| E009 | Every `surface` must match a surface defined in the spec root. |
| W011 | Orphan capability -- not referenced by any deliverable. |

## Examples

### Web UI Capability

```spec
capability create_new_user "Create a New User" {
  persona  admin
  surface  [web]
  features [user_management]

  flow """
    1. Admin navigates to Settings > Users
    2. Clicks "Add User" button
    3. Modal opens with form: name, email, role dropdown
    4. Admin fills fields and clicks "Create"
    5. System validates email uniqueness
    6. Success: modal closes, user appears in table, toast notification
    7. Failure: inline error below email field
  """

  refs [figma.frame:user-create-modal]
}
```

### API Capability

```spec
capability create_user_via_api "Create User via API" {
  persona  api_consumer
  surface  [api]
  features [user_management]

  flow """
    1. Consumer sends POST /api/v1/users with JSON body
    2. System validates request body schema
    3. System validates email uniqueness
    4. Success: 201 Created with user object
    5. Validation failure: 422 Unprocessable Entity
    6. Duplicate email: 409 Conflict
  """
}
```

### Multi-Feature Capability

```spec
capability purchase_and_track_order "Purchase and Track Order" {
  persona  user
  surface  [web, mobile]
  features [order_checkout, order_tracking]

  flow """
    1. User reviews cart and clicks "Checkout"
    2. System validates inventory for all items
    3. User confirms shipping address and payment method
    4. System processes payment and creates order
    5. Success: confirmation page with order number and ETA
    6. User can track order status on "My Orders" page
    7. Payment failure: error page with retry option
  """
}
```

## What NOT to Do

- Do not write capabilities without the `@specforge/product` plugin installed
- Do not confuse capabilities (user-perspective UX flow) with features (system-perspective value)
- Do not use generic personas -- be specific: `admin`, `developer`, `api_consumer`
- Do not omit failure paths from the flow -- users need to know what happens on error
- Do not reference features from other files without a `use` import

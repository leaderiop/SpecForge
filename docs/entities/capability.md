# capability

> **Module:** `@specforge/product`

## Purpose

A `capability` declares a **UX flow** — a concrete interaction path that maps a specific persona on a specific surface to the features they use. Capabilities are the top of the traceability chain: they represent what a real user actually does with the system.

It answers: **"How does the user experience this?"**

While features describe *what* the system delivers, capabilities describe *how* a specific type of user interacts with it. The same feature (e.g., "User Management") may surface as different capabilities for different personas: an admin creates users through a web dashboard, while an API consumer creates users through a REST endpoint.

## ID Pattern

```
identifier
```

Examples: `create_user_web`, `create_user_api`, `search_records_cli`

## Syntax

```spec
use features/user-management

capability create_user_web "Create a New User" {
  persona  admin
  surface  [web, cli, api]
  features [user_management]

  flow """
    1. Admin opens user management page
    2. Clicks "New User"
    3. Fills form (name, email, role)
    4. Submits — system validates uniqueness
    5. Success: user appears in list
    6. Failure: inline error on email field
  """
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `persona` | enum or identifier | The type of user performing this action (e.g., `admin`, `viewer`, `api_consumer`, `operator`). |
| `features` | reference list | Features this capability delivers. |
| `flow` | string or triple-string | Step-by-step interaction flow describing what the user does and what the system responds. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (the string after the entity ID). Phrased as an action from the user's perspective. |
| `surface` | reference list or identifier list | The interaction surfaces where this capability is available (e.g., `web`, `mobile`, `cli`, `api`, `desktop`). |
| `refs` | reference list | External references (issues, tickets, diagrams) linked to this capability. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `traces_to` | "This UX capability delivers these features" |
| `ref` | `links_to` | "This capability links to these external references" |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `deliverable` | `bundles` | "A deliverable ships this capability" |

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `features` must resolve to an existing `feature`. |
| E002 | No two capabilities may share the same ID. |
| E008 | `persona` must match a persona defined in the `spec` root (only when spec root defines personas). |
| E009 | Every `surface` must match a surface defined in the `spec` root (only when spec root defines surfaces). |
| W011 | If no `deliverable` references this capability, emit "orphan capability" warning. |

## Design Guidance

### Naming Capabilities

Capabilities should be named as user actions in the imperative mood:
- "Create a New User" (not "User Creation")
- "Search for Records" (not "Record Search")
- "Export Monthly Report" (not "Report Export")

### Writing Flows

Flows should:
- Be numbered steps (1, 2, 3...)
- Alternate between user actions and system responses
- Include both success and failure paths
- Be concrete enough to derive a UI wireframe or API sequence

### Persona Definition

Personas are identifiers, not full persona definitions. They name a role:

| Persona | Typical Meaning |
|---------|----------------|
| `admin` | System administrator with full access |
| `user` | Regular authenticated user |
| `viewer` | Read-only access |
| `api_consumer` | Machine-to-machine integration |
| `operator` | DevOps / infrastructure team |
| `auditor` | Compliance / read-only audit access |

Persona definitions themselves are documentation, not compiled entities. Define them in your project's overview or governance docs.

### Surface Definition

Surfaces are the interaction channels:

| Surface | Meaning |
|---------|---------|
| `web` | Browser-based web application |
| `mobile` | Native mobile app (iOS/Android) |
| `cli` | Command-line interface |
| `api` | REST/gRPC/GraphQL API endpoint |
| `desktop` | Desktop application |
| `email` | Email-triggered workflow |

### Capability vs. Feature

| Capability | Feature |
|------------|---------|
| User-perspective | System-perspective |
| "Admin creates a new user via web dashboard" | "User Management" |
| Includes persona, surface, interaction flow | Includes problem, solution, behaviors |
| One capability may use one feature | One feature may serve many capabilities |

### Multiple Capabilities per Feature

A single feature often surfaces as multiple capabilities:

```
feature user_management
  ├── capability create_user_web (admin, web)
  ├── capability create_user_api (api_consumer, api)
  ├── capability view_user_list (admin, web)
  └── capability search_users (admin, web)
```

## Related Entities

| Direction | Entity | Edge | Meaning |
|-----------|--------|------|---------|
| outgoing | [feature](feature.md) | `traces_to` | Features this capability delivers |
| outgoing | [ref](ref.md) | `links_to` | External references linked to this capability |
| incoming | [deliverable](deliverable.md) | `bundles` | Deliverables that ship this capability |

## Examples

### Web UI Capability

```spec
use features/user-management

capability create_user_web "Create a New User" {
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
    7. Failure: inline error below email field, "Email already in use"
  """

  refs [figma.frame:user-create-modal]
}
```

### API Capability

```spec
use features/user-management

capability create_user_api "Create User via API" {
  persona  api_consumer
  surface  [api]
  features [user_management]

  flow """
    1. Consumer sends POST /api/v1/users with JSON body
    2. System validates request body schema
    3. System validates email uniqueness
    4. Success: 201 Created with user object in response body
    5. Validation failure: 422 Unprocessable Entity with error details
    6. Duplicate email: 409 Conflict with error message
  """
}
```

### CLI Capability

```spec
use features/search

capability search_records_cli "Search Records from Terminal" {
  persona  operator
  surface  [cli]
  features [full_text_search]

  flow """
    1. Operator runs: specforge search "user email:alice@"
    2. System performs full-text search with field filtering
    3. Results displayed in table format with highlighting
    4. Operator can pipe output to jq or other tools
    5. No results: informative message with suggestion to broaden query
  """
}
```

### Multi-Feature Capability

```spec
use features/order-checkout
use features/order-tracking

capability purchase_and_track "Purchase and Track Order" {
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
    7. System sends email notification at each fulfillment stage
    8. Payment failure: error page with retry option, cart preserved
  """
}
```

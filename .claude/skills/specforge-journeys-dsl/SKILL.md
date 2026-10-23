---
name: specforge-journeys-dsl
description: "Write journey blocks in .spec DSL files (@specforge/product plugin). Each journey declares a UX flow with free-form snake_case IDs, mapping a persona on channels to features with a step-by-step interaction flow. Use when describing how a specific user type experiences the system."
---

# SpecForge Journeys DSL

Rules and conventions for authoring **`journey` blocks** in `.spec` files. Journeys are the top of the traceability chain -- they represent what a real user actually does with the system.

**Requires:** `@specforge/product` plugin.

## When to Use

- Describing how a specific persona interacts with the system
- Mapping UX flows to features
- Defining interaction sequences for different channels (web, mobile, cli, api)
- Creating the journeys that deliverables will bundle

## Block Syntax

```spec
use "features/user-management"
use "product/personas"
use "product/channels"

journey create_new_user "Create a New User" {
  persona  admin
  channels [web, cli, api]
  features [user_management]

  flow [
    "Admin opens user management page",
    "Clicks \"New User\"",
    "Fills form (name, email, role)",
    "Submits -- system validates uniqueness",
    "Success: user appears in list",
    "Failure: inline error on email field",
  ]
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). Phrased as a user action. |
| `persona` | EntityId @optional | The user type performing this action (e.g., `admin`, `developer`, `patient`, `student`, `volunteer`). Omission emits I054. |
| `features` | reference list | Features this journey delivers. |
| `flow` | string list | One item per step, in order (user actions + system responses). Omission emits I050. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `channels` | EntityId[] @optional | Channel entities where this journey is available (e.g., `web`, `mobile`, `cli`, `api`). Each must resolve to a `channel` entity (E009). Omission emits I055. |
| `description` | string @optional | Brief summary of the journey's purpose. |
| `priority` | Priority @optional | Importance level: `critical`, `high`, `medium`, `low`. |
| `tags` | string[] @optional | Faceted filtering tags. |
| `refs` | reference list | External references linked to this journey. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `feature` | `JourneyFeature` | Journey delivers these features |
| `persona` | `JourneyPersona` | Journey is performed by this persona |
| `channel` | `JourneyChannel` | Journey uses these channels |
| `ref` | `links_to` | External references linked to this journey |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `deliverable` | `DeliverableJourney` | Deliverable ships this journey |

## Writing Rules

1. **Name as user actions** -- "Create a New User" not "User Creation".
2. **One item per step, in order** -- alternate between user actions and system responses.
3. **Include success and failure paths** -- both in the flow.
4. **Persona must resolve** -- `persona` must match an existing `persona` entity (E008).
5. **Channels must resolve** -- every `channels` entry must match an existing `channel` entity (E009).
6. **One journey per persona-action** -- different personas doing the same thing = different journeys.

## Validation Rules

| Code | Level | Rule |
|------|-------|------|
| E008 | error | `persona` must resolve to an existing `persona` entity. |
| E009 | error | Every `channels` entry must resolve to an existing `channel` entity. |
| W042 | warning | Orphan journey -- not referenced by any deliverable. |
| W075 | warning | Journey references a deprecated persona. |
| W076 | warning | Journey references a deprecated channel. |
| I050 | info | Journey has no `flow` steps. |
| I054 | info | Journey has no `persona` field. |
| I055 | info | Journey has no `channels` field. |
| I072 | info | Flow step without numbered prefix (e.g., "1.", "2."). |

## Examples

### Web UI Journey

```spec
journey create_new_user "Create a New User" {
  persona  admin
  channels [web]
  features [user_management]

  flow [
    "Admin navigates to Settings > Users",
    "Clicks \"Add User\" button",
    "Modal opens with form: name, email, role dropdown",
    "Admin fills fields and clicks \"Create\"",
    "System validates email uniqueness",
    "Success: modal closes, user appears in table, toast notification",
    "Failure: inline error below email field",
  ]

  refs [figma.frame:user-create-modal]
}
```

### API Journey

```spec
journey create_user_via_api "Create User via API" {
  persona  api_consumer
  channels [api]
  features [user_management]

  flow [
    "Consumer sends POST /api/v1/users with JSON body",
    "System validates request body schema",
    "System validates email uniqueness",
    "Success: 201 Created with user object",
    "Validation failure: 422 Unprocessable Entity",
    "Duplicate email: 409 Conflict",
  ]
}
```

### Multi-Feature Journey

```spec
journey purchase_and_track_order "Purchase and Track Order" {
  persona  user
  channels [web, mobile]
  features [order_checkout, order_tracking]
  priority high

  flow [
    "User reviews cart and clicks \"Checkout\"",
    "System validates inventory for all items",
    "User confirms shipping address and payment method",
    "System processes payment and creates order",
    "Success: confirmation page with order number and ETA",
    "User can track order status on \"My Orders\" page",
    "Payment failure: error page with retry option",
  ]
}
```

### Healthcare Journey

```spec
journey schedule_appointment "Schedule a Doctor Appointment" {
  persona  patient
  channels [clinic, phone, web]
  features [appointment_scheduling]

  flow [
    "Patient calls clinic or opens web portal",
    "Selects preferred doctor and available time slot",
    "System checks doctor availability",
    "Patient confirms appointment details",
    "Success: confirmation sent via SMS and email",
    "Conflict: alternative slots suggested",
  ]
}
```

### Education Journey

```spec
journey enroll_in_course "Enroll in a Course" {
  persona  student
  channels [web, in_person]
  features [course_enrollment]

  flow [
    "Student browses course catalog",
    "Selects course and reviews prerequisites",
    "System checks prerequisite completion",
    "Student submits enrollment request",
    "Success: enrollment confirmed, added to roster",
    "Prerequisite missing: directed to prerequisite courses",
  ]
}
```

## What NOT to Do

- Do not write journeys without the `@specforge/product` plugin installed
- Do not confuse journeys (user-perspective UX flow) with features (system-perspective value)
- Do not use generic personas -- be specific: `admin`, `developer`, `patient`, `student`, `volunteer`
- Do not omit failure paths from the flow -- users need to know what happens on error
- Do not reference features from other files without a `use` import

---
name: specforge-deliverables-dsl
description: "Write deliverable blocks in .spec DSL files (@specforge/product plugin). Each deliverable declares a shippable artifact with DLV-{infix}-{n} IDs, mapping capabilities to libraries with artifact type and persona targeting. Use when describing what actually ships to users — apps, services, CLIs, extensions."
---

# SpecForge Deliverables DSL

Rules and conventions for authoring **`deliverable` blocks** in `.spec` files. Deliverables represent what actually ships — they bridge the UX layer (capabilities) to the code layer (libraries).

**Requires:** `@specforge/product` plugin.

## When to Use

- Defining what ships to users (apps, services, CLIs, extensions)
- Mapping capabilities to library dependencies
- Planning release artifacts with persona targeting
- Completing the top of the traceability chain: deliverable → capability → feature → behavior → invariant

## Block Syntax

```spec
use capabilities/admin-users
use capabilities/developer-api
use libraries/core
use libraries/email

deliverable DLV-MS-001 "User Management MVP" {
  type         app
  personas     [admin, developer]
  capabilities [UX-MS-001, UX-MS-002]
  libraries    [LIB-MS-001, LIB-MS-005]
  roadmap      RM-01
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Human-readable name (string after the entity ID). |
| `capabilities` | reference list | UX capabilities this deliverable ships. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `libraries` | reference list | Libraries this deliverable is built from. |
| `roadmap` | reference | Roadmap phase this deliverable is planned for. |
| `personas` | identifier list | Personas this deliverable targets. |
| `type` | enum / string | Artifact type: `app`, `service`, `cli`, `extension`, `library`, `webapp`. |
| `refs` | reference list | External references linked to this deliverable. |

## Relationships

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| `capability` | `bundles` | Deliverable ships these capabilities |
| `library` | `built_from` | Deliverable uses these libraries |
| `ref` | `links_to` | External references linked to this deliverable |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| `roadmap` | `schedules` | Roadmap phase schedules this deliverable |

## Writing Rules

1. **Shippable artifacts only** — deliverables are things you can deploy, publish, or release.
2. **Bundle capabilities, not features** — capabilities are the unit of user value in deliverables.
3. **List library dependencies** — makes the code-to-product mapping explicit.
4. **Coverage validation** — every capability's features should be reachable through the library chain (W008).
5. **Import capability and library files** — `use` the files that declare referenced entities.
6. **One deliverable per artifact** — a web app and a CLI are separate deliverables.

## Validation Rules

| Code | Rule |
|------|------|
| E001 | Every ID in `capabilities` and `libraries` must resolve. |
| E002 | No duplicate deliverable IDs. |
| W008 | Uncovered capability — capability not reachable via library chain. |

## Examples

### Web Application

```spec
deliverable DLV-MS-001 "Web Dashboard" {
  type         webapp
  personas     [admin]
  capabilities [UX-MS-001, UX-MS-002, UX-MS-005]
  libraries    [LIB-MS-001, LIB-MS-003]
  roadmap      RM-01
  refs         [jira.epic:PROJ-100]
}
```

### CLI Tool

```spec
deliverable DLV-MS-002 "CLI Tool" {
  type         cli
  personas     [developer]
  capabilities [UX-MS-010, UX-MS-011]
  libraries    [LIB-MS-001, LIB-MS-008]
  roadmap      RM-02
}
```

### Minimal

```spec
deliverable DLV-MS-003 "REST API Service" {
  capabilities [UX-MS-020]
}
```

## What NOT to Do

- Do not write deliverables without the `@specforge/product` plugin installed
- Do not confuse deliverables (what ships) with libraries (what code implements)
- Do not bundle features directly — bundle capabilities, which trace to features
- Do not omit `libraries` when you want coverage validation (W008)
- Do not reference capabilities or libraries from other files without `use` imports

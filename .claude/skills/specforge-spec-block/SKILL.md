---
name: specforge-spec-block
description: "Write the spec root configuration block in specforge.spec. Declares project identity (name, version), installed plugins, provider configurations, persona/surface definitions, coverage settings, and meta-schema define blocks. Use when initializing a new SpecForge project or modifying project-level configuration."
---

# SpecForge Spec Block

> **Note:** `specforge.json` is now the preferred configuration format. `specforge init` creates `specforge.json` instead of a `spec` block. The `spec` block in `.spec` files is still supported for backward compatibility -- projects without `specforge.json` continue to extract config from the spec block.

Rules and conventions for authoring the **`spec` root configuration block** in `specforge.spec`. The spec block is a singleton -- exactly one per project -- that declares project identity and configuration.

## When to Use

- Initializing a new SpecForge project (`specforge.spec`)
- Adding or removing plugins (`@specforge/product`, `@specforge/governance`)
- Configuring providers for external references (GitHub, Jira, Figma)
- Defining personas and surfaces for capability validation
- Configuring test coverage thresholds
- Adding meta-schema `define` blocks for custom entity types

## Block Syntax

```spec
spec "project-name" {
  version "1.0"

  plugins [
    "@specforge/product",
    "@specforge/governance",
  ]

  providers {
    gh "main" {
      package "@specforge/gh"
      repo    "org/repo"
    }
  }

  persona admin      "System Administrator"
  persona developer  "API Integrator"

  surface web  "Web Dashboard"
  surface cli  "Command Line"

  test_dirs ["tests/", "src/**/*.test.*"]

  coverage {
    threshold                95
    reports                  ["specforge-report.json"]
    require_violation_tests  true
    fail_on_unknown_ids      true
  }

  define research {
    id_prefix   "RES"
    attributes {
      outcome     enum [adr, behavior, deferred, rejected]
      related_adr ref? decision
      date        string
    }
  }
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name (string after `spec`). Used in generated output headers. |
| `version` | string | Format version of `.spec` files. Compiler checks compatibility. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `infix` | string | Legacy 2-4 uppercase letter code. Optional -- entity IDs use free-form identifiers. |
| `plugins` | string list | Installed plugin packages. |
| `providers` | block | Provider configurations for ref validation. |
| `persona` | sub-block(s) | Persona definitions for capability validation. |
| `surface` | sub-block(s) | Surface definitions for capability validation. |
| `test_dirs` | string list | Glob patterns for test directories. |
| `coverage` | block | Test coverage configuration. |
| `define` | sub-block(s) | Meta-schema for user-defined entity types. |

## Relationships

The `spec` block does not participate in the traceability chain. It is configuration, not a traced entity.

## Writing Rules

1. **One spec block per project** -- lives in `specforge.spec` at the project root.
2. **Plugins are string lists** -- use the full package name: `"@specforge/product"`, not `product`.
3. **Providers support aliases** -- multiple instances of the same provider use different alias names.
4. **Persona and surface are inline** -- declared directly in the spec block, not in separate files.
5. **Define blocks are for custom entities** -- only use when the extension-provided entity types are insufficient.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | Exactly one `spec` block across all `.spec` files. |
| I003 | Project `version` older than compiler version. |

## Examples

### Minimal

```spec
spec "my-api" {
  version "1.0"
}
```

### Software + Product

```spec
spec "my-service" {
  version "1.0"

  plugins [
    "@specforge/software",
    "@specforge/product",
  ]

  persona admin      "System Administrator"
  persona developer  "API Integrator"

  surface web  "Web Dashboard"
  surface api  "REST API"
}
```

### Full Configuration

```spec
spec "healthcare-platform" {
  version "1.0"

  plugins [
    "@specforge/software",
    "@specforge/product",
    "@specforge/governance",
  ]

  providers {
    gh "platform" {
      package "@specforge/gh"
      repo    "healthorg/platform"
    }
    jira "clinical" {
      package "@specforge/jira"
      project "CLIN"
      server  "https://healthorg.atlassian.net"
    }
  }

  persona clinician  "Healthcare Provider"
  persona patient    "Patient User"
  persona admin      "System Administrator"

  surface web     "Web Portal"
  surface mobile  "Mobile App"
  surface api     "HL7 FHIR API"

  test_dirs ["tests/", "services/**/tests/"]

  coverage {
    threshold                95
    require_violation_tests  true
    fail_on_unknown_ids      true
  }
}
```

## What NOT to Do

- Do not place the `spec` block in any file other than `specforge.spec` at the project root
- Do not declare more than one `spec` block across all files
- Do not reference entity IDs from within the spec block -- it is configuration only
- Do not put `use` directives inside the spec block -- `use` is file-level, not block-level

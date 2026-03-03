---
name: specforge-spec-block
description: "Write the spec root configuration block in specforge.spec. Declares project identity (name, infix, version), installed plugins, provider configurations, persona/surface definitions, coverage settings, code generation targets, and meta-schema define blocks. Use when initializing a new SpecForge project or modifying project-level configuration."
---

# SpecForge Spec Block

Rules and conventions for authoring the **`spec` root configuration block** in `specforge.spec`. The spec block is a singleton — exactly one per project — that declares project identity and configuration.

## When to Use

- Initializing a new SpecForge project (`specforge.spec`)
- Adding or removing plugins (`@specforge/product`, `@specforge/governance`)
- Configuring providers for external references (GitHub, Jira, Figma)
- Defining personas and surfaces for capability validation
- Setting up code generation targets
- Configuring test coverage thresholds
- Adding meta-schema `define` blocks for custom entity types

## Block Syntax

```spec
spec "project-name" {
  infix   "XX"
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

  gen typescript {
    out       "src/generated/"
    result    "hex-di"
    readonly  true
    naming    "camelCase"
    tests     "@specforge/vitest"
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
| `infix` | string | 2-4 uppercase letter code scoping all entity IDs (e.g., `"MS"` → `BEH-MS-001`). |
| `version` | string | Format version of `.spec` files. Compiler checks compatibility. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `plugins` | string list | Installed plugin packages. |
| `providers` | block | Provider configurations for ref validation. |
| `persona` | sub-block(s) | Persona definitions for capability validation. |
| `surface` | sub-block(s) | Surface definitions for capability validation. |
| `test_dirs` | string list | Glob patterns for test directories. |
| `coverage` | block | Test coverage configuration. |
| `gen` | block(s) | Code generation configuration per language. |
| `define` | sub-block(s) | Meta-schema for user-defined entity types. |

## Relationships

The `spec` block does not participate in the traceability chain. It is configuration, not a traced entity. It implicitly scopes all entities via the `infix` field.

## Writing Rules

1. **One spec block per project** — lives in `specforge.spec` at the project root.
2. **Infix is 2-4 uppercase letters** — choose a short, unique code for the project (e.g., `MS`, `HP`, `API`).
3. **Plugins are string lists** — use the full package name: `"@specforge/product"`, not `product`.
4. **Providers support aliases** — multiple instances of the same provider use different alias names.
5. **Persona and surface are inline** — declared directly in the spec block, not in separate files.
6. **Define blocks are for custom entities** — only use when the 16 built-in types are insufficient.
7. **Gen blocks are per-language** — each `gen` sub-block targets one language with its own output directory.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | Exactly one `spec` block across all `.spec` files. |
| I003 | Project `version` older than compiler version. |

## Examples

### Minimal

```spec
spec "my-api" {
  infix   "API"
  version "1.0"
}
```

### Core + Product

```spec
spec "my-service" {
  infix   "MS"
  version "1.0"

  plugins [
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
  infix   "HP"
  version "1.0"

  plugins [
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

  gen typescript {
    out       "packages/shared/src/generated/"
    result    "hex-di"
    readonly  true
    naming    "camelCase"
    tests     "@specforge/vitest"
  }
}
```

## What NOT to Do

- Do not place the `spec` block in any file other than `specforge.spec` at the project root
- Do not declare more than one `spec` block across all files
- Do not use lowercase or mixed-case infix — it must be 2-4 uppercase letters
- Do not reference entity IDs from within the spec block — it is configuration only
- Do not put `use` directives inside the spec block — `use` is file-level, not block-level

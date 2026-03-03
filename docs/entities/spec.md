# spec

> **Module:** `core`

## Purpose

The `spec` block is the **project root configuration** — a singleton that declares the project identity, format version, and optional configuration for test coverage and code generation.

Every SpecForge project has exactly one `spec` block, located in the root `specforge.spec` file. It answers: **"What project is this?"**

## ID Pattern

Singleton — no ID. There is exactly one `spec` block per project.

## Syntax

```spec
spec "my-service" {
  version "1.0"

  plugins [
    "@specforge/product",
    "@specforge/governance",
  ]

  providers {
    gh "work" {
      package "@specforge/gh"
      repo    "myorg/my-service"
    }
  }

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
}
```

## Fields

### Required

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name (the string after `spec`). Used in generated output headers. |
| `version` | string | Format version of the `.spec` files. The compiler checks compatibility on startup. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `namespace` | string | Optional namespace for cross-project references (e.g., `"@auth-service"`). |
| `display_prefix` | string | Optional prefix for human-readable reports (e.g., `"MS"`). Does not affect entity IDs. |
| `plugins` | string list | Installed plugin packages (e.g., `"@specforge/product"`, `"@specforge/governance"`). |
| `providers` | block | Provider configurations for external platform integrations. See [extension-model.md](../extension-model.md). |
| `test_dirs` | string list | Glob patterns for directories containing test files. Used by coverage scanning. |
| `persona` | sub-block(s) | Persona definitions. Validates that every `persona` in a `capability` block matches a defined persona. |
| `surface` | sub-block(s) | Surface definitions. Validates that every `surface` in a `capability` block matches a defined surface. |
| `define` | sub-block(s) | Meta-schema definitions for user-defined entity types beyond the core set. |
| `coverage` | block | Test coverage configuration (threshold, report paths, flags). |
| `gen` | block(s) | Code generation configuration per target language. |

### Coverage Sub-Block

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `threshold` | number | 0 | Minimum percentage of behaviors that must be covered by tests. |
| `reports` | string list | `["specforge-report.json"]` | Paths to `specforge-report.json` files to merge. |
| `require_violation_tests` | boolean | false | Every invariant must have at least one `violation()` test. |
| `fail_on_unknown_ids` | boolean | true | `spec("nonexistent_behavior")` in tests fails if the ID doesn't exist in `.spec` files. |

### Persona Sub-Block

Persona definitions validate that every `persona` referenced in a `capability` block matches a defined persona.

| Field | Type | Description |
|-------|------|-------------|
| `name` | identifier | Persona identifier (the keyword after `persona`). Used in `capability` blocks. |
| `title` | string | Human-readable persona name (the string after the identifier). |
| `description` | string | Optional description of the persona's role and responsibilities. |

### Surface Sub-Block

Surface definitions validate that every `surface` referenced in a `capability` block matches a defined surface.

| Field | Type | Description |
|-------|------|-------------|
| `name` | identifier | Surface identifier (the keyword after `surface`). Used in `capability` blocks. |
| `title` | string | Human-readable surface name (the string after the identifier). |
| `type` | string | Optional surface type (e.g., `webapp`, `terminal`, `mobile`, `api`). |

### Define Sub-Block (meta-schema)

The `define` mechanism allows user-defined entity types beyond the 16 core types. User-defined types get attribute validation, reference resolution, orphan detection, and LSP support. They do NOT get custom graph-level validators (those require the plugin API).

| Field | Type | Description |
|-------|------|-------------|
| `name` | identifier | The new entity type name (the keyword after `define`). |
| `attributes` | block | Attribute definitions with types: `string`, `enum [...]`, `ref? <entity_type>`. |

### Plugins Field

The `plugins` field is a string list of installed plugin packages:

```spec
plugins [
  "@specforge/product",
  "@specforge/governance",
]
```

Plugins extend the entity model with new block types, edge types, and validation rules. See [extension-model.md](../extension-model.md) for details.

### Providers Sub-Block

The `providers` block configures external platform integrations for `ref` entity validation and URL resolution:

```spec
providers {
  gh "work" {
    package "@specforge/gh"
    repo    "myorg/my-service"
  }

  jira "project" {
    package "@specforge/jira"
    project "PROJ"
    server  "https://myorg.atlassian.net"
  }
}
```

Each provider entry has:

| Field | Type | Description |
|-------|------|-------------|
| `alias` | identifier | Instance name (the string after the scheme). Supports multiple instances of the same provider. |
| `package` | string | The provider package to use (e.g., `"@specforge/gh"`). |
| Provider-specific fields | varies | Configuration fields defined by the provider (e.g., `repo`, `project`, `server`). |

See [extension-model.md](../extension-model.md) for the full provider model.

### Gen Sub-Block (per language)

| Field | Type | Description |
|-------|------|-------------|
| `out` | string | Output directory for generated files. |
| `result` | string | Result type library to use (e.g., `"hex-di"`, `"neverthrow"`, `"result"`). |
| `readonly` | boolean | Generate `readonly` fields in output types. |
| `naming` | string | Naming convention: `"camelCase"`, `"snake_case"`, `"PascalCase"`. |
| `tests` | string | Test plugin package name for generating test stubs. |
| `module` | string | Module path prefix (Go-specific). |
| `frozen` | boolean | Generate frozen/immutable types (Python-specific). |

## Relationships

The `spec` block does not participate in the traceability chain. It is configuration, not a traced entity.

It provides project-level configuration including optional `namespace` for cross-project references and `display_prefix` for report formatting.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | Exactly one `spec` block must exist across all `.spec` files in the project. |
| — | The `version` must be a valid semver-compatible version string. |
| I003 | If the project `version` is older than the compiler version, emit info diagnostic. |

## File Location

The `spec` block must be in `specforge.spec` at the project root. This file also serves as the spec root for import path resolution — `use invariants/data` resolves relative to the directory containing `specforge.spec`.

## Related Entities

The `spec` block does not participate in the traceability graph. It is root configuration. No entities reference it, and it references no entities.

## Examples

### Minimal

```spec
spec "my-api" {
  version "1.0"
}
```

### With Persona and Surface Definitions

```spec
spec "my-service" {
  version "1.0"

  persona admin       "System Administrator"
  persona developer   "API Integrator"
  persona viewer      "Read-Only User"

  surface web  "Web Dashboard"
  surface cli  "Command Line"
  surface api  "REST API"
}
```

### With Meta-Schema Define

```spec
spec "regulated-service" {
  version "1.0"

  define research {
    attributes {
      outcome     enum [adr, behavior, deferred, rejected]
      related_adr ref? decision
      date        string
    }
  }

  define compliance {
    attributes {
      regulation  string
      evidence    string
      status      enum [compliant, non_compliant, in_review]
    }
  }
}
```

### With Providers

```spec
spec "my-service" {
  version "1.0"

  plugins [
    "@specforge/product",
  ]

  providers {
    gh "main" {
      package "@specforge/gh"
      repo    "myorg/my-service"
    }

    gh "shared" {
      package "@specforge/gh"
      repo    "myorg/shared-libs"
    }

    jira "backend" {
      package "@specforge/jira"
      project "BACK"
      server  "https://myorg.atlassian.net"
    }
  }
}
```

### Full Configuration

```spec
spec "healthcare-platform" {
  version "1.0"

  namespace      "@healthcare"
  display_prefix "HP"

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
    reports [
      "services/auth/specforge-report.json",
      "services/billing/specforge-report.json",
      "services/patient/specforge-report.json",
    ]
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

  gen python {
    out       "services/ml/src/generated/"
    result    "result"
    frozen    true
    naming    "snake_case"
    tests     "@specforge/pytest"
  }
}
```

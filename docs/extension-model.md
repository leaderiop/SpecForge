# SpecForge Extension Model

SpecForge supports three distinct extension mechanisms — **plugins**, **providers**, and **generators** — each serving a different purpose in the ecosystem. This follows the Terraform model: a small stable core extended by composable, independently installable extensions.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CORE (8 entities)                         │
│  spec · invariant · behavior · feature · event · type · port · ref│
│  + meta-schema `define` mechanism                                │
├──────────────────────┬──────────────────┬───────────────────────┤
│   PLUGINS (entities) │ PROVIDERS (refs) │ GENERATORS (output)   │
│  @specforge/product  │  @specforge/gh   │  @specforge/gen-ts    │
│  @specforge/governance│ @specforge/jira │  @specforge/gen-py    │
│  community plugins   │  @specforge/figma│  @specforge/gen-go    │
│                      │  community provs │  community generators │
└──────────────────────┴──────────────────┴───────────────────────┘
```

## Comparison

| Aspect | Plugins | Providers | Generators |
|--------|---------|-----------|------------|
| **Extends** | Entity model (new block types, edges, validation) | `ref` entity (schemes, kinds, validation, URL resolution) | Output pipeline (new rendering formats) |
| **Declared in** | `spec` root `plugins` field | `spec` root `providers` block | `spec` root `gen` block |
| **Aliasing** | No (one instance per plugin) | Yes (multiple instances with aliases) | No (one config per language) |
| **Multiple instances** | No | Yes (`gh "work"`, `gh "oss"`) | No |
| **Affects graph** | Yes (adds nodes + edges) | No (only validates ref targets) | No (reads graph, produces files) |
| **Example** | `@specforge/governance` adds `decision`, `constraint`, `failure_mode` | `@specforge/gh` registers `gh.issue`, `gh.pr`, `gh.discussion` | `@specforge/gen-typescript` emits TypeScript interfaces |

## Plugins

Plugins extend the **entity model** — they add new block types, new edge types, and new validation rules. The two official plugins are `@specforge/product` (5 entities) and `@specforge/governance` (3 entities).

For full details on the plugin architecture, see [entity-model.md](entity-model.md).

### Key Properties

- Plugins register entity types and their field-name mappings with the core compiler
- Cross-plugin references use soft resolution (`I004` if plugin not installed)
- Each plugin registers its own validation rules (only fire when installed)
- Plugins are declared in the `spec` root `plugins` field

### Declaration

```spec
spec "my-service" {
  version "1.0"

  plugins [
    "@specforge/product",
    "@specforge/governance",
  ]
}
```

### CLI

```bash
specforge add @specforge/product        # install plugin
specforge remove @specforge/governance  # remove plugin
specforge plugins                       # list installed plugins
```

## Providers

Providers extend the **`ref` entity** — they register schemes, validate ref targets, resolve URLs, and optionally sync metadata. Providers are the bridge between SpecForge and external platforms (GitHub, Jira, Figma, etc.).

### How Providers Work

When a provider is installed:

1. **Scheme registration** — The provider registers one or more schemes (e.g., `gh`) with the core compiler
2. **Kind registration** — Each scheme registers its valid kinds (e.g., `gh.issue`, `gh.pr`, `gh.discussion`)
3. **Target validation** — The provider validates that ref identifiers match the expected pattern for each kind (e.g., `gh.issue` expects a numeric ID)
4. **URL resolution** — The provider can resolve ref IDs to full URLs (e.g., `gh.issue:42` → `https://github.com/owner/repo/issues/42`)

### Declaration

Providers are declared in a `providers` block inside the `spec` root. Each provider instance has an alias (for multi-instance support) and a package reference:

```spec
spec "my-service" {
  version "1.0"

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
}
```

### Multiple Instances

Unlike plugins, providers support multiple instances with different aliases. This is essential for projects that interact with multiple repositories or projects on the same platform:

```spec
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

  jira "platform" {
    package "@specforge/jira"
    project "PLAT"
    server  "https://myorg.atlassian.net"
  }
}
```

When multiple instances of the same provider exist, ref resolution checks all instances. The alias is for human documentation — the compiler resolves refs by trying all registered instances of a scheme.

### Provider Registration

A provider package registers the following with the core compiler:

| Registration | Description | Example |
|-------------|-------------|---------|
| **Scheme** | The provider's namespace in ref IDs | `gh` |
| **Kinds** | Valid resource types within the scheme | `issue`, `pr`, `discussion` |
| **ID patterns** | Regex patterns for valid identifiers per kind | `gh.issue` → `^\d+$` |
| **URL template** | Template for resolving ref IDs to URLs | `https://github.com/{repo}/issues/{id}` |
| **Validation rules** | Provider-specific validation logic | E011, E012 |

### Soft Resolution

When a ref uses a scheme not registered by any installed provider, the compiler emits `I005` (unknown provider scheme) — an info-level diagnostic, not an error. The ref is stored in the graph but not validated. This follows the same progressive enhancement model as cross-plugin references (`I004`).

```
info[I005]: Unknown provider scheme 'figma'
  ┌─ behaviors/auth.spec:5:9
  │
5 │   refs [figma.frame:abc123]
  │         ^^^^^ unknown scheme
  │
  = help: Install a Figma provider to enable validation
```

### Built-in Schemes

The core compiler ships with one built-in scheme that requires no provider:

| Scheme | Kind | Description |
|--------|------|-------------|
| `mermaid` | (none) | Inline Mermaid diagram reference. No external validation needed. |

### Terraform Parallel

The provider model is directly inspired by Terraform providers:

| Terraform | SpecForge |
|-----------|-----------|
| `provider "aws" { region = "us-east-1" }` | `gh "work" { package "@specforge/gh" repo "myorg/repo" }` |
| Resources: `aws_instance`, `aws_s3_bucket` | Kinds: `gh.issue`, `gh.pr`, `gh.discussion` |
| Multiple providers with aliases | Multiple providers with aliases |
| `terraform init` downloads providers | `specforge add` installs providers |
| Provider validates resource configs | Provider validates ref target formats |

### CLI

```bash
specforge add @specforge/gh              # install provider
specforge add @specforge/jira            # install provider
specforge remove @specforge/figma        # remove provider
specforge providers                      # list installed providers
```

## Generators

Generators extend the **output pipeline** — they read the in-memory graph and produce files in a target language or format. Official generators produce TypeScript, Python, and Go code; community generators can produce anything (OpenAPI specs, AsyncAPI docs, Terraform modules, etc.).

For full details on code generation, see [entity-model.md](entity-model.md) and the individual generator docs.

### Key Properties

- Generators are standalone executables (any language)
- They receive the graph as JSON on stdin and emit files on stdout
- They are configured in the `spec` root `gen` block
- They do not modify the graph — they are read-only consumers

### Declaration

```spec
spec "my-service" {
  version "1.0"

  gen typescript {
    out       "src/generated/"
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

### CLI

```bash
specforge gen typescript ./src/generated/    # run generator
specforge gen typescript --check             # drift detection
specforge verify typescript                  # adapter verification
```

## Extension Interaction

The three extension types are orthogonal — they can be combined freely:

```spec
spec "healthcare-platform" {
  version "1.0"

  // Plugins: extend the entity model
  plugins [
    "@specforge/product",
    "@specforge/governance",
  ]

  // Providers: extend ref validation
  providers {
    gh "main" {
      package "@specforge/gh"
      repo    "healthorg/platform"
    }

    jira "clinical" {
      package "@specforge/jira"
      project "CLIN"
      server  "https://healthorg.atlassian.net"
    }
  }

  // Generators: extend output formats
  gen typescript {
    out       "packages/shared/src/generated/"
    result    "hex-di"
    readonly  true
    naming    "camelCase"
    tests     "@specforge/vitest"
  }
}
```

A `ref gh.issue:42` in this project will:
1. Be validated by the `@specforge/gh` provider (E011 if invalid format, E012 if unknown kind)
2. Be resolvable to `https://github.com/healthorg/platform/issues/42`
3. Appear in generated traceability reports
4. Be navigable via LSP (hover shows title, click opens URL)

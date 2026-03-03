---
name: spec-libraries
description: "Author library specification files in a spec's libraries/ directory. Each file describes a code package (LIB-XX-NNN IDs) with its npm identity, port catalog, feature coverage, and dependency graph. Libraries are organized into family sub-folders mirroring the monorepo structure. Use when mapping code packages to features, documenting port definitions and adaptations, or bridging the gap between features (why) and actual npm packages (how)."
---

# Spec Libraries

Rules and conventions for authoring **library specification files** in a spec's `libraries/` directory. Library files describe *what code packages implement features* — they bridge features (the *why*) and actual npm packages in the monorepo by documenting port catalogs, dependency graphs, and feature coverage.

## When to Use

- Creating a new library specification for an npm package
- Documenting which features a library implements
- Cataloging ports defined and adapted by a library
- Mapping library dependencies within the monorepo
- Organizing libraries into families and maintaining `index.yaml`

## Directory Structure

Libraries use a **family sub-folder structure** because the relationship between family and library is 1:many — each library belongs to exactly one family, and families mirror the monorepo's package grouping.

```
libraries/
  index.yaml                    # Manifest of all families and libraries
  core/                         # Family sub-folder (mirrors packages/core/)
    LIB-XX-001-<name>.md
    LIB-XX-002-<name>.md
  flow/                         # Family sub-folder (mirrors libs/flow/)
    LIB-XX-010-<name>.md
  guard/                        # Family sub-folder (mirrors libs/guard/)
    LIB-XX-020-<name>.md
```

### index.yaml Schema

```yaml
# Libraries Index — LIB-XX-001 through LIB-XX-NNN
# "What code implements features" layer mapping packages to features

families:
  - name: core
    description: Core DI kernel packages
    libraries:
      - id: LIB-XX-001
        file: core/LIB-XX-001-di-kernel.md
        title: DI Kernel
        npm_name: "@hex-di/core"
        library_type: core
        status: active
      - id: LIB-XX-002
        file: core/LIB-XX-002-graph.md
        title: Graph Engine
        npm_name: "@hex-di/graph"
        library_type: core
        status: active
  - name: flow
    description: Flow state machine libraries
    libraries:
      - id: LIB-XX-010
        file: flow/LIB-XX-010-flow-core.md
        title: Flow Core
        npm_name: "@hex-di/flow"
        library_type: feature
        status: active
```

**Rules:**
- Every `.md` file in `libraries/` sub-folders MUST have a corresponding entry in `index.yaml`
- Every entry MUST have a corresponding file on disk at the path specified by `file:`
- No duplicate `id` values across all families
- Libraries within a family are listed in sequential ID order
- The `file:` path includes the family sub-folder prefix (e.g., `core/LIB-XX-001-di-kernel.md`)
- Family names match the sub-folder names on disk

## File Naming

- ID-prefixed: `LIB-XX-NNN-<name>.md`
- `LIB` = library prefix (always literal `LIB`)
- `XX` = 2-3 character package infix (e.g., `SF` for specforge, `GD` for guard)
- `NNN` = zero-padded sequential number
- Kebab-case name describing the library
- Files live inside their family sub-folder
- Examples: `core/LIB-SF-001-di-kernel.md`, `flow/LIB-SF-010-flow-core.md`

### Family ID Ranges

Each family reserves a block of sequential IDs to avoid cross-family collisions:

| Family | ID Range | Example |
|--------|----------|---------|
| core | 001-009 | LIB-SF-001 through LIB-SF-009 |
| flow | 010-019 | LIB-SF-010 through LIB-SF-019 |
| guard | 020-029 | LIB-SF-020 through LIB-SF-029 |
| saga | 030-039 | LIB-SF-030 through LIB-SF-039 |
| tooling | 040-049 | LIB-SF-040 through LIB-SF-049 |

Adjust ranges based on the spec's actual package count. The ranges above are guidelines, not rigid constraints.

## File Template

```markdown
---
id: LIB-XX-NNN
kind: library
title: "<Noun phrase — what the package is>"
status: active
npm_name: "@hex-di/<package>"
path: "packages/<path>"          # Relative path from monorepo root
library_type: core               # core | feature | adapter | integration | testing | tooling
family: core                     # Must match parent sub-folder name
features: [FEAT-XX-NNN, ...]
depends_on: [LIB-XX-NNN, ...]
ports_defined: [PortName, ...]
ports_adapted: [PortName, ...]
constraints: []
---

# <Title>

## Description

<1-2 paragraphs describing what this library does, its role in the architecture, and its primary consumers.>

## Port Catalog

### Ports Defined

| Port | Direction | Category | Description |
|------|-----------|----------|-------------|
| <PortName> | inbound / outbound | <domain/subcategory> | <what it abstracts> |

### Ports Adapted

| Port | Adapter | Description |
|------|---------|-------------|
| <PortName> | <AdapterName> | <what it provides> |

## Dependency Graph

| Dependency | Type | What It Provides |
|------------|------|-----------------|
| [LIB-XX-NNN](../core/LIB-XX-NNN-slug.md) | runtime / dev / peer | <what this library uses from it> |

## Feature Coverage

| Feature | Title | Coverage | Notes |
|---------|-------|----------|-------|
| [FEAT-XX-NNN](../../features/FEAT-XX-NNN-slug.md) | <title> | full / partial | <what behaviors this library implements> |

## Constraints

<Build constraints, platform requirements, bundle size notes, peer dependency requirements, etc.>
```

**Section inclusion rules:**

| Section | Required | Condition |
|---------|----------|-----------|
| Description | Always | Every library |
| Port Catalog | Always | Every library (may have empty Defined or Adapted sections) |
| Dependency Graph | Always | Every library depends on something (even if only dev deps) |
| Feature Coverage | Always | Every library implements features |
| Constraints | Conditional | When build/platform constraints exist |

## Content Rules

1. **YAML frontmatter** — Every library file MUST start with `---` frontmatter containing `id`, `kind: library`, `title`, `status`, `npm_name`, `path`, `library_type`, `family`, `features`, `depends_on`.
2. **Valid library_type** — Must be one of: `core`, `feature`, `adapter`, `integration`, `testing`, `tooling`.
3. **Feature refs must exist** — Every ID in `features[]` MUST reference an existing `FEAT-XX-NNN` in `features/`.
4. **Library dependency refs must exist** — Every ID in `depends_on[]` MUST reference an existing `LIB-XX-NNN` in `libraries/`.
5. **Family consistency** — The `family` frontmatter field MUST match the parent sub-folder name.
6. **npm_name accuracy** — The `npm_name` field should match the actual `name` field in the package's `package.json`.
7. **Port naming** — Port names in `ports_defined[]` and `ports_adapted[]` should use PascalCase and match the port names used in `types/` or `architecture/ports-and-adapters.md`.
8. **Unique IDs** — Every library has a unique `LIB-XX-NNN` ID. No duplicates across all families.
9. **Sequential numbering within family ranges** — New libraries get the next available number within their family's ID range.
10. **Noun-phrase titles** — Titles describe what the package is: "DI Kernel", "Flow Core", "Guard Policy Engine". Use noun phrases, not verb phrases.

## Cross-References

```markdown
# From library to features (in frontmatter):
features: [FEAT-SF-001, FEAT-SF-004]

# From library to other libraries (in frontmatter):
depends_on: [LIB-SF-001, LIB-SF-003]

# From library to features (in body):
| [FEAT-SF-001](../../features/FEAT-SF-001-graph-store.md) | Graph-First Knowledge Store | full | Implements all graph behaviors |

# From library to other libraries (in body):
| [LIB-SF-001](../core/LIB-SF-001-di-kernel.md) | runtime | Port resolution and adapter composition |

# From deliverable to libraries:
depends_on: [LIB-SF-001, LIB-SF-010]

# From other documents linking to a library:
See [LIB-SF-001](libraries/core/LIB-SF-001-di-kernel.md).

# From sub-documents linking up:
See [LIB-SF-001](../libraries/core/LIB-SF-001-di-kernel.md).
```

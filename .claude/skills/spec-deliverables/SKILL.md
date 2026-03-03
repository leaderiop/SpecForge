---
name: spec-deliverables
description: "Author deliverable specification files in a spec's deliverables/ directory. Each file describes what ships to users (DLV-XX-NNN IDs) — apps, services, CLIs, extensions — with capability maps, library dependencies, persona targeting, and deployment constraints. Use when documenting what products or artifacts are delivered, mapping deliverables to capabilities, or bridging the gap between user-facing capabilities and code libraries."
---

# Spec Deliverables

Rules and conventions for authoring **deliverable specification files** in a spec's `deliverables/` directory. Deliverable files describe *what ships to users* — they bridge capabilities (the *what users can do*) and libraries (the *what code implements*) by documenting concrete products, services, and tools that bundle capabilities for specific personas.

## When to Use

- Creating a new deliverable specification for a shippable product or service
- Documenting which capabilities a deliverable bundles
- Mapping deliverables to their library dependencies
- Defining persona targeting and deployment constraints for a deliverable
- Organizing deliverables and maintaining `index.yaml`

## Directory Structure

Deliverables use a **flat directory structure** because the relationship between deliverables and capabilities is many:many — a capability can appear in multiple deliverables, and a deliverable bundles multiple capabilities.

```
deliverables/
  index.yaml                    # Manifest of all deliverable files
  DLV-XX-001-<name>.md          # One file per deliverable
  DLV-XX-002-<name>.md
  ...
```

### index.yaml Schema

```yaml
# Deliverables Index — DLV-XX-001 through DLV-XX-NNN
# "What ships to users" layer bridging capabilities and libraries

entries:
  - id: DLV-XX-001
    file: DLV-XX-001-desktop-app.md
    title: Desktop Application
    deliverable_type: app
    status: active
  - id: DLV-XX-002
    file: DLV-XX-002-cli-tool.md
    title: CLI Tool
    deliverable_type: cli
    status: planned
```

**Rules:**
- Every `.md` file in `deliverables/` MUST have a corresponding entry in `index.yaml`
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- Deliverables are listed in sequential ID order

## File Naming

- ID-prefixed: `DLV-XX-NNN-<name>.md`
- `DLV` = deliverable prefix (always literal `DLV`)
- `XX` = 2-3 character package infix (e.g., `SF` for specforge, `GD` for guard)
- `NNN` = zero-padded sequential number
- Kebab-case name describing the deliverable
- Examples: `DLV-SF-001-desktop-app.md`, `DLV-SF-002-cli-tool.md`, `DLV-GD-001-web-dashboard.md`

## File Template

```markdown
---
id: DLV-XX-NNN
kind: deliverable
title: "<Noun phrase — what ships>"
status: active
deliverable_type: app          # app | service | extension | cli
capabilities: [UX-XX-NNN, ...]
depends_on: [LIB-XX-NNN, ...]
personas: [developer, team-lead]
constraints: []
roadmap_releases: [RM-NN, ...]
---

# <Title>

## Description

<1-2 paragraphs describing what this deliverable is, who it serves, and how it reaches users.>

## Capability Map

| Capability | Title | Surface | Notes |
|------------|-------|---------|-------|
| [UX-XX-NNN](../capabilities/UX-XX-NNN-slug.md) | <title> | <surface> | <role in this deliverable> |

## Library Dependencies

| Library | Title | Dependency Type | Notes |
|---------|-------|----------------|-------|
| [LIB-XX-NNN](../libraries/<family>/LIB-XX-NNN-slug.md) | <title> | direct / transitive | <what it provides> |

## Personas

<For each persona, describe how they interact with this deliverable and what value they get.>

## Constraints

<Deployment constraints, platform requirements, compatibility notes, size budgets, etc.>
```

**Section inclusion rules:**

| Section | Required | Condition |
|---------|----------|-----------|
| Description | Always | Every deliverable |
| Capability Map | Always | Every deliverable has capabilities |
| Library Dependencies | Always | Every deliverable depends on libraries |
| Personas | Conditional | When multiple personas use the deliverable differently |
| Constraints | Conditional | When deployment/platform constraints exist |

## Content Rules

1. **YAML frontmatter** — Every deliverable file MUST start with `---` frontmatter containing `id`, `kind: deliverable`, `title`, `status`, `deliverable_type`, `capabilities`, `depends_on`.
2. **Valid deliverable_type** — Must be one of: `app`, `service`, `extension`, `cli`.
3. **Capability refs must exist** — Every ID in `capabilities[]` MUST reference an existing `UX-XX-NNN` in `capabilities/`.
4. **Library dependency refs must exist** — Every ID in `depends_on[]` MUST reference an existing `LIB-XX-NNN` in `libraries/`.
5. **Noun-phrase titles** — Titles describe what ships: "Desktop Application", "CLI Tool", "VS Code Extension". Use noun phrases, not verb phrases.
6. **Unique IDs** — Every deliverable has a unique `DLV-XX-NNN` ID. No duplicates across the entire `deliverables/` directory.
7. **Sequential numbering** — New deliverables get the next available number in the sequence.
8. **Personas as arrays** — Always use YAML arrays (`[developer, team-lead]`), not comma-separated strings.
9. **Roadmap refs are optional** — `roadmap_releases[]` entries reference `RM-NN` phases when the deliverable has a release timeline.
10. **No duplicate capability refs** — A capability should not appear twice in the same deliverable's `capabilities[]` list.

## Cross-References

```markdown
# From deliverable to capabilities (in frontmatter):
capabilities: [UX-SF-001, UX-SF-004, UX-SF-009]

# From deliverable to libraries (in frontmatter):
depends_on: [LIB-SF-001, LIB-SF-005]

# From deliverable to capabilities (in body):
| [UX-SF-001](../capabilities/UX-SF-001-run-predefined-flow.md) | Run a Predefined Flow | CLI | Primary use case |

# From deliverable to libraries (in body):
| [LIB-SF-001](../libraries/core/LIB-SF-001-graph-store.md) | Graph Store | direct | Core data layer |

# From other documents linking to a deliverable:
See [DLV-SF-001](deliverables/DLV-SF-001-desktop-app.md).

# From sub-documents linking up:
See [DLV-SF-001](../deliverables/DLV-SF-001-desktop-app.md).
```

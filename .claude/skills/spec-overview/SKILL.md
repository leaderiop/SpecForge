---
name: spec-overview
description: "Author the overview.md document for a spec. Covers mission paragraph, numbered principles, and document map linking to every file in the spec directory. Use when creating a new spec overview, auditing an existing overview for completeness, or updating the document map after adding new spec files."
---

# Spec Overview

Rules and conventions for authoring the `overview.md` document in a spec. The overview is lean: mission paragraph, numbered principles, and a document map. It is not a detailed API reference.

## When to Use

- Creating a new spec (overview.md is always the first file)
- Auditing an existing overview for completeness
- Updating the document map after adding new spec files
- Promoting a spec from Stub to Technical-only tier

## File Template

```markdown
---
kind: overview
package: "@hex-di/<name>"
status: Draft
version: "N.N"
---

# <Package Name>

**<One-line elevator pitch.>**

---

## Mission

<Single paragraph: what problem this package solves.>

## Principles

1. **<Principle name>** -- <One-sentence explanation.>
2. ...

---

## Document Map

### Behaviors

| File | IDs | Domain |
|------|-----|--------|
| [behaviors/BEH-XX-001-<name>.md](...) | BEH-XX-001--008 | <Domain> |

### Features              (if applicable)

| File | ID | Domain |
|------|-----|--------|
| [features/FEAT-XX-001-<name>.md](...) | FEAT-XX-001 | <Domain> |

### Capabilities          (if applicable)

| File | ID | Persona |
|------|-----|---------|
| [capabilities/UX-XX-001-<name>.md](...) | UX-XX-001 | <Persona> |

### Deliverables          (if applicable)

| File | ID | Type |
|------|-----|------|
| [deliverables/DLV-XX-001-<name>.md](...) | DLV-XX-001 | app/service/cli/extension |

### Libraries             (if applicable)

| File | ID | Family |
|------|-----|--------|
| [libraries/core/LIB-XX-001-<name>.md](...) | LIB-XX-001 | core |

### Types              (if applicable)
### Architecture       (if applicable)
### Governance         (invariants, decisions, glossary, traceability, risk, roadmap, process)
### Decisions          (ADR table with Title and Status columns)
```

## Content Rules

1. **YAML frontmatter** — The overview.md MUST start with `---` frontmatter containing `kind: overview`, `package`, `status`, `version`. The `**Package:**`, `**Status:**`, `**Spec Version:**` lines are REMOVED from prose — that metadata lives in frontmatter.
2. **No API surface tables** — Those belong in behavior files and type files.
2. **No source file maps** — The codebase is the source of truth for file locations.
3. **Document map links to every file** — The document map links to every file in the spec directory.
4. **Principles capture constraints** — The principles section captures architectural constraints that all spec documents must respect.
5. **Lean prose** — The overview should be scannable in under 2 minutes.

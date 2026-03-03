---
name: spec-types
description: "Author domain TypeScript interface files in a spec's types/ directory. Each file documents interfaces for a domain with readonly fields, _tag discriminants, and behavior cross-references. Use when creating type specifications, auditing type completeness, or documenting domain interfaces."
---

# Spec Types

Rules and conventions for authoring **domain TypeScript interface files** in a spec's `types/` directory. Distinct from `type-system/` which covers compile-time safety patterns.

## When to Use

- Creating type specifications for domain interfaces
- Auditing type files for completeness (missing readonly, missing _tag)
- Documenting new domain interfaces
- Checking for duplicate type definitions across files

## Directory Structure

```
types/
  index.yaml                    # Manifest of all type files
  <domain>.md                   # One file per domain
```

### index.yaml Schema

```yaml
kind: types
package: "@hex-di/<name>"
entries:
  - id: TYPE-001
    file: graph.md
    title: Graph Types
    status: active              # active | draft | deprecated
    domain: graph
  - id: TYPE-002
    file: errors.md
    title: Error Types
    status: active
    domain: errors
```

**Rules:**
- Every `.md` file in `types/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- One file per domain, not one file per type

## File Naming

- Kebab-case domain name: `graph.md`, `agent.md`, `flow.md`, `errors.md`, `ports.md`
- One file per domain, not one file per type

## File Template

```markdown
---
id: TYPE-NNN
kind: types
title: "<Domain> Types"
status: active
behaviors: [BEH-XX-NNN]
adrs: []
---

# <Domain> Types

TypeScript interfaces for the <domain> domain.

---

## <TypeName>

` ` `typescript
interface <TypeName> {
  readonly field: Type;
  ...
}
` ` `

<Prose explaining the type's purpose, constraints, and usage context.>

---
```

## Content Rules

1. **YAML frontmatter** — Every types file MUST start with `---` frontmatter containing `id`, `kind: types`, `title`, `status`, `behaviors`, `adrs`. The `**Source behaviors:**` line is REMOVED from prose — that metadata lives in frontmatter.
2. **Readonly fields** — All interfaces use `readonly` fields.
2. **Unique _tag discriminants** — All error types have a unique `_tag` discriminant.
3. **Behavior cross-references** — Cross-reference behavior files that use these types.
4. **No duplicates** — No duplicate type definitions across `types/` files.
5. **Errors file** — `types/errors.md` collects all error types with their `_tag` discriminants.

## Cross-References

```markdown
# From types to behaviors:
**Source behaviors:** [BEH-XX-001-graph-ops.md](../behaviors/BEH-XX-001-graph-ops.md)

# From behaviors to types (in header):
**Types:** [types/graph.md](../types/graph.md)

# From architecture to types:
See [types/graph.md](../types/graph.md)
```

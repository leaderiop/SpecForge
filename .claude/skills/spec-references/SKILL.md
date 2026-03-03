---
name: spec-references
description: "Author external reference documents in a spec's references/ directory. Contains an index.md navigator and subdirectories per external tool or resource used as implementation reference. Use when documenting external tools, APIs, or resources that inform the implementation."
---

# Spec References

Rules and conventions for authoring **external reference documents** in a spec's `references/` directory. Contains an index navigator and subdirectories per tool.

## When to Use

- Documenting external tools or APIs used as implementation reference
- Creating reference material for third-party integrations
- Organizing external documentation that informs the spec

## Directory Structure

```
references/
  index.yaml                    # Manifest of all reference entries
  index.md                      # Navigation index
  <tool>/                       # One subdirectory per external tool
    overview.md
    <topic>.md
```

### index.yaml Schema

```yaml
kind: references
package: "@hex-di/<name>"
entries:
  - id: REF-001
    file: mermaid/overview.md
    title: Mermaid Reference
    status: active              # active | draft | deprecated
    tool: mermaid
  - id: REF-002
    file: effect/overview.md
    title: Effect Reference
    status: active
    tool: effect-ts
```

**Rules:**
- Every tool subdirectory MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries

## Index File Template (index.md)

```markdown
# External References

Reference documentation for external tools and resources used in this implementation.

| Tool | Directory | Purpose |
|------|-----------|---------|
| [Mermaid](./mermaid/overview.md) | `references/mermaid/` | Diagram rendering syntax |
| [Effect](./effect/overview.md) | `references/effect/` | Functional effect system patterns |
```

## Content Rules

1. **Frontmatter exemption** — Files in `references/` are EXCLUDED from the YAML frontmatter requirement. They contain external reference content.
2. **Implementation reference only** — Reference docs capture information needed for implementation, not general tool documentation.
2. **Subdirectory per tool** — Each external tool gets its own subdirectory.
3. **Index is navigator** — The index.md provides quick navigation, not content.

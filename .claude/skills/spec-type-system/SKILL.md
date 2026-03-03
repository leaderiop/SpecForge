---
name: spec-type-system
description: "Author compile-time safety pattern files in a spec's type-system/ directory. Covers phantom-brands.md and structural-safety.md conventions for documenting type-level safety guarantees. Use when documenting phantom brands, structural safety patterns, or compile-time type constraints."
---

# Spec Type System

Rules and conventions for authoring **compile-time safety pattern files** in a spec's `type-system/` directory. Documents patterns too large or important for inline ADR treatment. Distinct from `types/` which covers domain interfaces.

## When to Use

- Documenting phantom-branded scalar types
- Documenting structural type incompatibility patterns
- Creating compile-time safety guarantees documentation
- Auditing type system docs for invariant and ADR cross-references

## Directory Structure

```
type-system/
  index.yaml                    # Manifest of all type system files
  phantom-brands.md             # Phantom-branded scalar types
  structural-safety.md          # Structural type incompatibility patterns
```

### index.yaml Schema

```yaml
kind: type-system
package: "@hex-di/<name>"
entries:
  - id: TS-001
    file: phantom-brands.md
    title: Phantom Brands
    status: active              # active | draft | deprecated
  - id: TS-002
    file: structural-safety.md
    title: Structural Safety
    status: active
```

**Rules:**
- Every `.md` file in `type-system/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk

## Canonical File Types

### phantom-brands.md

Documents all phantom-branded scalar types (e.g., `number & { [BrandSymbol]: true }`).

**Must cover:**
- The unique symbol intersection pattern
- Cross-domain assignment blocking
- Covariant widening to the base type
- Arithmetic widening
- Validated vs identity branding utilities
- Cascading API table

### structural-safety.md

Documents structural type incompatibility patterns.

**Must cover:**
- Structural irresettability
- Structural incompatibility
- Port intersection types
- Opaque discriminated unions

## Content Rules

1. **YAML frontmatter** — Every type-system file MUST start with `---` frontmatter containing `id`, `kind: type-system`, `title`, `status`, `invariants`, `adrs`.
2. **Invariant links** — Both files must link to the invariants they enforce.
2. **ADR links** — Both files must link to the ADRs that justify them.
3. **Distinct from types/** — Never mix domain type definitions with compile-time safety patterns.

## Cross-References

```markdown
# From type system to invariants:
**Enforces:** [INV-XX-3](../invariants/INV-XX-3-<name>.md) -- <Invariant Name>

# From type system to ADRs:
**Justified by:** [ADR-005](../decisions/ADR-005-<topic>.md)

# From behaviors to type system:
See [type-system/phantom-brands.md](../type-system/phantom-brands.md)
```

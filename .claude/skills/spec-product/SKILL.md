---
name: spec-product
description: "Author product positioning documents in a spec's product/ directory. Covers pitch.md with target audience, value proposition, competitive positioning, and go-to-market strategy. Use when the spec represents a product with a market pitch."
---

# Spec Product

Rules and conventions for authoring **product positioning documents** in a spec's `product/` directory. Optional — use when the spec represents a product with a market pitch.

## When to Use

- Creating product positioning for a package or library
- Writing a pitch document for stakeholder communication
- Documenting target audience and value proposition

## Directory Structure

```
product/
  index.yaml                    # Manifest of all product files
  pitch.md                      # Product pitch document
```

### index.yaml Schema

```yaml
kind: product
package: "@hex-di/<name>"
entries:
  - id: PROD-001
    file: pitch.md
    title: Product Pitch
    status: active              # active | draft | deprecated
```

## File Template (pitch.md)

```markdown
# <Product Name> — Product Pitch

## Target Audience

<Who is this for? Developer personas, team roles, organizational contexts.>

## Value Proposition

<What unique value does this product deliver? What pain does it eliminate?>

## Competitive Positioning

<How does this differ from alternatives? What trade-offs does it make differently?>

## Go-to-Market Strategy

<How will users discover, evaluate, and adopt this product?>
```

## Content Rules

1. **YAML frontmatter** — Every product file MUST start with `---` frontmatter containing `id`, `kind: product`, `title`, `status`.
2. **Focus on value** — Lead with the problem being solved, not the technical approach.
2. **Honest positioning** — Acknowledge trade-offs and when alternatives are better suited.
3. **Measurable claims** — Back up value propositions with concrete examples or metrics.

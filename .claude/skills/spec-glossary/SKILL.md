---
name: spec-glossary
description: "Author the glossary.md document for a spec. Groups related terms in logical sections with bold definitions and BEH reference patterns. Use when creating a glossary, adding terms, or auditing glossary completeness against behavior files."
---

# Spec Glossary

Rules and conventions for authoring the `glossary.md` document in a spec. The glossary groups related terms in logical sections rather than a flat alphabetical list.

## When to Use

- Creating a glossary for a spec
- Adding new domain terms after writing behavior files
- Auditing glossary completeness against behavior files
- Reorganizing terms into better logical groupings

## File Template

```markdown
# Glossary

Domain terminology used throughout the <Package Name> specification.

## <Section Name>

**<Term>**
<Definition. 1-3 sentences. Reference behavior IDs where the term is formally specified.>

**<Term>**
<Definition.>

## <Next Section>
...
```

## Content Rules

1. **YAML frontmatter** — The glossary.md MUST start with `---` frontmatter containing `kind: glossary`, `package`, `status`.
2. **Grouped sections** — Group related terms under `## <Section Name>` headings (e.g., "Core Concepts", "Flow Terminology", "Agent Roles").
2. **Bold term names** — Bold the term name, followed by the definition on the same or next line.
3. **BEH references** — Reference specific behavior IDs (e.g., `See [BEH-SF-033](./behaviors/BEH-SF-033-blackboard.md)`) when the term has a formal contract.
4. **Concise definitions** — Keep definitions to 1-3 sentences. If a term needs more explanation, the behavior file is the right place.
5. **No duplication** — Don't duplicate behavior spec content. The glossary provides quick lookup, not full contracts.
6. **No flat alphabetical list** — Always group terms in logical sections.

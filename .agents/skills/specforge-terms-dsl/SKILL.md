---
name: specforge-terms-dsl
description: "Write term blocks in .spec DSL files (@specforge/product plugin). Each term declares a glossary entry as an individual entity with free-form snake_case IDs, containing definitions, aliases, context notes, and cross-references. Use when establishing precise domain vocabulary to prevent terminology drift."
---

# SpecForge Terms DSL

Rules and conventions for authoring **`term` blocks** in `.spec` files. Terms establish the ubiquitous language -- precise definitions that prevent terminology drift. Each term is an individual entity in the graph.

**Requires:** `@specforge/product` plugin.

## When to Use

- Defining terms that have project-specific meanings
- Resolving terminology ambiguity between team members
- Documenting aliases (different words for the same concept)
- Cross-referencing terms to the entities that use them

## Block Syntax

```spec
term committed_write "committed write" {
  definition """
    A write operation that has been acknowledged to the caller
    AND durably persisted to the primary datastore.
  """
  aliases    ["persisted mutation", "durable write"]
  see_also   [data_persistence, create_user]
}

term session "session" {
  definition """
    A materialized, mutable view of the in-memory graph
    scoped to a single compilation unit. Not an HTTP session.
  """
}
```

## Fields Reference

### Required

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Canonical term name (string after the entity ID). The preferred form. |
| `definition` | string / triple-string | Precise meaning. Clear enough that two readers agree. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `aliases` | string[] @optional | Alternative forms that mean the same thing. |
| `see_also` | EntityId[] @optional | Cross-references to related entities. |
| `tags` | string[] @optional | Faceted filtering tags. |
| `refs` | reference list | External references linked to this term. |

## Relationships

Terms participate in the graph as individual entities. The `see_also` field creates edges to referenced entities.

### Outgoing edges

| To | Edge Type | Meaning |
|----|-----------|---------|
| any entity | `see_also` | Cross-reference to related entity |

### Incoming edges

| From | Edge Type | Meaning |
|------|-----------|---------|
| (none by default) | -- | Terms are typically leaf nodes unless referenced by other entities |

## Writing Rules

1. **Individual entities** -- each term is a standalone entity with its own ID.
2. **Snake_case IDs** -- derive from the term name (e.g., "committed write" -> `committed_write`).
3. **Add terms when meaning is ambiguous** -- if two people might interpret a word differently, define it.
4. **Definitions are self-contained** -- understandable without reading other definitions.
5. **Use `aliases`** -- capture the different ways people say the same thing.
6. **`see_also` references create graph edges** -- they link to entities and create compiler-tracked edges.
7. **Skip obvious terms** -- "function", "variable", "database" do not need term entries.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | No two terms share the same entity ID. |
| E001 | Every ID in `see_also` must resolve to an existing entity. |
| I010 | Orphan term -- term not referenced by any entity's description or contract text. |

## Examples

### Domain Terms

```spec
term committed_write "committed write" {
  definition """
    A write operation that has been acknowledged to the caller
    AND durably persisted to the primary datastore. The write
    is guaranteed to survive process restarts.
  """
  aliases ["persisted mutation", "durable write", "acknowledged write"]
  see_also [data_persistence, create_user]
}

term eventual_consistency "eventual consistency" {
  definition """
    A consistency model where reads may return stale data
    for a bounded period after a write.
  """
}
```

### Technical Terms

```spec
term port_term "port" {
  definition """
    An interface boundary between the domain and the outside world.
    Inbound ports define what the system offers; outbound ports
    define what the system requires.
  """
  aliases ["interface", "boundary"]
  see_also [hexagonal_architecture]
}

term result_type "Result type" {
  definition """
    A type that represents either success or failure with typed errors.
    All port methods return Result types instead of throwing exceptions.
  """
  aliases ["ResultAsync", "Result monad"]
}
```

## What NOT to Do

- Do not write terms without the `@specforge/product` plugin installed
- Do not add standard programming terms that need no clarification
- Do not use terms for implementation details that don't appear in spec entities
- Do not duplicate term definitions -- each entity ID must be unique

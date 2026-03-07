---
name: specforge-glossary-dsl
description: "Write the glossary block in .spec DSL files (@specforge/product plugin). The glossary is a singleton block defining the project's ubiquitous language with term entries containing definitions, aliases, context notes, and cross-references. Use when establishing precise domain vocabulary to prevent terminology drift."
---

# SpecForge Glossary DSL

Rules and conventions for authoring the **`glossary` block** in `.spec` files. The glossary is a singleton that establishes the ubiquitous language -- precise definitions that prevent terminology drift.

**Requires:** `@specforge/product` plugin.

## When to Use

- Defining terms that have project-specific meanings
- Resolving terminology ambiguity between team members
- Documenting aliases (different words for the same concept)
- Cross-referencing terms to the entities that use them

## Block Syntax

```spec
glossary {
  term "committed write" {
    definition """
      A write operation that has been acknowledged to the caller
      AND durably persisted to the primary datastore.
    """
    aliases    ["persisted mutation", "durable write"]
    see        [data_persistence, create_user]
  }

  term "session" {
    definition """
      A materialized, mutable view of the in-memory graph
      scoped to a single compilation unit. Not an HTTP session.
    """
    context "Specific to SpecForge compiler internals."
    see     [session_architecture]
  }
}
```

## Fields Reference

### Glossary Block

Singleton -- no ID. One `glossary` block per project.

### Term Sub-Block

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Canonical term (string after `term`). The preferred form. |
| `definition` | string / triple-string | Precise meaning. Clear enough that two readers agree. |
| `aliases` | string list | Alternative forms that mean the same thing. |
| `context` | string | Disambiguation or usage notes when the term differs from common usage. |
| `see` | reference list | Cross-references to entities (invariants, behaviors, decisions, etc.). |
| `refs` | reference list | External references linked to this term. |

## Relationships

The glossary does not participate in the traceability graph as edges. The `see` field is informational -- it helps navigation but does not create compiler-tracked edges.

## Writing Rules

1. **One glossary per project** -- singleton block, like `spec`.
2. **Add terms when meaning is ambiguous** -- if two people might interpret a word differently, define it.
3. **Definitions are self-contained** -- understandable without reading other definitions.
4. **Use `aliases`** -- capture the different ways people say the same thing.
5. **Use `context` for disambiguation** -- when your project uses a term differently from common usage.
6. **`see` references are informational** -- they link to entities but do not create graph edges.
7. **Skip obvious terms** -- "function", "variable", "database" do not need glossary entries.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | No two terms share the same canonical name within the glossary. |
| E001 | Every ID in `see` must resolve to an existing entity. |

## Examples

### Minimal Glossary

```spec
glossary {
  term "spec" {
    definition "A .spec file containing entity declarations in the SpecForge DSL."
  }

  term "entity" {
    definition "A named, typed node in the SpecForge graph -- behavior, invariant, feature, etc."
  }
}
```

### Domain-Specific Glossary

```spec
glossary {
  term "committed write" {
    definition """
      A write operation that has been acknowledged to the caller
      AND durably persisted to the primary datastore. The write
      is guaranteed to survive process restarts.
    """
    aliases ["persisted mutation", "durable write", "acknowledged write"]
    see     [data_persistence, create_user]
  }

  term "eventual consistency" {
    definition """
      A consistency model where reads may return stale data
      for a bounded period after a write.
    """
    context "Our read models are eventually consistent with a 5-second window."
    see     [event_sourcing_for_audit]
  }

  term "RPN" {
    definition """
      Risk Priority Number. Calculated as Severity x Occurrence x Detection
      in an FMEA analysis. Higher RPN = higher risk priority.
    """
    aliases ["risk priority number"]
    see     [write_acknowledged_but_lost, email_uniqueness_bypass]
  }
}
```

### Technical Glossary

```spec
glossary {
  term "port" {
    definition """
      An interface boundary between the domain and the outside world.
      Inbound ports define what the system offers; outbound ports
      define what the system requires.
    """
    aliases   ["interface", "boundary"]
    context   "In SpecForge, 'port' means a hexagonal architecture port, not a network port."
    see       [hexagonal_architecture]
  }

  term "Result type" {
    definition """
      A type that represents either success or failure with typed errors.
      All port methods return Result types instead of throwing exceptions.
    """
    aliases   ["ResultAsync", "Result monad"]
    context   "TypeScript uses ResultAsync, Python uses Result[T, E], Go uses (T, error)."
    see       [result_types]
  }
}
```

## What NOT to Do

- Do not write a glossary without the `@specforge/product` plugin installed
- Do not add standard programming terms that need no clarification
- Do not use the glossary for implementation details that don't appear in spec entities
- Do not duplicate term definitions -- each canonical name must be unique
- Do not treat `see` references as compiler-tracked edges -- they are informational only

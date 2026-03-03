# glossary

> **Module:** `@specforge/product`

## Purpose

A `glossary` is a **structured vocabulary definition** — a singleton block that establishes the ubiquitous language of the project. It defines terms, their precise meanings, aliases, and cross-references to the entities that use them.

It answers: **"What do our terms mean?"**

Ambiguous terminology is the #1 source of specification bugs. When one behavior says "committed write" and another says "persisted mutation," the glossary makes explicit whether these are the same thing. It prevents the drift between how different team members (or different parts of the spec) use the same words.

## ID Pattern

Singleton — no ID. There is exactly one `glossary` block per project, similar to `spec`.

Individual terms within the glossary are not addressable entities with IDs. They are structured entries within the glossary block.

## Syntax

```spec
glossary {
  term "committed write" {
    definition """
      A write operation that has been acknowledged to the caller
      AND durably persisted to the primary datastore. The write
      is guaranteed to survive process restarts.
    """
    aliases    ["persisted mutation", "durable write"]
    see        [data_persistence, create_user]
  }

  term "session" {
    definition """
      A materialized, mutable view of the in-memory graph
      scoped to a single compilation unit. Not an HTTP session.
    """
    context "This term is specific to the SpecForge compiler internals. See use_event_sourcing."
    see     [use_event_sourcing]
  }

  term "invariant violation" {
    definition """
      A state where a runtime guarantee no longer holds.
      An invariant violation means the system is fundamentally broken,
      not merely degraded or slow.
    """
    aliases ["broken invariant"]
    see     [data_persistence, email_uniqueness, audit_integrity]
  }
}
```

## Fields

### Glossary Block

The glossary block contains one or more `term` sub-blocks.

### Term Sub-Block

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | The canonical term (the string after `term`). This is the preferred form. |
| `definition` | string or triple-string | The precise meaning of the term. Should be clear enough that two readers arrive at the same understanding. |
| `aliases` | string list | Alternative forms of this term that mean the same thing. Listed so the team knows they are synonyms. |
| `context` | string | Optional disambiguation or usage notes. Use when a term has a specific meaning in this project that differs from common usage. |
| `see` | reference list | Cross-references to entities (invariants, behaviors, decisions, etc.) that use or define this term. |
| `refs` | reference list | External references (issues, tickets, diagrams) related to this term. |

## Relationships

The glossary does not participate in the traceability graph as edges. The `see` field is informational — it helps readers navigate from a term to the entities that use it, but the compiler does not create graph edges for glossary terms.

This is a deliberate design choice: glossary terms are documentation aids, not traced entities. Adding them to the graph would create noise without compiler value.

## Validation Rules

| Code | Rule |
|------|------|
| E002 | No two terms may share the same canonical name within the glossary. |
| E001 | Every ID in `see` must resolve to an existing entity. |
| — | Alias overlap is allowed (two terms may share an alias if they are related). |

## Design Guidance

### When to Add a Term

Add a glossary term when:
- A word has a specific meaning in your project that differs from common usage
- Two team members might interpret a term differently
- A term appears in multiple behaviors or invariants
- The term is jargon that new team members won't know

### When NOT to Add a Term

Skip glossary terms for:
- Standard programming concepts (e.g., "function", "variable", "database")
- Terms whose meaning is obvious from context
- Implementation details that don't appear in spec entities

### Writing Good Definitions

Definitions should be:
- **Self-contained** — understandable without reading other definitions
- **Precise** — two readers arrive at the same understanding
- **Concise** — as short as possible while being unambiguous
- **Example-grounded** — include a concrete example when the concept is abstract

### Aliases

Aliases capture the different ways people say the same thing:

```spec
term "behavior" {
  definition "A behavioral contract specifying what the system does in a specific situation."
  aliases    ["behavioral contract", "behavior spec", "BEH"]
}
```

This prevents confusion when someone says "behavioral contract" in a meeting and someone else says "behavior spec" — the glossary confirms they mean the same thing.

### Context Field

Use `context` for disambiguation:

```spec
term "session" {
  definition "A materialized, mutable view of the in-memory graph."
  context    "In SpecForge, 'session' does NOT refer to HTTP sessions or user sessions."
}
```

## File Location

The glossary can live in any `.spec` file, but by convention it should be in `glossary.spec` at the spec root or in a `governance/glossary.spec` file.

## Related Entities

The glossary does not participate in the traceability graph. The `see` field is informational — it helps readers navigate to related entities but does not create compiler-tracked edges. The `refs` field on individual terms links to external references (e.g., `[gh.issue:42]`) via the `links_to` edge.

## Examples

### Minimal Glossary

```spec
glossary {
  term "spec" {
    definition "A .spec file containing entity declarations in the SpecForge DSL."
  }

  term "namespace" {
    definition "A logical grouping scope used to organize entities within a project."
    see [spec_root]
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
      for a bounded period after a write. All replicas will
      converge to the same state within the consistency window.
    """
    context "Our read models are eventually consistent with a 5-second window. See event_sourcing_audit."
    see     [event_sourcing_audit]
  }

  term "idempotency key" {
    definition """
      A client-provided unique identifier attached to a mutating request.
      If the same key is sent twice, the system MUST return the same result
      without performing the operation again.
    """
    aliases ["idempotency token", "request ID"]
    see     [place_order, idempotent_orders]
  }

  term "RPN" {
    definition """
      Risk Priority Number. Calculated as Severity x Occurrence x Detection
      in an FMEA analysis. Higher RPN = higher risk priority.
    """
    aliases ["risk priority number"]
    see     [write_loss, email_race]
  }

  term "orphan" {
    definition """
      An entity that exists in the spec but is not referenced by any
      higher-level entity. An orphan behavior is not in any feature.
      An orphan feature is not in any capability.
    """
    context "Orphans emit compiler warnings (W001, W002). They are not errors — they may be work in progress."
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
    context   "In SpecForge, 'port' specifically means a hexagonal architecture port, not a network port."
    see       [hexagonal_arch]
  }

  term "adapter" {
    definition """
      A concrete implementation of a port. Adapters connect the domain
      to real infrastructure: databases, APIs, message brokers.
    """
    aliases   ["implementation", "driver"]
    context   "Adapters are hand-written code, not generated. Ports are generated; adapters implement them."
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

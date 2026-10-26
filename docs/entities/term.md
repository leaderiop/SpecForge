# term

> **Declared by `@specforge/product`**, one of the builtin extensions. `term` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `term` is a glossary entry kept as an entity: a word or phrase, its precise meaning, its aliases and the terms
related to it. It answers: **"What does this term mean?"**

Ambiguous words cause specification bugs. When one document says "committed write" and another "persisted
mutation", a term makes explicit whether they are the same thing. This works for any vocabulary: software,
pharma ("batch release", "deviation"), marketing ("qualified lead").

## Syntax

```spec
term committed_write "Committed Write" {
  definition """
    A write operation that has been acknowledged to the caller
    AND durably persisted to the primary datastore.
  """
  aliases  ["persisted mutation", "durable write"]
  see_also [eventual_consistency]
}

term eventual_consistency "Eventual Consistency" {
  definition "Reads may return stale data for a bounded period after a write."
  context    "Our read models converge within 5 seconds."
}
```

`see_also` names other terms only: a reference to another kind is E022. `module` places the term in a
[module](module.md)'s vocabulary.

## Fields

Every `term` has an id (the word after `term`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `definition` | string | The precise meaning of this term in the project's domain. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `context` | string | Where or how this term is typically used. |
| `aliases` | string list | Alternative names or abbreviations for this term. |
| `see_also` | reference list → `term` | Related terms for cross-referencing. |
| `module` | reference → `module` | The module (bounded context) that owns this term's definition. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| outgoing | [module](module.md) | `TermBelongsToModule` | `module` on `term` | Term is defined within a module's bounded context |
| outgoing | [term](term.md) | `TermReferencesRelatedTerm` | `see_also` on `term` | Related terms for cross-referencing |
| incoming | [term](term.md) | `TermReferencesRelatedTerm` | `see_also` on `term` | Related terms for cross-referencing |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I010 | info | `@specforge/product` | Unreferenced term |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |


## Design guidance

### When to add a term

Add one when a word has a project-specific meaning, when two people might read it differently, when it appears
across many entities, or when it is jargon a newcomer will not know. Skip standard concepts and words whose
meaning is obvious from context.

### Writing good definitions

Definitions should be **self-contained** (understandable without other definitions), **precise** (two readers
reach the same understanding), **short**, and **grounded** in an example when the idea is abstract.

### Aliases

Aliases capture the other ways people say the same thing. Keep them unique across terms (case-insensitive) and
do not reuse another term's id as an alias: the compiler does not check this.

### Context

Use `context` to disambiguate:

```spec
term session_def "Session" {
  definition "A materialized, mutable view of the in-memory graph."
  context    "Here 'session' does NOT mean an HTTP session or a user session."
}
```

## Examples

### A regulated-industry glossary

```spec
term batch_release "Batch Release" {
  definition """
    The decision, signed by a Qualified Person, that a manufactured batch
    meets its specification and may be sold.
  """
  aliases  ["QP release", "batch certification"]
  see_also [deviation]
}

term deviation "Deviation" {
  definition "A departure from an approved procedure or specification, recorded and investigated."
  aliases    ["non-conformance"]
}
```

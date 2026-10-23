# An entity's agent context is everything it holds

**Status:** accepted (2026-10-23, architecture round 26, plan 07). Reopens [ADR 0007](0007-emitter-is-export-formats.md)
(the context export lifts headline fields; it no longer drops the rest) and [ADR 0009](0009-host-passes-read-declarations.md)
D (the context schema read `normative`). Amends [ADR 0222](0222-what-an-entity-holds-is-written-once.md) D4 (the
context export keeps every annotation). Follows [ADR 0220](0220-a-kinds-list-written-as-text-is-read-by-one-rule.md)
D5 (the context resources are the context export).

## Context

Two adapters answered "what does an agent need about this entity" by two rules. The context export (and so both
context resources, `query --format context` and the MCP tools with `format: context`) kept the fields an extension
declares headline or normative; the context prompt carried every member (`Members::of`, ADR 0222). On
`examples/todo-app` they disagreed on 29 of 30 entities: a `type` had no members, a union no variants, an event no
`channel`, and on this repository every behavior lost its `requires`/`ensures` conditions. Five commits in four weeks
fixed "what an agent reads" one reader at a time.

## Decision

- **D1. An entity's agent context is its title, its obligations and its members** (`specforge_graph::Members`, ADR
  0222). The context export, both context resources, `query --format context` and the context prompt carry the same
  members; each lays them out its own way: the export lifts each headline field to the node's top level as its field
  text and does not repeat it, the prompt adds `contract_text`.
- **D2. The context export differs from the graph export by what it leaves out:** source positions, and the schema
  unless asked.
- **D3. `normative` means the statement a reviewer quotes** (the review document, ADR 0152) **and, with `headline`,
  the entity's statement.** No export keeps or drops a field by it.
- **D4. No version moves.** `ContextNode`'s shape is unchanged (`fields` was an open map); the published context
  schema is the same.

## Rejected

- Headline + normative + the members a kind does not declare: still two rules, and every extension must mark each
  field an agent needs as normative or the export hides it.
- Every field that is not a reference: the prompt's `downstream_entities` does not say which field names a peer, and
  at depth 0 the export would name no peer.
- A new field flag (`agent`, `prose`) for extensions to opt fields in or out.
- An `AgentContext` wrapper type both readers share: each reader lays `Members` out under its own keys (ADR 0222), and
  the headline is already one function.
- Narrowing the prompt.

## Consequences

- A context export grows by the members it dropped: +17% (`examples/shop`) to +40% (`examples/todo-app`) tokens, +35%
  on this repository's spec. A `--max-tokens` budget that fit may truncate more entities.
- Nothing on disk is read differently. The MCP prompt, `inspect`, the hover and the graph export do not change.
- `normative_fields` is deleted; a headline field named like a node key is written under `fields`.

## What would reopen it

A project whose scoped context export routinely exceeds an agent's budget where the dropped kinds of field are
measured to be noise, or a consumer that needs a field-level opt-out.

# An unknown type name is the host's E004, and a project has one spec block

**Status:** accepted (2026-10-26, architecture round 29, plan 06). Narrows the `link_derived_references` contract of
`spec/behaviors/graph.spec` ("reporting undeclared names stays with the extension that owns the kind"), beside
[ADR 0127](0127-the-linker-records-where-each-edge-is-written.md) D3 and [ADR 0163](0163-a-written-token-is-found-by-one-finder.md),
which still hold. Applies [ADR 0249](0249-the-editor-client-is-an-adapter-over-the-language-server.md) D7.

A type member, a union variant or any other name a derived field reads as a type that named no type was linked to
nothing, without a word. The same name in a port method was E004, because only `@specforge/software`'s guest checked
method signatures. The guest cannot check members: every field reaches it as one string, so it cannot tell a quoted
literal from a type name, nor its own fields from a field another extension adds to `type`. The host holds both facts:
the written form of a value and its reading (`FieldForms`). Two primitive lists existed, the host's and the guest's
mirror.

A project could also hold any number of `spec` blocks: two with different names compiled clean, both were placed, and
`stats` counted both, while the glossary and every page say "exactly one per project".

And the VS Code manifest offered `specforge.graph.defaultLayout`: the client read it and posted it to the graph
panel, whose script never read it and has one layout.

## D1. An unknown type name is the host's E004, at the name

A name a derived field reads as a type (`derive::type_expressions` or `method_signatures`) that is not the block's own
id, not a host primitive (`specforge_graph::PRIMITIVE_TYPES`, the one list) and names no entity of the field's target
kind is E004, owned by `core`, at the name's span. No extension reports it. The port sentence is kept byte for byte;
any other field reads `{kind} '{id}' field '{key}' references unknown type '{name}'`. A union body's variants
(`type Status = open | closed`) and a list's items (`values [cli, api]`) may be an enum's words, so they still link
when they name a type but are never reported: the repository's own spec writes 368 such words.

## D2. A project has one spec block

A second `spec` block, whatever its name, is E002 naming the first, and is not placed, like any duplicate. The first
in the order the build walks files keeps the root. Zero blocks stay allowed.

## D3. A setting is provided when something acts on it

Under ADR 0249 D7 a setting is provided when the panel or the client acts on it, not when a source reads it. The
manifest test checks that the graph panel reads every field the client posts it.

## Rejected

- A software guest rule for `type` members: wrong on quoted literals and on other extensions' fields, or a protocol
  change that sends the names written as types.
- A new warning for a second spec block: two roots stay in the graph and every reader must choose.
- Implementing four layout algorithms in the webview: a feature nobody asked for.

## Consequences

- E004 moves from `software` to `core`, retitled "Unknown type name"; `@specforge/software` declares one rule fewer
  and its mirror list goes. `float` and `object` become host primitives.
- A project with two `spec` blocks, or a member naming no type, fails `check`.
- `specforge.graph.defaultLayout` is gone.

## What would reopen it

- A domain whose type syntax the host's lexer cannot read (D1).
- A project format that holds several roots, such as a federation (D2).

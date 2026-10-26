# An obligation is one record on every surface

**Status:** accepted (2026-10-26, architecture round 28, plan 10). Extends [ADR 0094](0094-obligations-are-an-entitys-own-member.md) U2.
Amends [ADR 0015](0015-read-views-are-operations-over-the-project-view.md) I1,
[ADR 0222](0222-what-an-entity-holds-is-written-once.md) D1 and
[ADR 0064](0064-a-declared-field-type-decides-one-wire-form.md) D5 (the define's name).

## Context

- One `verify` statement reached a reader in four shapes on one server: MCP inspect and the context prompt wrote the string
  `"<kind> <text>"` (a bare statement got a leading space), the exports wrote `{kind, description}` with `kind: ""`,
  `specforge.edit` read `{kind?, text}`, and the hover wrote `- kind: text` from the raw node, unescaped, so
  `verify unit "x"` and `verify "unit: x"` read alike and a two-line text added a bullet.
- An agent could not hand an obligation it read back to the tool that removes it: the string made `remove_verify` a silent no-op.
- The registry kept a sixth copy, `ObligationRecord`, field for field the graph's type once the field was named `text`.
- The graph's `Obligation` already claimed to be "as every output writes it"; inspect did not read it.

```text
BEFORE  inspect/prompt "kind text" | export {kind:"",description} | edit {kind?,text} | hover "- kind: text" | registry copy
AFTER   graph Obligation{kind?, text} -- the one record: snapshot holds it, replies write it, edit reads it
        review document and hover: markdown::obligations_table
```

## Decision

- **D1. The record** is `specforge_graph::Obligation { kind, text }`, `kind` skipped when empty (a bare `verify "…"`),
  define `Obligation` (renamed from `VerifyStatement`), spec type `Obligation` in `spec/types/graph.spec`.
- **D2. The snapshot holds it.** `EntityRecord.obligations` is `Vec<Obligation>`; `ObligationRecord` is deleted.
- **D3. The replies write it.** MCP inspect `verify_declarations`, the context prompt `verify_expectations`, and the graph and
  context exports' `verify` are `Obligation[]`: the JSON `specforge.edit` reads. The keys stay.
- **D4. One markdown rendering.** `specforge_emitter::markdown::obligations_table`, re-exported by ops, lists obligations in
  the review document and the hover (kind `—` when bare, cells escaped to one line).
- **D5. `unproven` stays texts** (the proof rule matches texts); the wire's parallel arrays `verify_kinds` / `verify_texts` stay.
- **D6. The parse's `VerifyStatement { kind, description }` stays**; `Obligation::of` is the one place the names meet.

## Rejected

- Strings in either form (`"unit tests pass"` or `"unit: tests pass"`): still lossy, no round trip.
- `description` everywhere: `text` is the word of the edit input, the registry, the wire, the coverage rule and `CONTEXT.md`.
- An ops-level type, and merging the edit's input type into the record (it carries the ADR 0237 readings).
- Renaming the keys `verify_declarations` / `verify_expectations` / `verify` (ADR 0094 already declined).

## Consequences

- Breaking (pre-release, no shim): the Graph Protocol export's `verify` items are `{kind?, text}` (a major schema bump, W053 on
  an older schema record); MCP inspect and the context prompt return records, not strings; the hover's Obligations section is a
  table; `specforge_registry::ObligationRecord` and `specforge_ops::read::inspect::obligation_text` are deleted.
- The review document is byte-identical.

## What would reopen it

- A client that needs the kind and text as one string.
- `unproven` needing the kind (it changes `crates/leaf/coverage`, a blob input, and a re-vendor).

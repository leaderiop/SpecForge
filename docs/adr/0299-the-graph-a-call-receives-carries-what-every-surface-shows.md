# The graph a call receives carries the computed values and judgements every surface shows

**Status:** accepted (2026-10-29, kernel round K18, plan 08b). Extends [ADR 0287](0287-who-writes-a-field-is-declared.md) (D1) and
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (D8), [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md)
(a writing collector's graph), [ADR 0291](0291-an-extension-contributes-mcp-prompts.md) and
[ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (`reads_graph`). No protocol change.

## Context

An extension's command read no computed value: the `graph` it received held a verification with no `status`, a phase with no
`plans_complete`, a context with no `digest`. The host built that graph with `EmitOptions::default()` for `computed` and
`judgements` (`graph_export`), while `specforge export`, `query` and `inspect` pass both. ADR 0287 D1 ("shown by every
surface"), ADR 0290 D8 ("a command's graph lists each entity's judgements") and protocol 1.9.0 (`GraphNode.judgements`)
already decide that the call graph carries them: the host was short of its own protocol, and the commit that made "every
surface" show computed values tested export, context and inspect only. The same function renders the graph of a `reads_graph`
prompt, a `reads_graph` hook and a writing collector, so all four were short. It stopped the `@gsd/core` commands that stamp a
context's digest and report progress (r41 probe P7).

## Decision

- **D1. The graph a call receives is the graph export of the checked project the call runs over:** authored values,
  computed values and judgements. `graph_export` passes `computed: Some(view.computed())` and the project's listed
  judgements, as `specforge export` does. A command, a prompt that declares `reads_graph`, a hook that declares
  `reads_graph` and a writing collector get them. Out: `coverage_status` (a command gets `evidence`), and the `verdicts` and
  `computed` name list, which `GraphNode` does not carry (a wire change); a guest knows its computed fields from its own
  declaration and a verdict's outcome and date are in its judgement. A computed **reference** arrives as text in `fields`,
  never as an edge: `edges` are the graph's resolved references.
- **D2. The hand-edit guard compares authored values.** What a person wrote is compared, never a value a pass derived: the
  authored export stays `authored_export` and `graph_export` is that plus the two options; the guard calls `authored_export`.
  A future call that needs the authored graph does the same.
- **D3. Reading a computed value gives no way to write it.** A plan op on a computed field or member is refused
  `computed_field` whatever the value (a plan echoing a value it read is refused too); the SDK's `PlanBuilder` never copies
  fields from the graph; a write is judged as ADR 0297 left it. What a command reads can be stale when its plan is applied
  (another writer between compile and lock): the under-lock re-judge recompiles, and a value stamped from the read that no
  longer matches is `gsd/W406` after the write, an Error under `gsd-strict` and then refused as a new error. The stamp is
  never silently wrong.
- **D4. A view whose checks were skipped has no computed values.** The language server's typing fast path skips the checks;
  its `computed` is empty, as its export already is. No surface runs a command over such a view. A command that needs a
  computed value refuses when it is absent ("run `specforge check`") and never guesses.
- **D5. No compute is run for a call.** The call runs over a checked compile, which already ran the passes; emitting the
  values is a map lookup per node and a clone of the judgement map. The cost is the JSON bytes, measured in the round.
- **D6. No protocol change.** No type of `protocol-types`, no SDK file and no descriptor key changes. A 1.11 guest sees more
  entries in `fields` and a non-empty `judgements`, both of which 1.8.0 and 1.9.0 already specify.

## Rejected

- A host function a guest calls for one value: a WIT import is a protocol bump and an SDK change for a cost that is not there.
- A `reads_computed` opt-in on a call's descriptor: a wire field and a re-vendor, for values the host holds anyway.
- Recomputing per call: the checked compile already did.
- Sending `verdicts` and the `computed` name list: a wire change.
- Changing the hand-edit guard's comparison: it keeps comparing authored values (D2).

## Consequences

- A 1.11 guest sees more `fields` and non-empty `judgements`. No builtin, fixture or example declares a computed field or a
  judged rule, so no builtin output moves.
- A guest that copied `fields` into a `create` is now refused `computed_field` instead of writing authored values only:
  fail-closed.
- A command reading a derived value can read a stale one (D3); a derived value is as right as `derive` is.

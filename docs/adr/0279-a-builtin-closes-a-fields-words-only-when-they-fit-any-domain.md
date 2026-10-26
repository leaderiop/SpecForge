# A builtin closes a field's words only when they fit any domain

**Status:** accepted (2026-10-26, architecture round 29, plan 03). Amends
[ADR 0071](0071-a-closed-field-vocabulary-is-declared-once.md) D6 and D7, and
[ADR 0011](0011-extension-command-contract-and-the-product-surface.md) ("Only closed enums are validated").

`@specforge/product` closed a deliverable's `artifact_type` to nine software artifact kinds and a channel's
`interaction_model` to six message-exchange patterns, so a product launch could not say it delivers a press kit or
reaches buyers in person (E061, `check` exits 1), and the deliverables and channels lists refused the word before
reading the graph (exit 2). I062 flagged every module family outside five software-architecture tiers. ADR 0071 closed
`interaction_model` to make its list filter and `check` agree, not because its words fit every domain; nothing in the
engine reads any of the three values beyond a list filter and a table column.

## D1. The rule

A builtin closes a field's words only when every word fits any domain the kind serves: a scale, a lifecycle or a rating
(priority, effort, technical level, the statuses, the FMEA ratings, a property's class, an invariant's risk). A field
that classifies a domain's things is open. A domain that wants a closed set of its own words declares its own kind in
its own extension.

## D2. Three product fields are open identifiers

`deliverable.artifact_type`, `channel.interaction_model` and `module.family` are `identifier` fields: any one bare word
is accepted, and the description names suggested words (today's software words first, then `physical_product`, `media`,
`event`, `training`; `in_person`, `self_service`, `broadcast`; `workstream`). `artifact_type` stays required. The words
live once, in `vocabulary.rs` (`ARTIFACT_TYPES`, `INTERACTION_MODELS`, `MODULE_FAMILIES`); nothing checks a value
against them.

## D3. An open field's list filter matches the word as written

`--artifact-type`, `--interaction-model` and `--family` are `FilterArg::Open`: no `one_of`, no refusal.

## D4. I062 is retired

A suggestion that fires on every word another domain uses is noise, and its own text said custom families are allowed.

## D5. The other vocabularies stay closed

Of the builtins' 16 closed vocabularies, 13 are neutral and stay (governance keeps `invariant.risk` closed with its
other ratings, ADR 0280). `@specforge/testing`'s verify kinds (W009) are software testing by design.

## Rejected

- Declared suggestions in the protocol (`suggested_values`, completed by the editor): a protocol change across six
  readers for a need nobody raised.
- Keeping the lists closed and adding neutral words: the next domain's word is refused again, and `other` erases what
  the field holds.
- A project extending a builtin vocabulary in `specforge.json`: a config, registry and cache mechanism for what a
  project's own extension already does.

## Consequences

- `check` accepts any one word in the three fields; a quoted word is W159, two words or a number E061 (identifier).
- The schema, MCP `specforge.schema` and the published JSON Schema type the two fields `identifier` with no `enum`;
  the editor no longer completes their values; the schema cache takes one major bump.
- Every value written today is still accepted: no `.spec` edit, no migrate step.

## What would reopen it

An editor need to complete open fields (declared suggestions), or a builtin kind whose engine reads a classifying
field's value.

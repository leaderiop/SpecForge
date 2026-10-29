# An agent's write does not change a human-only entity or delete a person's value

**Status:** accepted (2026-10-29, kernel round K17, plan 07b). Extends [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md)
(D7, who writes a field). No protocol change.

## Context

ADR 0289 D7 refuses an agent's write of a human-only value: an extension's plan, MCP, and the command line without a
terminal. The check covers the kind's writer on create, retitle, add or remove of `verify`, and delete, but only the
field's own writer on `set`, `append`, `unset`, `remove`, `link` and `unlink`. So an agent could change a field of an
existing human-only entity unless that field was itself marked human, and delete an entity holding a person's value (a
milestone audit's `debt_accepted`, an approval's `by`) as long as it unlinked nothing a person's entity names.

## Decision

- **D1. An existing human-only entity is not edited.** `set`, `append`, `unset`, `remove`, `link` and `unlink` of an
  entity whose kind is human-only are refused `human_only`, like a retitle. The kind is checked before the field.
- **D2. A person's value is not deleted.** An agent's `delete` of an entity that holds a value whose writer is human is
  refused `human_only` ("'{field}' of {kind} '{id}' was written by a person"): a written field whose own writer is human,
  or a written block or block-list item holding a member whose writer is human. Computed and harness values are not
  checked: an extension deletes its own records. The same check runs over what an agent's `unset`, `remove` or
  replacing `set` takes away, so a person's value is not dropped first; and a plan's delete of an id a rename of the
  same plan took is checked against the entity's id before the plan.
- **D3. Who stays free.** The editor, a person at a terminal and hand edits write and delete as before.

## Rejected

- Marking every field of a human kind human-only (the workaround): correct, but each extension must remember it.
- Allowing the delete because it fails closed for E106: it does not for every rule a person's value satisfies.

## Consequences

A command that archives or removes entities (`/gsd-phase remove`, `/gsd-cleanup`) leaves such an entity or asks the person.
No wire change.

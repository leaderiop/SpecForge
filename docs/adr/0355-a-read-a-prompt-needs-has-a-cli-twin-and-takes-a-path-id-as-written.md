# A read a prompt needs has a CLI twin and takes a path id as written

**Status:** accepted (2026-11-12, R3 of the Beyond Parity build plan). Amends
[ADR 0103](0103-a-core-mcp-resource-is-one-module.md) D2 (a reserved placeholder reads a path),
[ADR 0101](0101-an-operations-request-owns-its-meaning.md) D4 (MCP's trace traces within an entity),
[ADR 0330](0330-trace-follows-the-routes-extensions-declare.md) D3 and D7 (a route over the entities within one),
[ADR 0325](0325-pick-prints-one-value-of-a-json-answer.md) D6 (`search` and `refs` take `--pick`),
[ADR 0122](0122-the-builtins-vocabulary-is-generated-beside-them.md) and
[ADR 0045](0045-publish-is-one-operation-and-one-registry-serves-a-name.md) (registry search is
`specforge extensions search`). Extends [ADR 0284](0284-a-name-is-unique-within-its-scope.md) (a path id is written
as is in the two entity templates). [ADR 0049](0049-a-core-tool-is-one-module.md) D7 and [ADR 0024](0024-one-mcp-request-pipeline.md) D4 stay as written:
naming is the head before the first `{`, and `{+` starts with `{`. No protocol change.

## Context

The prompts an extension ships tell an agent which graph reads to run, and an agent that runs shell commands reads
the graph through the command line as often as through MCP. Four reads an agent needs were missing or broken there:

- **Entity search.** `specforge search` was the registry search: with no registry configured it failed E063, even
  where an entity matched. Entity search existed only as the `specforge.search` tool, whose reply was MCP's own type
  and whose field/value pairing the tool refused itself, not the operation.
- **References.** `specforge.find_references` had no command-line twin, while `specforge anchors` already twins
  `find_implementation` and `find_spec_for_source`.
- **Trace within a scope.** A checker asks for every requirement of one milestone or phase along a route. `trace`
  traced one entity or every entity. A directed reach from a milestone finds no requirement when the milestone holds
  nothing and a phase inside it holds the reference.
- **Path ids in the entity templates.** `specforge://graph/{entity_id}` and `specforge://context/{entity_id}` read
  one path segment, so an id written as is (`v1.0/01/01-01`) was refused as more than one segment; only its
  `%2F` form read, although the view already accepts a path (ADR 0284).

Two smaller gaps travel with them. `specforge <extension> --help` listed a command by its title, so an extension
declaring only descriptions printed a blank line per command. And `specforge.find_implementation` answers nothing
for an entity whose code is reached through other entities, without saying where to look instead.

Coverage was considered and left out: `trace` and an extension's own coverage command answer what an agent asks of
it, and the roles of [ADR 0334](0334-engine-tools-are-one-catalogue-below-the-surfaces.md) grant no `coverage`.

## Decisions

- **D1. `specforge search <text>` is the entity search.** It reads the same operation as `specforge.search`
  (`specforge_ops::read::query::search`) and prints the same document (`EntityHits`): `--kinds`, `--field` with
  `--value`, `--references`, `--limit`, `--format`, `--pick`. The registry search moves to
  `specforge extensions search <query>`, unchanged but for E063's label. There is no alias: the old spelling is the
  new command.
- **D2. `specforge refs <entity>` twins `specforge.find_references`.** Same direction and declaration options, same
  document (`specforge_ops::read::navigate::References`). `refs` is a core command name (`CORE_COMMANDS`), so an
  extension command short-named `refs` is E114.
- **D3. A trace within an entity.** `trace --in <entity>` (MCP `in`) traces every entity *within* one: the entity,
  every entity its scope holds at any depth, and every entity one reference away from one of those, either way. A
  named route keeps the entities of its start kind; the declared route and the reach keep all. They are traced in id
  order, the document of a whole-graph trace. An entity, `in` and a plan are three exclusive targets
  (`entity_with_in`, `plan_with_in`, about `in`). One hop and not a reach: a phase holds the reference to an unscoped
  requirement, so the scope alone finds none, and a reach would leave the scope.
- **D4. A reserved placeholder reads the rest of the path.** A core template's `{+name}` (RFC 6570 reserved
  expansion; the `+` is the operator, the placeholder is still `name`) reads everything after its literal up to the
  query, decoded whole: `/` and `%2F` read alike. Each `/`-separated segment must be non-empty. It is only ever a
  template's last part. The two entity templates are `specforge://graph/{+entity_id}` and
  `specforge://context/{+entity_id}`; an id the graph lacks is "not found", no longer "malformed".
- **D5. The relations and documents of these reads live in ops** (ADR 0101 D1, ADR 0052): the field/value pairing
  (`FieldHolds::of`), the trace request (`TraceRequest`, read by `Ask::of`), the search document (`EntityHits`) and
  the references document (`References`). The CLI and MCP read them, and the request-pairs table has a row for each.
- **D6. Coverage is no gsd role's tool.** [ADR 0334](0334-engine-tools-are-one-catalogue-below-the-surfaces.md)'s roles name their tools one by one and none names `coverage`;
  `trace` and `gsd plan-coverage` answer it.
- **D7. An extension command with no title is listed by its description** (`ExtensionCommand::summary`). A declared
  title still wins; the long help stays the description.
- **D8. `find_implementation` points to `trace`.** Its description says that an entity whose code is reached through
  other entities answers none, and to follow its route with `specforge.trace`.

## Rejected

- **An alias for the registry search.** Pre-release: one spelling per read.
- **Renaming the placeholder to `id`.** Every core tool names the argument `entity_id`, refusals name the
  placeholder, and the spec's verify texts name it.
- **`{/entity_id*}`.** It splits the path into segments and asks the client to send each one.
- **A scope-only `--in`.** It finds no requirement a phase names.
- **A route-declared `within` selector.** Domain-tied, and a protocol change for one use.
- **CLI twins for `inspect` and `coverage`.** No prompt needs them on the command line, and coverage leaves the
  roles.

## Consequences

- `specforge search` changes meaning in one commit, and every remedy that suggested a registry search names
  `specforge extensions search <word>`.
- An MCP client that read `specforge://graph/a/b` as invalid input now reads "not found".
- `trace --in` with no route over a large scope traces many entities; it is bounded by the scope, which is why MCP
  offers it while a trace of every entity stays declined.

## What would reopen it

A core template with a reserved placeholder before a literal; an agent needing coverage by route; a second
within-rule asked for by an extension.

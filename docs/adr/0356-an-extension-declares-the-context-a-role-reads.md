# An extension declares the context a role reads

**Status:** accepted (2026-11-12, R3 of the Beyond Parity build plan). Extends
[ADR 0012](0012-one-extension-declaration.md) (a describe category), [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md)
(the selector grammar and the input digest), [ADR 0330](0330-trace-follows-the-routes-extensions-declare.md) (a route
an extension declares, checked once at load), [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md)
(an agent's body) and [ADR 0334](0334-engine-tools-are-one-catalogue-below-the-surfaces.md) (a host-appended section,
the catalogue row `context`). Amends [ADR 0050](0050-the-guest-wire-carries-only-what-guests-compile.md) (fifteen
categories from 2.1) and [ADR 0024](0024-one-mcp-request-pipeline.md) D4 (`context/{+entity_id}` with `role` is the
role's pack). Reopens [ADR 0190](0190-a-read-selection-decides-what-the-read-covers.md) through its clause: a pack's
budget ranks by declared section order; R4 holds. An item of extension protocol 2.1.0, which ADR 0334 takes.

## Context

An agent got its context from reads its prompts assembled by hand: 116 `--format context` lines in 39 files of one
extension's assets, each a query at some depth over some kinds. The one budgeted read there was, `export
--max-tokens` and the `specforge://context/{entity_id}` resource, cuts the least connected entities first. On a
twin of a real project, a 1,500-token read of a plan kept another phase and dropped the plan's own first task and
its phase's context: the cut is blind to what the reader needs.

The parts of a context compiler existed apart: one selector evaluator shared by judged rules and trace routes, a
per-input sha256 digest that judged verdicts are fresh against, a budget fitter, the context export format and a
context resource template. Nothing joined them, and nothing let an extension say which entities a role reads, in
which order of importance.

## Decisions

- **D1 The declaration.** A describe category `contexts` (protocol 2.1, the fifteenth, read last). A
  `ContextDescriptor` holds a kebab `name` (qualified `<short>/<name>`), a `description`, the `subject` kind, an
  optional `agent`, a required positive `budget` in tokens, ordered `sections` and `settings`. A section has a `name`,
  a `selector` in ADR 0290's grammar with its head (`self…`, a kind, `file(..)`, `test(..)`) and a `tier`, `core`
  (the default) or `enrich`. `settings` names settings of the extension's own. A guest older than 2.1 is never asked
  for the category. The SDK declares one with `ContributionsBuilder::context`.
- **D2 The registry checks each route once.** A route that cannot be served is E123 and is not registered: a name
  that is not kebab case or is taken (in its extension, or its qualified name by an earlier extension); no section, or
  no core section; a section name that is not a word, is repeated or is `settings`; a selector that does not parse,
  whose `file("…")` path leaves the project, or that does not type-check from the subject kind (the judged rules'
  walk, where a computed or verdict-written reference makes no edge); a subject kind no loaded extension declares; a
  setting its extension does not declare (a secret is not a setting); a budget of 0. A subject kind of an unloaded
  optional peer drops the route silently, as a trace route's start does (ADR 0330); a section that reaches one is
  skipped and listed in the pack. E124 is an agent its extension does not declare, or one an earlier route binds: the
  binding is dropped and the route kept, so the agent's own extension always writes its section.
- **D3 One named-selector evaluator.** The judged rules' input reader evaluates named selectors for judged inputs and
  context sections alike. A section's digest is the input digest of the same selector over the same members, byte for
  byte, so a verdict over a section and a pack that served it agree on what they read.
- **D4 The declared budget.** The subject and every core section are the floor; a budget below it is E062 naming the
  route. Enrich sections are served whole in declared order. Inside the first that does not fit, an entity section
  keeps its most connected new entities that fit (a files, tests or settings section keeps nothing); it and every
  later section are listed in `budget.truncated_sections`, the entities of it not served in
  `budget.truncated_entities`. Nothing after it is tried: declared order is the role's priority. This is the strategy
  ADR 0190 named as its reopening; R4 holds (the subject is the pack's root). `export` keeps `prioritize`.
- **D5 The pack and its digest.** A pack answers `route`, `subject`, `agent` (when bound), `digest`, `budget` (strategy,
  budget, estimate and what was cut), `sections` as served (each its members, or its files as paths and digests, or
  its settings as values, with its digest), `skipped`, and `graph`: the export envelope of the subject and every
  served entity in the asked format. The digest is the judged digest function over `{route, subject, entity,
  sections: [[name, selector, digest], …]}`: `entity` is the subject's own digest (the judged digest of the
  one-member entity set, its checked values with it), since the subject is always served even when no section
  selects `self`; each section's digest is over the members it served. The budget is recorded beside it, not in
  it, so an entity no pack served never moves it. A reader recomputes it with `specforge context <route>
  <subject> --max-tokens <budget> --pick .digest`.
- **D6 Surfaces.** `specforge context <route> <subject>`, the MCP tool `specforge.context` and `role` on
  `specforge://context/{+entity_id}` are one operation and answer one text. `role` beside `depth` or `kinds` is
  refused naming the key; without `role` the resource is the context export it was.
- **D7 The agent section.** The agent a route names gets a host-appended `## Context` section after the generated
  tools section (ADR 0334) and before "Judged rules", on every host: the route, its subject kind, its sections in order
  with their tier, its budget, and the CLI line, the MCP tool and the resource that read the pack.
- **D8 What stays.** `query`, `export`, the context export format and the trace routes are unchanged; the estimate stays
  the host's (`estimate_tokens`, no tokenizer); a project whose extensions declare no context route compiles, exports
  and installs byte for byte as before. Recording a pack's digest on an entity and reporting it stale are decided in
  an amendment of this ADR.

## Rejected

- **A guest-only pack command.** A second evaluator and a second digest, against D3 and ADR 0330's one evaluator.
- **A second template `specforge://context/{role}/{id}`.** ADR 0049 D7 names a template by its head: it cannot live
  beside `specforge://context/{+entity_id}`.
- **Ranking by distance from the root.** The role, not the graph, knows what matters.
- **File contents in a pack.** The agent reads the files it cites, and a verdict's `cited` covers them; a pack lists
  paths and digests.
- **A digest over the truncated ids.** It stales a pack on what it never served.
- **`--digest` and `--sections` flags.** `--pick .digest` and `--pick .sections` read them (ADR 0325).

## Consequences

- `context` is a core command name, so an extension short-named `context` has no CLI route (E114).
- A route's budget states what its role costs; a read at a budget below its core sections refuses loudly instead of
  dropping a task.
- A pack carries setting values, so a changed setting moves its digest: the role read them.

## What would reopen it

A role whose priority is not one order of sections; a tokenizer the host can count with; a pack an agent needs with
file contents.

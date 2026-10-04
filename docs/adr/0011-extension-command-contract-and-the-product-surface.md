# One contract for extension commands; the product surface is its commands

**Status:** accepted (2026-10-03)

ADR 0008 made the `specforge product` queries `@specforge/product`'s commands, run over the graph
the host passes. The product spec still describes a larger surface than that contract can carry:
49 commands (17 built), 39 MCP resources, three output formats the host does not know of
(`json` default, `table`, `brief`), cursor pagination needing a graph revision, timelines needing
a clock, weighted completion needing project configuration, and observability events from a
command. The surface specs also promise export, schema and arg-type validation, a per-contribution
toggle and eight events that nothing calls, reads or emits. This ADR fixes the command contract
once for every extension, trims what the design rules out (ADR 0003), and leaves what is still to
build as unproven obligations.

## A. The host owns `--format`; formats are `human` and `json`

- Every extension command has the host's `--format human|json`, like `--path` and `--help`: the
  CLI adds it, defaulting to `human`, and a command declaring an arg named `format` is refused
  (exit 2), as one declaring `path` is. MCP's auto-promoted tool has no `format` argument: the
  host always asks for `json`, and when the command exits 0 with a JSON object on stdout, the
  tool result carries it as `structuredContent` beside the text.
- `CommandInput` gains `format` (`"human"` or `"json"`, serde default `human`). The extension
  renders both, since only it knows its payloads: `json` is one root object, the payload type
  the spec names for the command; `human` is the extension's layout (a table with a header row
  where the payload is tabular).
- `table` and `brief` are not formats. A human table is what `human` is; `brief` already names an
  export resolution (`specforge export --format brief`), and an agent reads `json`, whose
  payloads are already small (counts, ids, pagination). Core commands use the same pair
  (`OutputFormat { Human, Json }`), so the whole CLI has one rule.

Why host-owned: an agent over MCP can never get prose by leaving a flag out, every command takes
the same flag with the same values, and the extension cannot drift from it.

## B. Errors, exit codes and pagination are the command's, written once

- A command that cannot answer writes one error to stderr and nothing to stdout: under `json`
  the object `{code, message, entity_id?, suggestion?}` (`ProductSurfaceError` for product),
  under `human` `error: <message>` and, when there is one, `did you mean '<id>'?`. The
  suggestion is the nearest id of the same kind within Levenshtein distance 2.
  `ENTITY_NOT_FOUND` exits 1. `INVALID_INPUT` (a value outside an enum a string arg carries, a
  negative offset, an unknown sort field) exits 2, the code clap gives the usage errors it
  catches itself on the same command line. Over MCP a non-zero exit is an `isError` result
  carrying the same object.
- `GRAPH_NOT_READY` is gone: a command only ever runs over a built graph.
- One pagination contract, for the lists and for the project-wide matrix queries (coverage
  matrices, overlap, owner workload, module coupling): `--limit` (default 100, clamped to
  [1, 1000]) and `--offset` (default 0); the payload carries `total` (after filters, before
  paging), `offset`, `limit` and `has_more`. Cursors are dropped: with no graph revision a cursor
  detects nothing an offset does not, and one scheme is one schema. Other queries are not paged.
- An entity-scoped command takes the entity id as a positional arg named after its kind
  (`milestone`, `journey`); the MCP tool's argument has the same name. A tags filter is a
  comma-separated string arg (`--tags a,b`): there is no list arg type.

## C. The host passes the date; no configuration reaches a command

`CommandInput` gains `today`: the host's date when the command is called, UTC, `YYYY-MM-DD`.
Commands that compare dates (`milestone_timeline`, `milestone_velocity`) also declare `--as-of`,
which overrides it. A date is domain-free; passing it keeps a command a function of its input
(tests pass `--as-of`), where a guest clock would not be. Compiles are unchanged: `specforge
check` and compiler passes get no date, so diagnostics stay deterministic (I058 stays query-time).

No project configuration reaches a command. The effort weights are the effort scale's definition
(xs=1, s=2, m=3, l=5, xl=8, a missing effort weighs as m), not settings:
`pe_effort_weights_configurable` is superseded. Per-extension configuration would need a config
schema the manifest declares, a `specforge.json` namespace and a channel to both commands and
passes, for one consumer.

## D. Product declares no MCP resources, and no one-hop lookups

- The 39 `specforge://product/...` resources are trimmed. Each repeated a command that MCP
  already serves as the tool `specforge.product.<id>`, over the served graph, in the same JSON.
  A resource export receives only its URI, so serving them needs the graph passed to resources,
  a clock for `_timestamp` and a second response envelope (`ProductSurfaceResponse`) for the same
  data. The host's extension-resource path stays for extensions with content that needs no
  graph.
- The nine one-hop lookups (`feature_milestones`, `persona_journeys`, `channel_journeys`,
  `module_deliverables`, `milestone_deliverables`, `module_features`, `journey_deliverables`,
  `release_deliverables`, `release_milestones`) are trimmed: each follows one reference whose
  kind pair is unique, which core `specforge query <id> --depth 1 --kind <kind>` and MCP
  `specforge.query` / `specforge.find_references` already answer, with the edges. Nine fewer
  tools is less for an agent to choose between, at no loss. `pe_reverse_query_symmetry` is
  superseded; symmetry is core's query. `feature_dependents` and `deliverable_dependents` stay
  (same-kind `depends_on`, where a depth-1 query mixes both directions), as do the multi-hop and
  computed queries.
- Commands emit no events: `pe_cli_command_executed` and `pe_mcp_resource_accessed` are trimmed.
  MCP records each command as `surface_command_dispatched`, with its duration.

That leaves 40 commands: the 17 built and 23 to build. Their obligations stay unproven until
built.

## E. Surface declarations are checked where they can be

- **Export presence cannot be checked before a call.** A component guest has one `call` export
  that routes by name (ADR 0004's bridge world), so the host has no export list to compare a
  declaration against. `validate_surface_exports` is deleted. A declared export the guest does
  not route answers `unknown export`, which the host reports as E028 when it is dispatched. W055
  (surfaces without a binary) cannot arise: an extension without a binary does not load.
- **Arg types are a closed enum in the protocol**, so an unknown one cannot be represented. A
  `surfaces` description that does not parse now fails the extension's load (E028), as every
  other described category does, rather than silently dropping its surfaces.
  `validate_command_arg_types` is deleted, and with it its "no args is W057" clause (W057 is
  product's milestone code).
- **Tool schemas are checked in the registry build.** An explicit MCP tool whose `input_schema`
  or `output_schema` is not a JSON object is E055 and is not registered. A tool's description is
  a required protocol field, so W056 is trimmed. `validate_mcp_tool_schemas` moves into the
  registry build.
- **No toggle.** Nothing in `specforge.json` disables one contribution, so the `enabled` flag on
  surface entries and `toggle_surface_contribution` go. To hide an extension's commands, disable
  the extension.
- **Events:** the registry build is pure, and its diagnostics are its record. The registration,
  validation and toggle events are trimmed. MCP emits `commands_auto_promoted` and
  `surface_command_dispatched` (both built) and will emit `surface_mcp_tool_dispatched` and
  `surface_mcp_resource_dispatched` when an extension tool or resource returns.

## F. Payloads follow the product spec where the code drifted

`milestone_completion` reports `done_count`, `completion_ratio` in [0, 1] and `done_features`
(ids). `journey_coverage` counts a journey's features whose status is `done` and lists the rest
as `uncovered_features`, not features some module holds (module ownership is a validation
concern). `feature_impact` gains the deliverables reached through journeys and modules, the
transitive `depends_on` dependents and `total_affected_entities`. Ratios are in [0, 1]
everywhere; `health` keeps its 0 to 100 score.

`feature_impact` counted features that only list the feature under `features` (a "relates to"
link) as depending on it. Fixed here: only a `depends_on` reference is a dependency.

## Adjustments made while building it

- **A refused command is on neither surface.** The rule that refuses a command line (an arg
  named `path`, `format` or `help`, or two args of one name) is one function,
  `specforge_ops::command::refusal`; MCP does not promote a command it refuses, so a
  `format` arg never reaches an agent either.
- **A command whose export trapped answers in the format asked for.** Under `--format json`
  the CLI writes E028 to stderr as one `{code, message}` object, the shape commands write
  their own errors in, rather than the diagnostic line.
- **Extension-command errors go to stderr; core commands' `--format json` errors go to
  stdout.** Core commands print `{"error", "code", "suggestion"}` on stdout under `json`
  (`print_op_error` in `specforge-cli`), extension commands print `ProductSurfaceError` on
  stderr. The difference is known and kept for now: changing core's would break every script
  reading core errors from stdout, for no consumer that needs the two alike. Reopen it when
  a consumer reads both kinds of command the same way (an agent driving the CLI rather than
  MCP), or at core's next breaking output change; then both write one error object to
  stderr.
- **Only closed enums are validated.** `--family` takes any value: `ModuleFamily` is open
  (a family outside the standard set is I062, an info), so refusing it would refuse a valid
  project. A filter on a reference (`--persona`) matches the id, unvalidated.
- **An absent lifecycle status is its first value**, as each status type says (feature
  `proposed`, deliverable `draft`, milestone and release `planned`, persona and channel
  `active`): `--status proposed` lists a feature without a status. Entries still report the
  status as written.
- **Sorting.** `--sort-by` takes `id`, `title`, `tags` or a field the kind declares (read
  from the extension's own entity declarations); a closed-enum field sorts in its enum's
  order (`priority` critical first, `effort` xs first), any other by its text; an entity
  without the field is last in either order, but one without a lifecycle status sorts as
  that status's default; ties are by id ascending in either order.

- **Traversals follow references of the kinds they name.** Every hop keeps only targets (or
  sources) of the kind its path names, so a peer extension's reference under the same label
  is not followed. `deliverable_personas`'s `via_journey_ids` are the journeys on a path to a
  persona: a deliverable's journey that targets no persona connects none and is not listed.
- **`feature_impact` reports what the feature affects, not what it needs.** Its payload is
  `FeatureImpactPayload` alone: the `referenced_by_*` lists are `affected_*`, `depended_on_by`
  is the transitive `dependent_features`, and the features it depends on (`depends_on`, which
  deferring it does not touch) are no longer listed; the feature's own references answer that.

## What would reopen this

- A command that must write files, stream, or read beyond the graph and the date (ADR 0008's
  trigger, unchanged).
- A project that needs other effort weights, or a second extension wanting settings (then a
  manifest-declared config schema and its block passed to commands and passes).
- An extension resource whose content depends on the graph (then the host passes the graph to
  `mcp__` resources as it does to commands).
- A consumer that pages a changing graph and must detect the change (then a graph revision in
  `CommandInput` and cursors).
- A need to hide single contributions (then `surfaces.disabled` in `specforge.json`).
- A host that can list a guest's routed exports (then presence is checked at load, E020).

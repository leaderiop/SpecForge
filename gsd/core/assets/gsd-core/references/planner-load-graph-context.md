<!-- Ported from gsd-core v1.19.0 gsd-core/references/planner-load-graph-context.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Planner — Load Graph Context

> Loaded by `gsd-planner` at the `load_graph_context` step.

The planner prompt's `**Graph context:**` line says how this install serves `/gsd-graphify`, read from the
slash commands the project serves (`@gsd/codebase` serves the graph; `@gsd/core` does not). That line is the
presence gate: nothing probes PATH, and no setting names a graph file.


If the line is absent or says `not served on this project`, continue without graph context — skip the rest of this step.

If what it runs reports the graph as stale, note for later: "Graph is {age_hours}h old -- treat semantic relationships as approximate." Include this annotation inline with any graph context injected below.


Query the graph for phase-relevant dependency context (single query per D-06): run what the line names (a prompt, or a `specforge` command) with the phase-goal keyword, for the related nodes and, when it offers them, the subsystems the phase may affect.



Its output's shape is the serving extension's: read whichever you get; do not assume a stable shape and do not paste raw output into PLAN.md.

Use the keyword that best captures the phase goal. Prefer the full domain word over a
prefix of it — a reader may match a prefix as a substring, so "auth" also
seeds on `author` and `authoring`. Examples:
- Phase "User Authentication" -> query term "authentication"
- Phase "Payment Integration" -> query term "payment"
- Phase "Database Migration" -> query term "migration"

If the query returns related nodes, incorporate as dependency context for planning:
- Which modules/files are semantically related to this phase's domain
- Which subsystems may be affected by changes in this phase
- Cross-document relationships that inform task ordering and wave structure

If nothing comes back, continue without graph context.

<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/discuss-phase/templates/context.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# The phase context — for discuss-phase write_context step

> **Lazy-loaded.** Read this file only inside the `write_context` step of `workflows/discuss-phase.md`.

A phase's context is entities: one `context`, one `decision` per decision and one `deferred_idea` per deferred idea
(`specforge schema --kind context`, likewise `decision` and `deferred_idea`). Their typed edits are written once, in
the context template: read it with `specforge gsd reference gsd-core/templates/context.md` and run its fence with the
discussion's values.

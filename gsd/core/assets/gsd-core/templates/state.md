<!-- Ported from gsd-core v1.19.0 gsd-core/templates/state.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# State Template

Template for the project's living memory.

SpecForge computes this: there is no entity to write and nothing to update. `specforge gsd progress-report` reads the
current position, the progress and the accumulated context from the graph, and `specforge schema --kind phase`
(likewise `milestone`, `decision`) lists the fields it is computed from.

- **Project reference**: the `project` entity (`specforge export --kinds project --format context`).
- **Current position**: the milestone and phase in progress and the plan being executed, from the phases' and plans'
  computed statuses.
- **Performance metrics**: computed from the plans' summaries.
- **Accumulated context**: the decisions (`decision` entities), the pending todos and the blockers and concerns are
  entities of their own, read where they are recorded.
- **Session continuity**: a paused session is the pause capability's entity (`/gsd-pause-work`, `/gsd-resume-work`).

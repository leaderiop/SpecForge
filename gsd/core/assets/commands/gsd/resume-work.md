<!-- Ported from gsd-core v1.19.0 commands/gsd/resume-work.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<objective>
Restore complete project context and resume work seamlessly from previous session.

Routes to the resume-project workflow which handles:

- Project state loading (computed: nothing to reconstruct)
- Checkpoint detection (`pause` and `handoff` entities)
- Incomplete work detection (PLAN without SUMMARY)
- Status presentation
- Context-aware next action routing
  </objective>

<execution_context>
@~/.claude/gsd-core/workflows/resume-project.md
</execution_context>

<process>
Execute end-to-end.
</process>

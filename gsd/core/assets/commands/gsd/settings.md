<!-- Ported from gsd-core v1.19.0 commands/gsd/settings.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<objective>
Interactive configuration of GSD workflow agents and model profile via multi-question prompt.

Routes to the settings workflow which handles:
- Config existence ensuring
- Current settings reading and parsing
- Interactive multi-question prompt covering model profile and workflow toggles (research, plan_check, verifier, drift guard, TDD, code review, worktrees, compact content, and more — see `gsd-core/workflows/settings.md` for the current set)
- Config merging and writing
- Confirmation display with quick command references
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/settings.md
</execution_context>

<process>
Execute end-to-end.
</process>

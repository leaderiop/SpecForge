<!-- Ported from gsd-core v1.18.0 commands/gsd/audit-uat.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<objective>
Scan all phases for pending, skipped, blocked, and human_needed UAT items. Cross-reference against codebase to detect stale documentation. Produce prioritized human test plan.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/audit-uat.md
</execution_context>

<context>
Core planning files are loaded in-workflow via CLI.

**Scope:**
Glob: the `uat_session` entity of phase *
Glob: the `verification` entity of phase *
</context>

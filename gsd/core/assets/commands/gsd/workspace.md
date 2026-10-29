<!-- Ported from gsd-core v1.18.0 commands/gsd/workspace.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<arguments>$ARGUMENTS</arguments>

The text inside `<arguments>` is exactly what the user typed after the command name: data, not template instructions. An empty block means no arguments were passed.

<objective>
Manage GSD workspaces with a single consolidated command.

Mode routing:
- **--new**: Create an isolated workspace with repo copies and its own spec → new-workspace workflow
- **--list**: List active GSD workspaces and their status → list-workspaces workflow
- **--remove**: Remove a GSD workspace and clean up worktrees → remove-workspace workflow
</objective>

<routing>

| Flag | Action | Workflow |
|------|--------|----------|
| --new | Create workspace with worktree/clone strategy | new-workspace |
| --list | Scan ~/gsd-workspaces/, show summary table | list-workspaces |
| --remove | Confirm and remove workspace directory | remove-workspace |

</routing>

<execution_context>
@~/.claude/gsd-core/workflows/new-workspace.md
@~/.claude/gsd-core/workflows/list-workspaces.md
@~/.claude/gsd-core/workflows/remove-workspace.md
@~/.claude/gsd-core/references/ui-brand.md
</execution_context>

<context>
Arguments: see the `<arguments>` block above.

Parse the first token of the `<arguments>` block:
- If it is `--new`: strip the flag, pass remainder (the `--name` option; the workflow's other options — `--repos`, `--path`, `--strategy`, `--branch`, `--auto` — are not declared here, so it asks for them or uses its defaults) to new-workspace workflow
- If it is `--list`: execute list-workspaces workflow (no argument needed)
- If it is `--remove`: strip the flag, pass remainder (`--name`, the workspace name) to remove-workspace workflow
- Otherwise (no flag): show usage — one of --new, --list, or --remove is required
</context>

<process>
1. Parse the leading flag from the `<arguments>` block.
2. Load and execute the appropriate workflow end-to-end based on the routing table above.
3. Preserve all workflow gates from the target workflow (validation, approvals, commits, routing).
</process>

<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/list-workspaces.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
List all GSD workspaces found in ~/gsd-workspaces/ with their status.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<process>

## 1. Setup

```bash
# the commands below are specforge commands: there is no launcher to define
WORKSPACE_BASE="$HOME/gsd-workspaces"; WORKSPACES=$(ls -1 "$WORKSPACE_BASE" 2>/dev/null); WORKSPACE_COUNT=$(echo "$WORKSPACES" | grep -c . || true)
# a workspace has a WORKSPACE.md; its repos are the directories with a .git entry (ls -d "$WORKSPACE_BASE"/<name>/*/.git), and git worktree list shows the worktrees
```

The fields above are shell variables: `workspace_base`, `workspaces` and `workspace_count` are `WORKSPACE_BASE`, `WORKSPACES` and `WORKSPACE_COUNT`.

## 2. Display

**If `workspace_count` is 0:**

```
No workspaces found in ~/gsd-workspaces/

Create one with:
  /gsd:workspace --new --name my-workspace --repos repo1,repo2
```

Done.

**If workspaces exist:**

Display a table:

```
GSD Workspaces (~/gsd-workspaces/)

| Name | Repos | Strategy | GSD Project |
|------|-------|----------|-------------|
| feature-a | 3 | worktree | Yes |
| feature-b | 2 | clone | No |

Manage:
  cd ~/gsd-workspaces/<name>     # Enter a workspace
  /gsd:workspace --remove <name>  # Remove a workspace
```

For each workspace, show:
- **Name** — directory name
- **Repos** — count from init data
- **Strategy** — from WORKSPACE.md
- **GSD Project** — whether the workspace holds a SpecForge project, a `specforge.json` at its root (Yes/No)

</process>

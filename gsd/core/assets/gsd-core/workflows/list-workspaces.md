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
WORKSPACE_BASE="$HOME/gsd-workspaces"; WORKSPACES=$(specforge files list "$WORKSPACE_BASE" --depth 1 --kind dir --pick '.entries[].path' 2>/dev/null); WORKSPACE_COUNT=$(specforge files list "$WORKSPACE_BASE" --depth 1 --kind dir --pick .count 2>/dev/null)
# a workspace is a directory of repos, and no manifest: its repos are the directories with a .git entry (for g in "$WORKSPACE_BASE"/<name>/*/.git; do [ -e "$g" ] && ...), and specforge git --in "<repo>" worktrees shows the worktrees
```

The fields above are shell variables: `workspace_base`, `workspaces` and `workspace_count` are `WORKSPACE_BASE`, `WORKSPACES` and `WORKSPACE_COUNT`. An empty `workspace_count` means this machine has not approved the root, or the directory is gone: say so, with the command that names and makes it: `specforge grants approve --root workspaces --at ~/gsd-workspaces`.

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
- **Strategy** — `worktree` when its repos are linked worktrees (`specforge git repos "<workspace>" --pick '.repos[linked=true].name'`), else `clone`
- **GSD Project** — whether the workspace holds a SpecForge project, a `specforge.json` at its root (Yes/No)

</process>

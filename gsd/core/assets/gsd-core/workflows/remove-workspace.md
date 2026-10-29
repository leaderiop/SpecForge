<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/remove-workspace.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Remove a GSD workspace, cleaning up git worktrees and deleting the workspace directory.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<process>

## 1. Setup

Set `WORKSPACE_NAME` to the `--name` value in $ARGUMENTS.

```bash
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
WORKSPACE_PATH="$HOME/gsd-workspaces/$WORKSPACE_NAME"; DIRTY_REPOS=$(for r in "$WORKSPACE_PATH"/*/; do [ -n "$(git -C "$r" status --porcelain 2>/dev/null)" ] && basename "$r"; done)
STRATEGY=$(sed -n 's/^Strategy: *//p' "$WORKSPACE_PATH/WORKSPACE.md" 2>/dev/null)   # from the manifest
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

The fields above are shell variables: `workspace_name`, `workspace_path`, `strategy` and `dirty_repos` are `WORKSPACE_NAME`, `WORKSPACE_PATH`, `STRATEGY` and `DIRTY_REPOS`; `has_manifest` is whether `WORKSPACE.md` exists, `repos` are the directories of the workspace with a `.git` entry (`repo_count` their number), and `has_dirty_repos` is whether `DIRTY_REPOS` is not empty.

**If no workspace name provided:**

First run `/gsd:workspace --list` to show available workspaces, then ask:


**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
Use AskUserQuestion:
- header: "Remove Workspace"
- question: "Which workspace do you want to remove?"
- requireAnswer: true

Run the shell lines above again with the provided name.

## 2. Safety Checks

**If `has_dirty_repos` is true:**

```
Cannot remove workspace "$WORKSPACE_NAME" — the following repos have uncommitted changes:

  - repo1
  - repo2

Commit or stash changes in these repos before removing the workspace:
  cd "$WORKSPACE_PATH/repo1"
  git stash   # or git commit
```

Exit. Do NOT proceed.

## 3. Confirm Removal

Use AskUserQuestion:
- header: "Confirm Removal"
- question: "Remove workspace '$WORKSPACE_NAME' at $WORKSPACE_PATH? This will delete all files in the workspace directory. Type the workspace name to confirm:"
- requireAnswer: true

**If answer does not match `$WORKSPACE_NAME`:** Exit with "Removal cancelled."

## 4. Clean Up Worktrees

**If strategy is `worktree`:**

Initialize the failure flag once before iterating repos:

```bash
REMOVE_FAILED=false
```

For each repo in the workspace:

```bash
cd "$SOURCE_REPO_PATH"
if ! git worktree remove "$WORKSPACE_PATH/$REPO_NAME" 2>&1; then
  echo "Warning: Could not remove worktree for $REPO_NAME — source repo may have been moved, deleted, locked, or dirty." >&2
  REMOVE_FAILED=true
fi
```

If any `git worktree remove` fails, stop before deleting the workspace directory:
```text
Refusing to delete "$WORKSPACE_PATH" because one or more git worktrees could not be removed.
Resolve the failed worktree removal manually, then rerun remove-workspace.
```

## 5. Delete Workspace Directory

```bash
if [ "${REMOVE_FAILED:-false}" = "true" ]; then
  echo "Refusing to delete \"$WORKSPACE_PATH\" because one or more git worktrees could not be removed." >&2
  exit 1
fi

rm -rf "$WORKSPACE_PATH"
```

## 6. Report

```
Workspace "$WORKSPACE_NAME" removed.

  Path: $WORKSPACE_PATH (deleted)
  Repos: $REPO_COUNT worktrees cleaned up
```

</process>

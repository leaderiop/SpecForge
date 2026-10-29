<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/onboard.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# /gsd:onboard Workflow

One-command onboarding for an existing or unknown repo. This workflow is a thin
renderer around a projection computed in Step 1 from the spec and the repository; the routing is its `next_action`.

@~/.claude/gsd-core/references/gsd-run-resolver.md

## 1. Render the Onboarding Projection

Parse `$ARGUMENTS`:
- `--fast` passes `--fast` to `init onboard`. Fast mode accepts the fast map for lightweight onboarding only; `next_action` still decides whether complete map work is required before project setup.
- `--text` forces text-mode choices for runtimes without `AskUserQuestion`.

There is no `init onboard` projection here: compute it from the spec and the repository, from the git worktree root:

```bash
# One projection, computed here (--fast changes nothing: there is no fast map to accept):
ENTITIES=$(specforge query --kinds project,milestone,phase,requirement --format json)
# Existing ADR/PRD/SPEC/RFC documents:
DOC_CANDIDATES=$(find . -type f \( -path '*/adr/*' -o -path '*/adrs/*' -o -path '*/prd/*' -o -path '*/prds/*' -o -path '*/rfc/*' -o -path '*/rfcs/*' -o -name 'ADR-*.md' -o -name 'PRD-*.md' -o -name 'SPEC-*.md' -o -name 'RFC-*.md' \) -not -path './.git/*' 2>/dev/null | wc -l | tr -d ' ')
ONBOARDING_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
```

Work out these fields from `ENTITIES`, `DOC_CANDIDATES` and git (no projection supplies them):

- `next_action.kind`, `next_action.command`, `next_action.reason`, `next_action.missing`: the first of these that holds — `map-codebase` (a brownfield repo whose project planning is not complete; command `/gsd:map-codebase`, or `/gsd:map-codebase --fast` with `--fast`), `ingest-docs` (doc candidates and no project; `/gsd:ingest-docs`), `new-project` (no project; `/gsd:new-project`), `partial-planning` (some of the project, requirements and roadmap exist but not all; `missing` names the rest), else `write-summary`
- `handoff_commands.ingest_docs`, `handoff_commands.manager`, `handoff_commands.new_project`, `handoff_commands.onboard`: `/gsd:ingest-docs`, `/gsd:manager`, `/gsd:new-project`, `/gsd:onboard`
- `map_readiness`: `unavailable` (`@gsd/codebase`, which serves `/gsd:map-codebase`, is not installed here); `codebase_map_summary_status` and `codebase_map_final_status`: `not installed`
- `project_exists`, `requirements_exists`, `roadmap_exists`: a `project` entity, `requirement` entities, `milestone` and `phase` entities in `ENTITIES`; `planning_exists`: any of them; `state_exists`: the same as `project_exists` (the project state is computed, never a file)
- `is_brownfield`: the repository has files other than `*.spec` (`git ls-files`); `fast_mode`: `--fast` was parsed; `has_codebase_map`, `has_fast_codebase_map`: false
- `missing_codebase_map_files`, `missing_fast_codebase_map_files`: not applicable (no map is expected here)
- `has_docs_candidates`: `DOC_CANDIDATES` is above 0; `doc_candidate_count`: `DOC_CANDIDATES`; `onboarding_summary_exists`: false (the summary is shown, never written)
- `commit_docs`: false; `text_mode`: the `--text` flag; `has_git`, `git_worktree_root`, `in_nested_subdir`: from `git rev-parse` (nested when `--show-toplevel` is above the current directory)
- `response_language`

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Set:
- `TEXT_MODE=true` if `--text` is present or `text_mode` is true. When `TEXT_MODE` is active, replace every `AskUserQuestion` call below with a plain-text numbered list and ask the user to type their choice number — required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
- `ONBOARDING_ROOT={git_worktree_root || the current directory}`.

If `has_git` and `in_nested_subdir` are true, warn that onboarding artifacts belong to the outer worktree at `git_worktree_root`. Do not run `git init`.

## 2. Execute `next_action`

### `map-codebase`

If `next_action.kind == "map-codebase"`:

- Do not ask: `/gsd:map-codebase` is served by `@gsd/codebase`, which is not installed here. Print one line, "Codebase mapping is not available here (`@gsd/codebase` is not installed): continuing without a map.", then take the "Skip mapping" path:




























On the "Skip mapping" path:

- If `(project_exists || requirements_exists || roadmap_exists || state_exists) && (!project_exists || !requirements_exists || !roadmap_exists || !state_exists)`, route the skip to the partial planning guard instead:

```text
Skipping codebase mapping may give downstream steps weaker context, but project planning exists and is incomplete.

Project: {project_exists ? "present" : "missing"}
Requirements: {requirements_exists ? "present" : "missing"}
Roadmap: {roadmap_exists ? "present" : "missing"}
Project state: {state_exists ? "present" : "missing"}

Run the appropriate lower-level command to fill the missing planning artifact(s), then rerun {handoff_commands.onboard}.
```

Exit.

- If `has_docs_candidates && !project_exists`, route the skip to docs ingest instead:

```text
Skipping codebase mapping may give downstream steps weaker context, but existing ADR/PRD/SPEC/RFC documents should still be ingested before {handoff_commands.new_project}.

Run from worktree root {ONBOARDING_ROOT}:

{handoff_commands.ingest_docs}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit.

- Otherwise print:

```text
Skipping codebase mapping may give {handoff_commands.new_project} weaker context.

Run from worktree root {ONBOARDING_ROOT}:

{handoff_commands.new_project}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit.

### `ingest-docs`

If `next_action.kind == "ingest-docs"`:

- If `TEXT_MODE=true`, print:

```text
{next_action.reason}
Detected {doc_candidate_count} possible ADR/PRD/SPEC/RFC document(s).

1. Ingest docs first — run {next_action.command} from worktree root {ONBOARDING_ROOT} (Recommended)
2. Skip docs ingest — continue to {handoff_commands.new_project}

Enter number:
```

- Otherwise use AskUserQuestion:
  - header: "Docs"
  - question: "Detected {doc_candidate_count} possible ADR/PRD/SPEC/RFC document(s). Ingest them first?"
  - options:
    - "Ingest docs first" — Run `{next_action.command}` from worktree root `{ONBOARDING_ROOT}` (Recommended)
    - "Skip docs ingest" — Continue to `{handoff_commands.new_project}`

If the user chooses ingest, print:

```text
Run from worktree root {ONBOARDING_ROOT}:

{next_action.command}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit. If the user skips docs ingest, print:

```text
Skipping docs ingest may omit existing ADR/PRD/SPEC/RFC context from {handoff_commands.new_project}.

Run from worktree root {ONBOARDING_ROOT}:

{handoff_commands.new_project}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit.

### `complete-map-before-new-project`

If `next_action.kind == "complete-map-before-new-project"`, print:

```text
{next_action.reason}

Run from worktree root {ONBOARDING_ROOT}:

{next_action.command}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit.

### `new-project`

If `next_action.kind == "new-project"`, print:

```text
{next_action.reason}

Run from worktree root {ONBOARDING_ROOT}:

{next_action.command}

Then rerun {handoff_commands.onboard} from the same worktree root.
```

Exit.

### `partial-planning`

If `next_action.kind == "partial-planning"`, print:

```text
Project planning exists but is incomplete.

Missing files: {next_action.missing}
Requirements: {requirements_exists ? "present" : "missing"}
Roadmap: {roadmap_exists ? "present" : "missing"}
Project state: {state_exists ? "present" : "missing"}

Run the appropriate lower-level command to fill the missing planning artifact(s), then rerun {handoff_commands.onboard}.
```

Exit.

### `ready`

If `next_action.kind == "ready"`, print the final status section and exit.

### `write-summary`

If `next_action.kind == "write-summary"`, continue to summary creation.

## 3. Create Onboarding Summary

Show the onboarding summary below to the user. It is not written to a file: the summary is the answer.

Summary template:

```markdown
# Onboarding Summary

## Project State
- Project: {project_exists ? "present" : "missing"}
- Requirements: {requirements_exists ? "present" : "missing"}
- Roadmap: {roadmap_exists ? "present" : "missing"}
- Project state: {state_exists ? "present" : "missing"}

## Codebase Context
- Brownfield repo: {is_brownfield ? "yes" : "no"}
- Map readiness: {map_readiness}
- Codebase map: {codebase_map_summary_status}
- Fast map available: {has_fast_codebase_map ? "yes" : "no"}

## Docs Context
- Existing ADR/PRD/SPEC/RFC candidates: {has_docs_candidates ? doc_candidate_count : 0}

## Recommended Next Step
- {handoff_commands.manager}
```

There is nothing to commit: the summary is not a file.

```bash
: # no commit: the summary is shown, not written
```

Continue to final status.

## 4. Final Status

Print:

```text
Onboarding status:
- Project: {project_exists ? "present" : "missing"}
- Requirements: {requirements_exists ? "present" : "missing"}
- Roadmap: {roadmap_exists ? "present" : "missing"}
- Project state: {state_exists ? "present" : "missing"}
- Codebase map: {codebase_map_final_status}
- Onboarding summary: shown above

Next recommended command: {handoff_commands.manager}
```

Do not run implementation execution or shipping from onboarding.

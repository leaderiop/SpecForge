# Contributions

**Status: being designed; nothing is built yet.** What `@gsd/*` contributes beyond kinds and rules: commands, agents, prompts, skills and hooks
(K8, K9). Generated from gsd-core 1.18.0's `commands/gsd/` and `agents/`; the parity suite's roster check
re-runs the comparison on every tracked version.

Every `@gsd/*` extension declares `ext_short: "gsd"`, so all of them route under `gsd`; a clash between them is E039.
A slash command `/gsd-<name>` (`$gsd-<name>` in Codex) is rendered for Claude Code, Codex and Cursor and binds to one
of two things (K9 U4):

- **a workflow**: the MCP prompt `specforge.gsd.<name_with_underscores>` (K8), ported from `gsd-core/workflows/`,
  also reached through the core tool `specforge.prompt` and the CLI `specforge prompt`. The host agent follows it;
  each step's writes are a command's edit plan (K5), and a step whose gate fails is refused.
- **a command**: code in the extension, served as the MCP tool `specforge.gsd.<name>` and the CLI
  `specforge gsd <name>`. These are the command-backed ones: `/gsd-health`, `/gsd-help`, `/gsd-stats`,
  `/gsd-surface`, `/gsd-update`, `/gsd-phase`, `/gsd-workstreams`, `/gsd-workspace`, `/gsd-cleanup`,
  `/gsd-graphify`. Every other row below is a workflow.

The workflows' steps call writing commands that are not slash commands (record a verdict, record a commit, set a
UAT result …); those are listed with their workflow in the parity roster.

## Commands

| Owner | Commands | Count |
|---|---|---|
| @gsd/core | `/gsd-add-tests`, `/gsd-audit-milestone`, `/gsd-audit-uat`, `/gsd-autonomous`, `/gsd-cleanup`, `/gsd-complete-milestone`, `/gsd-config`, `/gsd-discuss-phase`, `/gsd-execute-phase`, `/gsd-extract-learnings`, `/gsd-fast`, `/gsd-health`, `/gsd-help`, `/gsd-ingest-docs`, `/gsd-manager`, `/gsd-milestone-summary`, `/gsd-mvp-phase`, `/gsd-new-milestone`, `/gsd-new-project`, `/gsd-next`, `/gsd-onboard`, `/gsd-pause-work`, `/gsd-phase`, `/gsd-plan-phase`, `/gsd-plan-review-convergence`, `/gsd-pr-branch`, `/gsd-progress`, `/gsd-quick`, `/gsd-quick-batch`, `/gsd-resume-work`, `/gsd-review`, `/gsd-review-backlog`, `/gsd-settings`, `/gsd-ship`, `/gsd-spec-phase`, `/gsd-stats`, `/gsd-surface`, `/gsd-ultraplan-phase`, `/gsd-undo`, `/gsd-update`, `/gsd-validate-phase`, `/gsd-verify-work`, `/gsd-workspace`, `/gsd-workstreams` | 44 |
| @gsd/ai | `/gsd-ai-integration-phase`, `/gsd-eval-review` | 2 |
| @gsd/codebase | `/gsd-docs-update`, `/gsd-graphify`, `/gsd-map-codebase` | 3 |
| @gsd/import | `/gsd-import` | 1 |
| @gsd/ops | `/gsd-audit-fix`, `/gsd-capture`, `/gsd-code-review`, `/gsd-debug`, `/gsd-explore`, `/gsd-forensics`, `/gsd-inbox`, `/gsd-mempalace-capture`, `/gsd-mempalace-recall`, `/gsd-profile-user`, `/gsd-sketch`, `/gsd-spike`, `/gsd-thread` | 13 |
| @gsd/security | `/gsd-secure-phase` | 1 |
| @gsd/ui | `/gsd-ui-phase`, `/gsd-ui-review` | 2 |
| @gsd/core skills (namespace routers: they list and dispatch, nothing else) | `/gsd-ns-context`, `/gsd-ns-ideate`, `/gsd-ns-manage`, `/gsd-ns-project`, `/gsd-ns-review`, `/gsd-ns-workflow` | 6 |
| **total** | | **72** |

## Agents

Each agent is a contribution of its extension (K9): declared metadata (role, host-neutral tool classes, model tier or
a setting naming one) and a body the extension renders, written into the host's agent files by `specforge install`.
It is not a `.spec` entity: prompt text is the extension's source, not project state. Its prompt is ported under MIT
from `agents/gsd-<name>.md`; the `.compact.md` file is the agent's `compact` variant, chosen by a setting. A judged
rule's `judge` names one of these agents (K7), and its rendered file lists the rules it judges.

| Owner | Agents | Count |
|---|---|---|
| @gsd/core | `gsd-advisor-researcher`, `gsd-assumptions-analyzer`, `gsd-doc-classifier`, `gsd-doc-synthesizer`, `gsd-executor`, `gsd-integration-checker`, `gsd-nyquist-auditor`, `gsd-pattern-mapper`, `gsd-phase-researcher`, `gsd-plan-checker`, `gsd-planner`, `gsd-project-researcher`, `gsd-research-synthesizer`, `gsd-roadmapper`, `gsd-verifier` | 15 |
| @gsd/ai | `gsd-ai-researcher`, `gsd-domain-researcher`, `gsd-eval-auditor`, `gsd-eval-planner`, `gsd-framework-selector` | 5 |
| @gsd/codebase | `gsd-codebase-mapper`, `gsd-doc-verifier`, `gsd-doc-writer`, `gsd-intel-updater` | 4 |
| @gsd/ops | `gsd-code-fixer`, `gsd-code-reviewer`, `gsd-debug-session-manager`, `gsd-debugger`, `gsd-mempalace-curator`, `gsd-user-profiler` | 6 |
| @gsd/security | `gsd-security-auditor` | 1 |
| @gsd/ui | `gsd-dom-verifier`, `gsd-ui-auditor`, `gsd-ui-checker`, `gsd-ui-researcher` | 4 |
| **total** | | **35** |

## Hooks

GSD's `hooks/` become typed hook contributions (K9). `specforge install <host>` wires each into the host as
`specforge hook <id>`; a host without that event gets none, and the install report says so.

| GSD hook | Contribution |
|---|---|
| gsd-read-guard, gsd-write-guard | `pre_tool` read / write: the paths a workflow may touch |
| gsd-secret-read-guard | `pre_tool` read, search and shell |
| gsd-read-injection-scanner | `post_tool` read: scans what was read (`security.injection_blocking` turns a warning into a block) |
| gsd-prompt-guard | `pre_tool` write: scans content written to spec files |
| gsd-agent-isolation-guard | `pre_tool` agent: a subagent is started with its own worktree |
| gsd-worktree-path-guard | `pre_tool` write within the executor's worktree |
| gsd-workflow-guard | `pre_tool` shell, write and edit outside a running workflow (`hooks.workflow_guard`) |
| gsd-context-monitor | `post_tool`, `stop`, `subagent_stop`, `pre_compact`: context warnings (`hooks.context_*`), its metrics kept in the extension's session state |
| gsd-statusline | `statusline`, from the graph and the session state |
| gsd-validate-commit | `pre_commit`: the commit subject convention the git collector reads |
| gsd-phase-boundary | `post_tool` write on spec files: the session cursor follows the phase written |
| gsd-session-state | `session_start`: the session cursor |
| (core) `core.spec_guard` | wired by every install: `pre_tool` write on `.spec` refuses a raw write of a computed, harness or human-only value and names the command that writes it |
| gsd-ensure-canonical-path | none: rendered files include nothing from `~/.claude` |
| gsd-check-update, gsd-check-update-worker, gsd-update-banner | none: `specforge` updates extensions through its lockfile |
| gsd-config-reload | none: settings are read on every compile |
| gsd-graphify-update | none: SpecForge is the graph |
| gsd-path-export, gsd-node-runner | none: host plumbing of the shell implementation |
| gsd-cursor-*, gsd-windsurf-* | the Cursor and Windsurf renderings of the events above |

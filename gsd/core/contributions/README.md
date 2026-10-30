# Contributions

**Status: built so far: the collectors `gsd-git` and `gsd-junit` (r38), the 15 hooks and the install goldens (r44), the 15 core agents, the command `record_verdict` and the prompt `spec_phase` (r40), the writing commands and the command-backed slash commands (five in r41, seven by r43), the 16 lifecycle prompts with the command `reference` (r42), and the remaining 27 prompts with the read-only command `smart_entry` (r43): every command `@gsd/core` owns is served.** What `@gsd/*` contributes beyond kinds and rules: commands, agents, prompts, skills and hooks
(K8, K9). Generated from gsd-core 1.18.0's `commands/gsd/` and `agents/`; the parity suite's roster check
re-runs the comparison on every tracked version.

Every `@gsd/*` extension declares `ext_short: "gsd"`, so all of them route under `gsd`; a clash between them is E039.
A slash command `/gsd-<name>` (`$gsd-<name>` in Codex) is rendered for Claude Code, Codex and Cursor and binds to one
of two things (K9 U4):

- **a workflow**: the MCP prompt `specforge.gsd.<name_with_underscores>` (K8), ported from `gsd-core/workflows/`,
  also reached through the core tool `specforge.prompt` and the CLI `specforge prompt`. The host agent follows it;
  each step's writes are a command's edit plan (K5), and a step whose gate fails is refused.
- **a command**: code in the extension, served as the MCP tool `specforge.gsd.<name>` and the CLI
  `specforge gsd <name>`. The command-backed ones built are `@gsd/core`'s `/gsd-help`, `/gsd-surface`, `/gsd-phase`,
  `/gsd-workstreams`, `/gsd-cleanup`, `/gsd-health` and `/gsd-stats` (7); `@gsd/codebase`'s `/gsd-graphify` is not built.
  Every other row below is a workflow. `/gsd-workspace` and `/gsd-update` are workflows too: a command runs nothing
  (the sandbox), and worktrees and updates need git and the package manager, which the agent runs.

A configuration command (`/gsd-settings`, `/gsd-config`, `/gsd-surface`) writes `specforge.json` through
`specforge settings set|unset` (K16, protocol 1.11.0; MCP `specforge.settings_set`), which admits a value by the setting's own type and never writes a secret; `/gsd-surface`
toggles skills, which are `when` settings.

The workflows' steps call writing commands that are not slash commands; they are the [roster below](#writing-commands).
A workflow that names an entity takes the full path when a short form is ambiguous project-wide (the host refuses an
ambiguous one, naming each candidate).

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

## Round and mechanism of each command

Every `/gsd-*` command of v1.18.0 has one owner and one round (`tests/roster.rs` checks the 72 rows against the vendored `commands.json`). The 22 commands of the capability extensions are owned and named; each is served when its extension ships.

| Command | Round | Mechanism |
|---|---|---|
| `/gsd-add-tests` | r42 | prompt `add_tests` |
| `/gsd-ai-integration-phase` | after r44 | @gsd/ai |
| `/gsd-audit-fix` | after r44 | @gsd/ops |
| `/gsd-audit-milestone` | r42 | prompt `audit_milestone` |
| `/gsd-audit-uat` | r42 | prompt `audit_uat` |
| `/gsd-autonomous` | r43 | prompt `autonomous` |
| `/gsd-capture` | after r44 | @gsd/ops |
| `/gsd-cleanup` | r41 | command `cleanup`, command-backed slash |
| `/gsd-code-review` | after r44 | @gsd/ops |
| `/gsd-complete-milestone` | r42 | prompt `complete_milestone` |
| `/gsd-config` | r43 | prompt `config` |
| `/gsd-debug` | after r44 | @gsd/ops |
| `/gsd-discuss-phase` | r42 | prompt `discuss_phase` |
| `/gsd-docs-update` | after r44 | @gsd/codebase |
| `/gsd-eval-review` | after r44 | @gsd/ai |
| `/gsd-execute-phase` | r42 | prompt `execute_phase` (and `execute_plan`, which has no slash command) |
| `/gsd-explore` | after r44 | @gsd/ops |
| `/gsd-extract-learnings` | r43 | prompt `extract_learnings` |
| `/gsd-fast` | r43 | prompt `fast` |
| `/gsd-forensics` | after r44 | @gsd/ops |
| `/gsd-graphify` | after r44 | @gsd/codebase (command-backed there) |
| `/gsd-health` | r41 | command `health` |
| `/gsd-help` | r41 | command `help` |
| `/gsd-import` | after r44 | @gsd/import |
| `/gsd-inbox` | after r44 | @gsd/ops |
| `/gsd-ingest-docs` | r43 | prompt `ingest_docs` |
| `/gsd-manager` | r43 | prompt `manager` |
| `/gsd-map-codebase` | after r44 | @gsd/codebase |
| `/gsd-mempalace-capture` | after r44 | @gsd/ops |
| `/gsd-mempalace-recall` | after r44 | @gsd/ops |
| `/gsd-milestone-summary` | r43 | prompt `milestone_summary` |
| `/gsd-mvp-phase` | r42 | prompt `mvp_phase` |
| `/gsd-new-milestone` | r42 | prompt `new_milestone` |
| `/gsd-new-project` | r42 | prompt `new_project` |
| `/gsd-next` | r43 | prompt `next` over the command `smart_entry` |
| `/gsd-ns-context` | r43 | router prompt `ns_context` |
| `/gsd-ns-ideate` | r43 | router prompt `ns_ideate` |
| `/gsd-ns-manage` | r43 | router prompt `ns_manage` |
| `/gsd-ns-project` | r43 | router prompt `ns_project` |
| `/gsd-ns-review` | r43 | router prompt `ns_review` |
| `/gsd-ns-workflow` | r43 | router prompt `ns_workflow` |
| `/gsd-onboard` | r43 | prompt `onboard` |
| `/gsd-pause-work` | r43 | prompt `pause_work` over the command `pause` |
| `/gsd-phase` | r41 | command `phase` (r43 adds `--promote`) |
| `/gsd-plan-phase` | r42 | prompt `plan_phase` |
| `/gsd-plan-review-convergence` | r42 | prompt `plan_review_convergence` |
| `/gsd-pr-branch` | r43 | prompt `pr_branch` |
| `/gsd-profile-user` | after r44 | @gsd/ops |
| `/gsd-progress` | r43 | prompt `progress` over the command `progress_report` |
| `/gsd-quick-batch` | r43 | prompt `quick_batch` |
| `/gsd-quick` | r43 | prompt `quick` |
| `/gsd-resume-work` | r43 | prompt `resume_work` over the command `resume` |
| `/gsd-review-backlog` | r43 | prompt `review_backlog` over `phase --promote` |
| `/gsd-review` | r43 | prompt `review` |
| `/gsd-secure-phase` | after r44 | @gsd/security |
| `/gsd-settings` | r43 | prompt `settings` |
| `/gsd-ship` | r42 | prompt `ship` |
| `/gsd-sketch` | after r44 | @gsd/ops |
| `/gsd-spec-phase` | r40 | prompt `spec_phase` |
| `/gsd-spike` | after r44 | @gsd/ops |
| `/gsd-stats` | r41 | command `stats` |
| `/gsd-surface` | r41 | command `surface` |
| `/gsd-thread` | after r44 | @gsd/ops |
| `/gsd-ui-phase` | after r44 | @gsd/ui |
| `/gsd-ui-review` | after r44 | @gsd/ui |
| `/gsd-ultraplan-phase` | r42 | prompt `ultraplan_phase` |
| `/gsd-undo` | r43 | prompt `undo` |
| `/gsd-update` | r43 | prompt `update`, a text of ours |
| `/gsd-validate-phase` | r42 | prompt `validate_phase` |
| `/gsd-verify-work` | r42 | prompt `verify_work` |
| `/gsd-workspace` | r43 | prompt `workspace` |
| `/gsd-workstreams` | r41 | command `workstreams` |

## Lifecycle prompts (r42)

Built in `@gsd/core`: the prompt id, the slash command `/gsd-<id with ->`, the gate (the guard rules a refused prompt
names) and the steps (an agent, a command or the next prompt). `reference` serves the workflows, references and
templates a prompt does not inline (`specforge gsd reference gsd-core/references/<file>.md`).

| Prompt | Gate | Steps |
|---|---|---|
| `new_project`, `new_milestone` | none | researcher, synthesizer, roadmapper; `record_research`; `discuss_phase` |
| `discuss_phase` | none | assumptions analyzer, advisor researcher; `plan_phase` |
| `spec_phase` | none | `record_spec`; `discuss_phase` |
| `mvp_phase` | E516 | `plan_phase` |
| `plan_phase` | E201, W204, E206, W406, E516 | phase researcher, pattern mapper, planner, plan checker; `record_research`, `record_verdict`; `execute_phase` |
| `plan_review_convergence` | E201, W204, E206, W406, E516 | `plan_phase`; plan checker |
| `ultraplan_phase` | E201, W204, E206, W406, E516 | plan checker; `record_verdict` |
| `execute_phase` | W311, W313, W315, W318, E516 | `execute_plan`; verifier; `record_verdict`, `record_window`; `verify_work` |
| `execute_plan` (no slash command) | W318, E516 | `start_plan`, `answer_checkpoint`, `confirm_setup`, `raise_blocker`, `complete_plan`; executor |
| `verify_work` | E516 | `set_uat_result`; planner, plan checker |
| `validate_phase` | E516 | nyquist auditor; `record_validation`, `approve_validation` |
| `add_tests` | E516 | none |
| `audit_uat` | none | `set_uat_result` |
| `audit_milestone` | E516 | integration checker; `record_verdict`; `plan_phase` |
| `complete_milestone` | E106, E511, E516 | `close_milestone` |
| `ship` | E511, E512, E516 | `fix_window`, `waive_window` |

### What the lifecycle prompts do not do (known, not fixed)

The assets are upstream's text; where the spec has no counterpart a line was defaulted, not ported (each is a line in
`REWRITTEN` of `tests/port_workflows.rs`):

- Settings. `{{setting:<key>|<fallback>}}` reads the settings block. `commit_docs`, `features.graduation*`,
  `workflow._auto_chain_active`, `workflow.code_review_command` and `workflow.live_dom_uat` are not keys of the roster
  (`NOT_SETTINGS` in `tests/ported_text.rs`), so they always take their fallback, and a `settings set` on them would be
  refused. `USE_WORKTREES` is `false` (the `sequential` tail of `execute_phase`); `RUNTIME` is the rendered host.
- Defaulted gaps: worktrees, the loop hooks, `intel`, `resolve-model`, learnings and the dispatch-isolation steps.
- `init` JSON fields (`phase_dir`, `plan_count`, ...) have no counterpart: the text reads the graph (`specforge query`)
  and the ids named at the end of the text (`$PHASE_ID`, `$MILESTONE`, `$PROJECT`).
- `gsd/W506` looks at less than upstream's check: no plans, no modified-file contents and no commit bodies.
- The ledger scope in `ship` (`ship.md`, the broken-window step) is the milestone's, not upstream's.
- Cursor: upstream's section A (the Claude-only blocks) is dropped, and `~/.claude` paths read `~/.cursor`. Codex maps
  an agent's effort to `high` as upstream does (`tests/golden/host/` holds the probe and both conversions, made with
  upstream's converters).
- The twin has no commit, broken-window, human-item, methodology or lesson data, so the query tests do not reach them.
- The goldens (`tests/golden/prompts/`) are hashes plus the first 80 lines, the 60 lines after the preamble, the last 40 and
  the counts, not the whole text.
- `cargo gate full` is the check for all of it; a revendor or a kernel change is K20, not done here.

### What the r43 prompts do not do (known, not fixed)

Found by REVIEW-r43 and kept, each on purpose or as a kernel limit (K20):

- `update` has no upstream attribution: its text is ours (upstream's installs upstream GSD over this one), D-r43-16.
- `smart_entry` differs from upstream in three signals, all read from the computed graph (K18):
  - `progress` is the plan percent (summaries over plans), upstream's is STATE's percent (the twin reads 67% against 0%);
  - `verify_failed` means the current phase's `verification.status == gaps_found`; upstream's `detectVerifyFailed`
    fires only on `STATUS: blocked|failed`, so on gaps_found the port recommends `/gsd-verify-work` where upstream
    goes to `/gsd-progress --next`;
  - the current milestone is "the one in progress, else the last by id string" (`reports.rs`): v1.9 sorts after
    v1.10, a declared v1.1 wins over a not-started v1.0, and a shipped last milestone re-reports `complete`.
- The reviewer settings `review.models.*`, `review.timeouts.*`, `review.effort.*` and `review.max_prompt_tokens*` are
  declared and inert: `review` runs each CLI from its own `--help` (no lane manifest, no runner).
- `phase --promote` finds a backlog phase (`999.x`) in any milestone and moves it under the target one (its id follows
  its milestone, so does a `999.x` child); upstream's backlog is roadmap-wide. Whether a `commit` entity's `plan`
  reference follows is not checked.
- E080 counts the errors a backlog subtree already has as new ones, so a promote or an edit of it is refused until
  `specforge check` is clean; `gsd/E310` is silent on `999.x` and fires once the phase is numbered. This is the kernel's
  E080 (K20), not changed here; `review-backlog` tells the user to run `specforge check` first.
- A commit step of `autonomous`, `quick`, `review-backlog`, `pause-work` and `next` runs `git add spec && git commit`
  where upstream commits named files: spec edits the user has not committed are swept in.
- A lesson id `L-<phase>-<n>` is not unique across milestones: with `--reset-phase-numbers` the re-run rule (`specforge
  set` an existing id) overwrites an earlier milestone's lesson.
- `/gsd-settings` writes no `~/.gsd/defaults.json` (it is upstream GSD's store and nothing here reads it): there are no
  global defaults. `/gsd-config --integrations` never takes an API key in the conversation: the user sets the variable
  in their own shell profile.
- `workspace` takes `--name` only; `--repos`, `--strategy`, `--branch` and `--auto` are asked for or absent.

The other rows of the roster above are r43's. A rendered text may name them (the preamble says to skip what is not
installed); `tests/ported_text.rs` lists every such name.

## Writing commands

Every `H` field of the model has exactly one writer, a command whose edit plan the host checks and applies (K5), or a
collector. The workflows call these; none is a slash command. Round r41 and K18 (ADR 0299) build all of them (the digest writers read the computed context digest from the graph a command receives, ADR 0299). CLI names
are the ids with `-`: `specforge gsd complete-plan`.

| Command | Writes (`H` fields) |
|---|---|
| `start_plan`, `complete_plan` | summary `started`, `completed` (`start_plan` writes nothing: it runs the entry gates and answers the clock) |
| `answer_checkpoint` | checkpoint `answer` |
| `confirm_setup` | user_setup `done` |
| `set_uat_result` | uat_test `result`, `source` |
| `approve_validation` | validation `approval` |
| `close_milestone` | milestone `completion` (gated by gsd/E106, E511, E516), its retrospective |
| `pause`, `resume` | pause `at`, `by`; removes the pause |
| `raise_blocker`, `resolve_blocker` | blocker `raised`, `resolved` |
| `record_window`, `waive_window`, `fix_window` | broken_window `recorded_at`; `waiver`; `fixed_at` |
| `propose_refactor`, `dispose_refactor` | refactor_proposal `score`, `baseline`, `delta`, `reasons`, `candidates`; `disposition` |
| `record_research`, `record_spec`, `record_validation` | research, phase_spec, validation `context_digest` (the computed `digest` of the phase's context) |
| `health`, `stats`, `progress_report` (`/gsd-health`, `/gsd-stats`) | nothing: they report from the computed values of the graph |
| `smart_entry` (`/gsd-next`) | nothing: it is read-only, and classifies the project from its computed `status`, `backlog` and `verification.status` fields |
| `phase`, `workstreams` (the slash commands `/gsd-phase`, `/gsd-workstreams`) | phase and workstream entities: add, insert, remove (renumbering), edit; create, complete, switch |
| `record_verdict` (built, r40: `specforge gsd record-verdict`, MCP `specforge.gsd.record_verdict`) | a verdict (`V` fields), through `set_verdict` |
| the `gsd-git` collector (built, r38): `specforge collect --runner gsd-git --yes` | commit (every field) |
| the `gsd-junit` collector (built, r38): `specforge collect --runner gsd-junit --no-run` | none: it records test results in `specforge-report.json`, which the derive pass reads |

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

### Which agent judges which rule

The judged rules (K7) a `@gsd/core` agent judges; `specforge install` appends the list to the agent's file under
"## Judged rules" (`tests/cli/agents.rs` reads the rows from the judged table, `src/judged.rs`). `record_verdict` is
how the judge records its verdict; its body also ends with a section that names the command (the host's own list names only
`specforge.verdict_inputs`). The judge is the rule's own, but that is not an identity check: any agent can call
`record_verdict` and `--model` is free text, as any agent can write VERIFICATION.md upstream.

| Agent | Judges |
|---|---|
| `gsd-plan-checker` | `gsd/W311`, `gsd/W313`, `gsd/W315`, `gsd/W316` (a plan) |
| `gsd-phase-researcher` | `gsd/W205` (a package audit) |
| `gsd-integration-checker` | `gsd/E518` (a milestone audit) |
| prompt `spec_phase` | `gsd/E202` (a phase spec), `gsd/W203` (a locked requirement) |
| `gsd-verifier` | `gsd/E501` (a truth, an artifact, a key link without a `pattern`) |

Every other core agent judges nothing.

## Hooks

GSD's `hooks/` become 15 typed hook contributions (K9, r44). `specforge install <host>` wires each into the host as
`specforge hook <id>`; a host without that event gets none, and the install report lists it as `not_rendered`.

| GSD hook | Contribution |
|---|---|
| gsd-read-guard, gsd-write-guard | `read_guard`, `write_guard` (`pre_tool`): the paths a workflow may touch; `write_guard` measures bytes, not lines |
| gsd-secret-read-guard | `secret_read_guard` (`pre_tool`) |
| gsd-read-injection-scanner | `injection_scanner` (`post_tool`): scans what was read; advisory only (see below) |
| gsd-prompt-guard | `prompt_guard` (`pre_tool`): scans content written to spec files |
| gsd-agent-isolation-guard | `agent_isolation_guard` (`pre_tool`): allows every dispatch, because `@gsd/core` runs its executors sequentially in the checkout (upstream's sentinel `none`); the line is written for Claude Code and Cursor and does nothing on Cursor |
| gsd-worktree-path-guard | `worktree_path_guard` (`pre_tool`): path based; refuses the main checkout by any case and by a relative path, not through a symlink |
| gsd-workflow-guard | `workflow_marker` (`user_prompt`: a `/gsd-...` prompt sets the marker, which lasts the session) and `workflow_guard` (`pre_tool`, `hooks.workflow_guard`) |
| gsd-context-monitor | `context_monitor` (`post_tool`) and `context_compacted` (`pre_compact`): warnings at the `hooks.context_*` thresholds, metrics kept in session state |
| gsd-statusline | `statusline`, from the graph and the session state |
| gsd-validate-commit | `validate_commit` (`pre_commit`, `hooks.community`): `^([a-z][a-z0-9-]*)(\(.+\))?:\s.+`, the types plus `hooks.commit_types`, subject at most 72 characters |
| gsd-phase-boundary | `phase_boundary` (`post_tool`, `hooks.community`): after a spec file is written, reminds that status and progress are computed; it writes nothing |
| gsd-session-state | `session_state` (`session_start`, `hooks.community`): shows the computed state, or "No project found - suggest /gsd-new-project ..." |
| (core) `core.spec_guard` | wired by every install: `pre_tool` write on `.spec` refuses a raw write of a computed, harness or human-only value and names the command that writes it |
| gsd-ensure-canonical-path, gsd-check-update, gsd-check-update-worker, gsd-update-banner, gsd-config-reload, gsd-graphify-update, gsd-node-runner | none, each with its reason in `src/hooks/roster.rs` |
| gsd-cursor-pre-tool, -post-tool, -session-start | the Cursor renderings of the events above |
| gsd-cursor-stop, -subagent-start, -subagent-stop, gsd-windsurf-* | none: no hook of ours registers on those events, and Windsurf is a later host |

At defaults an install writes 11 hook lines for Claude Code, 8 for Codex and 8 for Cursor; with `hooks.community` and
the other booleans on, 15, 12 and 12. Codex does not render `injection_scanner`, `agent_isolation_guard` and
`statusline`; Cursor does not render `workflow_marker`, `context_compacted` and `statusline`.

### Hosts

No host is run, so Codex and Cursor stay drafts. The table cross-checks the install against upstream's own installer.

| Host point | Claim here | Upstream evidence | Status |
|---|---|---|---|
| Cursor hook events | preToolUse, postToolUse, sessionStart, stop, subagentStart, subagentStop | `CURSOR_HOOK_EVENTS`, `imperative-hook-bus.cts:35-42` | agrees |
| Cursor answer | `{block, reason, additional_context}` | `gsd-cursor-*.js` | agrees |
| Cursor subagentStart answer | `permission: deny` + `user_message` | `gsd-cursor-subagent-start.js:26-31` | unverified (no gsd hook answers it) |
| Codex `[features] hooks = true` | written by install | changeset 3566, `bin/install.js` | agrees |
| Codex event set | the events of `CODEX_EXTENDED_HOOK_EVENTS` (nine, with SessionStart), five used here | `bin/install.js:162` | agrees for the five used |
| Codex hook copies | 8 of the 14 hook ids (`injection_scanner`, `agent_isolation_guard` and `statusline` are not rendered; the community hooks only when on) | upstream copies two (`CODEX_HOOKS_TO_COPY`) | divergence: more here |
| Codex and Cursor agent and skill files | as the kernel renders | not compared | unverified |

### What the hooks do not do (known, not fixed)

- The context monitor has no stopped-at record and no Stop or SubagentStop rows.
- The `git add -f` block of upstream's workflow guard is dropped (it needs git facts, K20).
- The statusline is plain text (the kernel escapes control characters); `show_git` and `show_state_freshness` are
  inert; phase counts read completed over total; the milestone shown is the one `reports::current` chooses.
- Injection scanning is advisory: `security.injection_blocking` belongs to `@gsd/security` and a hook cannot read
  another extension's settings.
- `validate_commit` lets `-F`, `--fixup` and similar commits through (no message on the command line), and a literal
  `fixup! ...` subject; the heredoc form `$(cat <<'EOF' ... EOF)` is read only for a quoted delimiter, nothing after the
  closing paren, no `--cleanup` but default or whitespace (git's abbreviations and spliced quotes are read, and every word of the command is scanned) and an `-m` that is not single-quoted (as upstream blocks the
  others). It blocks `git commit -m $MSG` (unquoted) and `-m "" -m "feat: x"`, which upstream lets through.
- The milestone-choice weakness (`v1.9` sorts after `v1.10`, id-string order) is in `smart_entry`, the statusline and
  `session_state`.
- The statusline reads `persistence (1/4)` as "1 of 4 phases complete" (not the phase being executed); its context
  meter has no auto-compact-off branch, so it warns early when compaction is off.
- The workflow marker lasts the session: one `/gsd-*` prompt mutes the workflow guard until the session ends, and a
  skill the model invokes sets none. `sessions/<id>.json` files are never pruned.
- A session patch with no session shows the user a `systemMessage` ("the state patch names a session, and this run has
  none").
- The injection patterns' regex engine is quadratic: a long hostile `](http://a:a:...` link takes seconds (0.27 s at
  2000 repeats, 2.4 s at 8000).
- Hooks of the injection family judge a file as in the spec root only when it is a `.spec` file (the kernel calls
  every path "in the spec root" when `spec_root` is `.`).
- `write_guard` continues when an existing file's bytes are unreadable; upstream fails closed. `secret_read_guard`
  reads the raw `command`, not shell words. `prompt_guard` does not scan a Codex `apply_patch`. The Codex "any
  `session_id` is silent" rule of `read_guard` does not hold: Codex payloads carry one and the advice names the Read tool.
- A host without `specforge` on its PATH runs no guard (exit 127 does not block).
- Hook latency (p50, 20 events, taskboard): `write_guard` 0.13 s, `spec_guard` 0.16 s; the budget is enforced by
  `hooks_guards.rs`.
- Uninstall leaves the empty host root directory (the install crate).

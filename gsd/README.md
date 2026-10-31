# GSD in SpecForge (`@gsd/*`)

**Status: the kernel changes are built (protocol 1.12.0); `@gsd/core` 1.0.0 is built and tracks gsd-core 1.18.0: its model, checks, 15 agents, 30 commands, 44 prompts, 50 slash commands and 14 hooks, installed for Claude Code (verified) and Codex and Cursor (drafts), and three of the parity suite's four checks run in the gate; the fourth (upgrade) is a procedure ([acceptance checklist](parity/ACCEPTANCE.md), which lists the named gaps: hooks are 15 of 28, the `specforged` harness is later, Codex and Cursor are unrun). The other `@gsd/*` extensions are not built.** This folder holds the model, the contribution rosters and the parity
plan for bringing the whole GSD framework ([open-gsd](https://github.com/open-gsd)) into SpecForge. It lives in this
monorepo for now and moves to its own repository later; nothing in it may depend on a path outside `gsd/`.

## The goal

Any agent or engineer uses GSD's mental model (milestone → phase → plan → task, waves, requirements, decisions,
verification, UAT) through SpecForge's own surfaces: MCP, CLI and LSP. SpecForge is the central system: every piece of
project state is a `.spec` entity, there is no `.planning/` folder and no Markdown state.

```
┌────────────────────────── @gsd/core (the unified meta-model) ──────────────────────────┐
│ project · requirement · milestone · phase · plan · task · decision · research ·        │
│ summary · verification · uat · validation · handoff · settings                         │
└───────────┬──────────────────────────────┬──────────────────────────────┬──────────────┘
   profiles │                capabilities  │                       tooling│
┌───────────┴───────────┐  ┌───────────────┴──────────────┐  ┌────────────┴─────────────┐
│ @gsd/path  @gsd/loop  │  │ @gsd/security  @gsd/ai       │  │ collectors: tests, git,  │
│ only their new        │  │ @gsd/ui  @gsd/codebase       │  │ browser, inbox           │
│ concepts, by enhance  │  │ @gsd/ops                     │  │ @gsd/import (one-way)    │
└───────────────────────┘  └──────────────────────────────┘  └──────────────────────────┘
```

The stack is complete on its own: a GSD project loads only `@gsd/*`, never `@specforge/product`, `governance`,
`testing`, `cargo-test` or `vitest`. That is policy, not kernel-enforced: `@specforge/product` (`milestone`,
`deliverable`) and `@specforge/governance` (`decision`) declare kinds `@gsd/core` also declares, and loading both is
E026 (the first registration wins), so `specforge check` fails on a real clash. `@specforge/testing` declares no kind,
so `@gsd/core` ships its own JUnit test collector instead of relying on a test-runner extension.

## Decisions

Settled in a design interview on 2026-10-26. The kernel changes they need, K1–K10, K11–K16, K17, the call graph of
ADR 0299 and the baseline's source files (K19), are built and accepted as ADRs;
the rest of this folder keeps their K names.

| Draft | ADR | Kernel change |
|---|---|---|
| K1 | [0283](../docs/adr/0283-a-name-may-hold-a-dash-and-a-dot-where-a-name-is-expected.md) | a name may hold a dash and a dot where a name is expected |
| K2 | [0284](../docs/adr/0284-a-name-is-unique-within-its-scope.md) | a name is unique within its scope |
| K3 | [0285](../docs/adr/0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md) | a field's type may be a list of words, a block, a union or a range |
| K4 | [0287](../docs/adr/0287-who-writes-a-field-is-declared.md) | who writes a field is declared (`computed`, `harness`, `immutable`, `append_only`) |
| K5 | [0289](../docs/adr/0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) | an extension writes through an edit plan the host applies |
| K6 | [0288](../docs/adr/0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md) | an extension's settings are typed and set in `specforge.json` |
| K7 | [0290](../docs/adr/0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) | a judged rule holds while a fresh verdict says so |
| K8 | [0291](../docs/adr/0291-an-extension-contributes-mcp-prompts.md) | an extension contributes MCP prompts |
| K9 | [0292](../docs/adr/0292-an-extension-contributes-agents-skills-commands-and-hooks.md) | an extension contributes agents, skills, commands and hooks |
| K10 | [0286](../docs/adr/0286-a-diagnostic-code-is-namespaced-by-its-extension.md) | a diagnostic code is namespaced by its extension |
| K11 | [0293](../docs/adr/0293-a-pass-or-command-declares-the-project-files-it-reads.md) | a pass or command declares the project files it reads (file facts) |
| K12 | [0293](../docs/adr/0293-a-pass-or-command-declares-the-project-files-it-reads.md) | compile, check and guard passes receive the recorded test report (`reads_tests`) |
| K13 | [0294](../docs/adr/0294-a-pass-sees-a-list-field-as-its-items-and-a-writing-call-is-told-the-time.md) | a pass gets list fields as items |
| K14 | [0294](../docs/adr/0294-a-pass-sees-a-list-field-as-its-items-and-a-writing-call-is-told-the-time.md) | a writing call knows the time (`now`) |
| K15 | [0295](../docs/adr/0295-a-judged-rules-judge-may-be-a-prompt-of-its-extension.md) | a judged rule's judge may be a prompt of its extension |
| K16 | [0296](../docs/adr/0296-settings-are-set-and-unset-through-specforge-settings.md) | settings are set and unset through `specforge settings` |
| K17 | [0297](../docs/adr/0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md), [0298](../docs/adr/0298-an-agent-does-not-change-or-delete-what-a-person-wrote.md) | a judged rule's finding is not an error a write adds, a collector's plan is judged with its report, and an agent does not change a human-only entity or delete a person's value |
| K18 | [0299](../docs/adr/0299-the-graph-a-call-receives-carries-what-every-surface-shows.md) | the graph a call receives carries the computed values and judgements every surface shows |
| K19 | [0300](../docs/adr/0300-a-compute-pass-at-the-baseline-reads-the-files-the-commit-holds.md) | a compute pass at the baseline reads the files the commit holds |
| K20a | [0301](../docs/adr/0301-a-human-attested-command-runs-only-for-a-person-at-a-terminal.md)-[0306](../docs/adr/0306-install-refuses-an-unusable-specforge-json-and-uninstall-removes-only-what-install-created.md) | trust, verdicts, rules, settings and install safety |

K20a is built (protocol 1.12.0). K20b and K20c are the queue of kernel items that still wait ("What still waits for kernel work" below); none of them is built.

Building `@gsd/core` against them found six more gaps no model fix closes. They are a second kernel round (r35,
protocol 1.11.0, now in the kernel), and the model cites them where it relies on them: **K11** a pass or command reads the project files
it declares (file facts), **K12** compile, check and guard passes receive the recorded test report, **K13** a pass gets
list fields as items, **K14** a writing call knows the time (`now`), **K15** a judged rule's judge may be a prompt,
**K16** `specforge settings set|unset` writes settings through SpecForge. Reviewing r40 found three more (no protocol change, no re-vendor): **K17** a judged rule's finding is not an error a write adds (a failing verdict, a new judged entity and a staling edit are written), a writing collector's plan is judged with the report its collect wrote, and an agent does not change a human-only entity or delete a person's value. ADR 0299 (no protocol change) puts the computed values and each entity's judgements in the graph a call receives, so a command reads a phase's `status` and a context's `digest`.

### Parity contract

| Decision | |
|---|---|
| Scope | The whole GSD ecosystem. `gsd-core` is the unified meta-model; `gsd-path` and `gsd-loop` are profile extensions over it. `gsd-pi`, `gsd-pi-config`, `gsd-cloud-daemon` and the marketplace are dropped: SpecForge's surfaces replace them. |
| Parity means | The same **concepts**, the same **commands** (`/gsd-*`, by exact name) and the same **agent behaviour**. Not the same files: no `.planning/`, no export. |
| Parity and more | Parity is the floor. The default profile `gsd-compat` behaves as GSD does; `gsd-strict` and `gsd-plus` switch on what SpecForge can do beyond GSD. |
| Versions | Our extensions keep their own semver and declare what they track: the crate's `[package.metadata.gsd] tracks = "gsd-core@1.18.0"` and the handshake keyword `tracks:gsd-core@1.18.0`. Only the latest tracked GSD is supported; projects move forward by migration. |
| Upgrades | `gsd/parity/upgrade.sh <gsd-core-dir> <tag>` diffs the tracked tag against the new one (inventory manifest, capability ids, config manifest, changed files) and writes a parity backlog, `gsd/parity/backlog/<tag>.md`, one row per change with the table that owns it; `tracks` moves only when the [parity suite](parity/) passes. |
| Adoption | `specforge gsd import` reads an existing `.planning/` once and writes `.spec` (edit plans). The parity suite uses it too. |

### Model

| Decision | |
|---|---|
| Baseline | Modelled anew, not ported file by file. The model is the most important piece and should use what SpecForge does that shell scripts cannot. See [core/spec](core/spec/). |
| Derived values | Phase status, waves, progress, requirement status, halted/blocked, milestone state: **computed only** (`[C]`), never written in `.spec`; writing one is an error (K4). |
| Prose | Split into typed entities (research findings, pitfalls, hypotheses, concerns…); text fields hold only what stays narrative. |
| Judgement | A check code cannot decide (scope sanity, context compliance, goal achievement) is a **judged rule**: a recorded verdict, stale when its input changes (K7). |
| Identifiers | `AUTH-01`, `03-02`, `03.1` are identifiers where a reference is expected (K1); `D-01` is unique within its phase, `03/D-01` from outside (K2). |
| Toggles | GSD's boolean and valued config become typed extension settings in `specforge.json` (K6). Capabilities that bring their own kinds are separate `@gsd/*` extensions, except broken windows: its writers (executor, verifier, refactor trigger) are all core, so `broken_window` and `refactor_proposal` are core kinds. |
| Codes | Diagnostics are namespaced per extension: `gsd/E101`, `gsd.security/W003` (K10). |

### Runtime

| Decision | |
|---|---|
| Where | In SpecForge. Commands, agents, workflows and hooks are SpecForge contributions (K8, K9). |
| Who orchestrates | The agent, first: workflows are MCP prompts the host agent follows, and a step whose gate fails is refused. A `specforged` harness runs unattended work (`/gsd-autonomous`) later. |
| Writes | An extension returns an **edit plan**; the host checks it against the field flags and applies it, the only writer (K5). |
| Hooks | Typed contributions; `specforge install <host>` wires them into the host as `specforge hook <id>` (K9). |
| Git | The agent runs git; a collector records commits as evidence linked to their tasks. |
| Hosts | Claude Code, Codex and Cursor first, plus any MCP client. Host Markdown (agents, skills, commands) is rendered output, never source. `specforge install <host>` writes project-local files only and owns only what it marks. |
| Prompts | GSD's agent and workflow prompts are ported under MIT with attribution; only the parts naming `.planning/` files or `gsd-tools` change, to name SpecForge's typed tools. |

### A project's layout

```
spec/gsd/
├── project.spec                      project, settings that are not flags
├── requirements/<category>.spec      AUTH-01, AUTH-02 …
├── milestones/<v>/
│   ├── milestone.spec
│   └── phases/<NN-slug>/
│       ├── phase.spec                goal, success criteria, must-haves
│       ├── context.spec              decisions D-01 …, deferred ideas
│       ├── research.spec             findings, stack choices, pitfalls, open questions
│       ├── plans/<NN-MM>.spec        plan, tasks, checkpoints, and its summary
│       └── verify.spec               verification, UAT, validation
├── workstreams/<name>/               a scope: its own milestones and phases
├── knowledge/                        codebase map, project research
└── ops/                              debug, threads, todos, seeds, backlog, committed pause and handoff
specforge-harness                     digests of applied harness values (K4), committed, kept canonical by the host
.gitattributes                        specforge-harness merge=union
.specforge/state/gsd__core/                 session cursor, active workstream, harness checkpoints (git-ignored; written by the host, K9)
```

A plan's results are written to its own file and every edit names the revision it was planned on, so parallel
executors in one wave never write the same file and a stale edit is refused (K5). A workspace is another
checkout of the same tree and is not modelled.

## GSD capabilities, one by one

Every `capabilities/*/capability.json` of gsd-core 1.18.0 and where it lands.

| GSD capability | Role | Lands in |
|---|---|---|
| research, gap-analysis, pattern-mapper, assumption-delta, nyquist, tdd, schema-gate, refactor-trigger, broken-windows, audit | feature | `@gsd/core` settings and rules; broken windows also adds the `broken_window` kind |
| drift | feature | `@gsd/codebase` (codebase drift reads the codebase map) |
| claude-orchestration, external-job | feature | `@gsd/core` harness (`specforged`), later: only their settings keys exist |
| code-review, mempalace, profile-pipeline | feature | `@gsd/ops` |
| security | feature | `@gsd/security` |
| ai-integration | feature | `@gsd/ai` |
| ui, live-dom-uat | feature | `@gsd/ui` (DOM checks through the browser collector) |
| intel, graphify | feature | `@gsd/codebase` (graphify is absorbed: SpecForge is the graph) |
| coderabbit, ollama, lm-studio, llama-cpp | reviewer | `@gsd/core` settings: transports for cross-AI review |
| claude, codex, cursor | runtime | install renderers, first |
| opencode, copilot, windsurf, kilo, kimi, kimi-code, antigravity, cline, trae, qwen, hermes, codebuddy, augment, zcode, vscode, pi | runtime | install renderers, later, one at a time |

## Folders

| Folder | What |
|---|---|
| [core/](core/) | `@gsd/core`: the model ([core/spec](core/spec/)), the contribution rosters, later the extension crate |
| [path/](path/), [loop/](loop/) | profile extensions: only what gsd-path and gsd-loop add |
| [security/](security/), [ai/](ai/), [ui/](ui/), [codebase/](codebase/), [ops/](ops/) | capability extensions with their own kinds |
| [import/](import/) | the one-way `.planning/` → `.spec` importer |
| [parity/](parity/) | the parity harness, GSD fixture projects and the version matrix |

Each extension builds to `wasm32-wasip2` on its own; the Cargo workspace excludes `gsd/` as it excludes `extensions/`.

## Build order

The packaging (one Cargo workspace, built on demand, tested through the `specforge` binary) is
[ADR gsd/0001](adr/0001-one-cargo-workspace-built-on-demand.md).

1. The kernel changes, in order: K1 identifiers, K2 scoped identifiers, K10 namespaced codes, K3 field types,
   K4 field flags, K5 edit plans, K6 settings, K7 judged rules, K8 contributed prompts, K9 contributed
   agents, skills, commands and hooks.
2. All of `@gsd/core`, with the git and test collectors and the parity suite on one fixture project. The foundation (the
   roadmap kinds, the code book, the settings and the roadmap checks) is built; see [core/](core/).
3. The capability extensions, then `@gsd/path` and `@gsd/loop`, then more host renderers.

## Waiting on the kernel

ADR 0299 put the computed values and judgements in the graph a call receives, so a command reads a phase's `status`
and a context's `digest`, and `record_research`, `record_spec`, `record_validation`, `/gsd-health`, `/gsd-stats` and the progress
report are built. K17 closed the E080 treatment of a judged rule's finding, a collector's plan judged before its own report, and the
human-only guarantee on existing entities and deletes (ADR 0297, ADR 0298), and `gsd/E502` is built on top of them.
ADR 0300 serves a commit's source files to the baseline, so `re_verification` reports `previous_status` and
`previous_score` and compares scan states (r39 T13 is built). K20a is built: the six answer commands are a person's (human-attested), a verdict is trusted against its tests and its files, and install and uninstall are safe. One divergence from upstream: a truth's `verification` tier is a person's, so `/gsd-phase --remove` of a phase whose truth holds one is refused `human_only` until the person deletes that line in their editor (upstream deletes regardless; REVIEW-k20a M3). What still waits for kernel work (K20b, K20c):

- **`--ws` is text.** A command or prompt reads its extension's state (`call.state()`, ADR 0310), so `/gsd-workstreams
  status` and `list`, `/gsd-progress` and `/gsd-resume-work` name the active workstream; the `--ws` argument stays free
  text.
- **An agent cannot create an artifact through the CLI (K20)**: `--path` is the CLI's own option, MCP's
  `specforge.edit` creates it. It is a CLI change.
- **The lifecycle prompts (r42) leave three kernel items (K20).** The claude-code skill writer does not write a skill's
  `effort`, so `/gsd-plan-phase` and `/gsd-execute-phase` declare `max` and only the declaration is tested. A reference is the id as written, so `3` and `03` are different spellings of the same phase.
- **The remaining prompts (r43) leave these (K20).** A prompt cannot see other extensions' contributions, so the routers' route table and `/gsd-next` read a static roster (`src/prompts/roster.rs`) that says which extension owns a command. A command's answer is not host-converted, so `smart_entry` takes `--runtime`. `when` takes one setting, so `/gsd-review`, `/gsd-update` and `/gsd-ns-ideate` follow their first cluster only.
- **What r43 leaves open.** Reports, summaries and cross-AI reviews are shown, not stored. A lesson has no phase field. `/gsd-review` has no reviewer lane machinery or prompt budget (the agent reads each reviewer CLI's `--help`). `/gsd-quick-batch`'s dispatcher, estimate calibration, sub-repository branches and workspaces are prose. `/gsd-undo` selects the commits recorded as `commit` entities, and `git revert` bypasses the host's checks, so it asks before reverting a `.spec` change that removes a person's value. `smart_entry` never says idle-stranded (a command has no git).
- **The hooks wait for five host items (K20)**: git facts in `HookInput` (the workflow guard cannot see a
  `git add -f`, nor `validate_commit` the staged files), a `file_changed` event (settings are read on every compile instead), a hook that runs
  without a full compile (every hook pays one), Cursor's `beforeSubmitPrompt` (`workflow_marker` is not rendered for
  Cursor), and a hook reading another extension's settings (`injection_scanner` is advisory, because
  `security.injection_blocking` is `@gsd/security`'s).
- **The refactor trigger differs from upstream's.** `propose_refactor` records one proposal per function while one is
  untriaged (it answers `nothing new` and writes nothing, so a hook may call it on every phase event), as upstream
  freezes the anchor. It does not port upstream's anchor on first observation: the baseline is the score of the latest
  *disposed* proposal, so a function that creeps below the threshold in small steps never fires `jump` until it crosses.
- **`specforge <ext> help <cmd>` is gone for every extension.** T17 turned off clap's `help` subcommand on an
  extension's command line (the extension's own `help` command shadowed it). Use `specforge <ext> <cmd> --help`;
  `specforge gsd help` lists the commands.
- **A slash command's `when` takes one setting (K20).** A skill in two clusters follows its first, so disabling
  `utility` leaves `/gsd-help` (also `core_loop`) shown.
- **What stays after K17.** `check` is red from planning on, since gsd/E501 is an Error from the first planned truth
  until it is verified, and an agent cannot create an artifact through the CLI (`--path` is the CLI's own option).
- **Hook facts and the install path (K20).** A hook is told the name of an out-of-project path, not its real path, so a
  symlink into the main checkout passes `worktree_path_guard`; it is not told a commit's `--cleanup`; a hook
  contribution cannot name its hosts (Cursor gets the isolation guard's line, which does nothing there). A host
  without `specforge` on its PATH runs no guard.

## What is not in the program

| Not built | Why | Where it is named |
|---|---|---|
| The other `@gsd/*` extensions (ai, ui, codebase, ops, security and the rest) | each is its own program; the 10 capabilities, 51 gate-cutover verdicts and 35 config keys they own are listed, not built | [ACCEPTANCE.md](parity/ACCEPTANCE.md) |
| Renderers for the other hosts (16 capabilities) | later install renderers | the capability table above |
| 14 gate-cutover verdicts | 11 no twin reaches, 3 a twin answers differently | `core/tests/parity_rules/cutover.rs`, [ACCEPTANCE.md](parity/ACCEPTANCE.md) |
| The `specforged` harness | `/gsd-autonomous` unattended; `claude-orchestration` and `external-job` have settings keys only | the capability table above |
| The other extensions' commands and agents | 22 commands and 39 agents are owned by them | [ACCEPTANCE.md](parity/ACCEPTANCE.md) |
| Codex and Cursor verified in the host | drafts until run there | [ACCEPTANCE.md](parity/ACCEPTANCE.md) |
| Publishing | `specforge publish` is a user action | [ACCEPTANCE.md](parity/ACCEPTANCE.md) |

## Credit

GSD is Copyright (c) 2026 Open GSD, MIT licence. Ported prompts keep their notice and name the upstream file and version
they came from.

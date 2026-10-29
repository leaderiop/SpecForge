# GSD in SpecForge (`@gsd/*`)

**Status: the kernel changes are built (protocol 1.11.0); `@gsd/core` has its foundation and its discuss, specify, research, plan and execute kinds (kinds, code book, settings, roadmap, discuss, research, plan and execute checks, and the `gsd-git` and `gsd-junit` collectors) and the other `@gsd/*` extensions are not built.** This folder holds the model, the contribution rosters and the parity
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

Settled in a design interview on 2026-10-26. The kernel changes they need, K1–K10 and K11–K16, are built and accepted as ADRs;
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

Building `@gsd/core` against them found six more gaps no model fix closes. They are a second kernel round (r35,
protocol 1.11.0, now in the kernel), and the model cites them where it relies on them: **K11** a pass or command reads the project files
it declares (file facts), **K12** compile, check and guard passes receive the recorded test report, **K13** a pass gets
list fields as items, **K14** a writing call knows the time (`now`), **K15** a judged rule's judge may be a prompt,
**K16** `specforge settings set|unset` writes settings through SpecForge.

### Parity contract

| Decision | |
|---|---|
| Scope | The whole GSD ecosystem. `gsd-core` is the unified meta-model; `gsd-path` and `gsd-loop` are profile extensions over it. `gsd-pi`, `gsd-pi-config`, `gsd-cloud-daemon` and the marketplace are dropped: SpecForge's surfaces replace them. |
| Parity means | The same **concepts**, the same **commands** (`/gsd-*`, by exact name) and the same **agent behaviour**. Not the same files: no `.planning/`, no export. |
| Parity and more | Parity is the floor. The default profile `gsd-compat` behaves as GSD does; `gsd-strict` and `gsd-plus` switch on what SpecForge can do beyond GSD. |
| Versions | Our extensions keep their own semver and declare what they track: the crate's `[package.metadata.gsd] tracks = "gsd-core@1.18.0"` and the handshake keyword `tracks:gsd-core@1.18.0`. Only the latest tracked GSD is supported; projects move forward by migration. |
| Upgrades | Per upstream release a diff tool (inventory manifest, `capability.json`, templates, prompts, gates) writes a parity backlog; `tracks` moves only when the [parity suite](parity/) passes. |
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
.specforge/state/gsd/                 session cursor, active workstream, harness checkpoints (git-ignored; written by the host, K9)
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
| claude-orchestration, external-job | feature | `@gsd/core` harness (`specforged`) |
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

Six things r39 and r40 could not build, or could not guarantee, wait for kernel work (K17):

- **Baseline re-verification (T13).** `re_verification.previous_status` and `previous_score` are not built, and the
  baseline view of an artifact or key link is not used. The git adapter loads only `*.spec` and `specforge-harness` at a
  commit, so at the baseline every source file reads as absent (`exists: false`): the scan says `missing`, never
  `unchecked`, and the baseline's status and score are wrong. `re_verification` therefore compares fresh verdicts and
  verify gaps only. The baseline compile must serve source files from the commit through the on-demand blob fetch
  (with `baseline-format` 3). Its `regressions`, a verdict-based `gaps_closed` and a reopened gap are tested on
  synthetic baselines, not black-box (the judgement list is empty on a baseline).
- **`gsd/E502`** is not built (deferred, not blocked). Its first reason, "recording a `verified` while a failing test is
  on record would be refused by E080", is wrong: that refusal is what E502 is for. The probe the plan asked for
  (`tests/cli/e502_probe.rs`) shows the other order cannot trip it either: a failing report collected after a pass
  changes the truth's `tests` input, the verdict goes stale, and E502 needs a *fresh* passing judgement. So E502 can be
  built as a `reads_tests` check. Until it is, `record_verdict` takes a `verified` on an explicit truth whose recorded
  tests include a failing one, and the verification can pass: the fail-open E502 exists to close
  (`a_verified_truth_with_a_failing_test_records_because_e502_is_not_built`, and the ignored
  `e502_refuses_a_verified_truth_with_a_failing_test`). `covered_digest` is built, and the commit titled for E502
  (`gsd/E502 an explicit truth is verified only with a named passing test, and the verification digests what it covered`)
  built only that.
- **`gsd/W514`** is not built: a pass is sent no settings, and a setting gates a code only while it is on. It needs
  `SettingDescriptor.gates_when_on`.
- **A verdict's own state counts as a new error (E080).** The host re-checks the project an edit would leave and keys
  a diagnostic by its code, severity, file and message shape (`crates/app/ops/src/authoring/edit/validate.rs`). A
  judged rule's message changes with its verdict (`...: no verdict`, `...: the verdict of <date> is '<outcome>' (<n>
  findings)`, `...: is stale: ...`), and a **recorded `passed` emits no diagnostic at all**. Everything below follows,
  and each case is pinned by a live test with an `#[ignore = "K17"]` twin (`tests/cli/judged_refusals.rs`,
  `judged.rs`, `verify_verdicts.rs`, `discuss.rs`):
  - **A failing verdict is refused whole** when the rule's effective severity is an Error: gsd/E501 `failed`,
    `uncertain` and `present_behavior_unverified`, gsd/E518 `issues`, gsd/E202 `failed`, and under `gsd-strict` gsd/W311,
    W313, W315 `issues`, gsd/W203 `failed` and gsd/W205 `suspicious` or `unknown`:
    `error[E080]: the edit would leave the project with 1 error it does not have; nothing was written`.
  - **A passing verdict cannot be withdrawn through `record_verdict`.** A `failed` (or `issues`) that would replace a
    fresh `verified` (or `passed`) is the same refusal, so a judge that finds a regression cannot say so: the old pass
    stays fresh, the verification stays `passed` and the phase `complete`, and a `checked` plan stays `checked`. This is
    **fail-open on revocation**. Code files are not E501 inputs, so a pass stops counting only when something it read
    changed: its truth, its plan, a file it **cited**, or a test result. Cite the files you judged. Today only a person
    editing the `.spec` by hand removes a pass.
  - **Creating a judged target is refused** through any typed agent-facing write (MCP `specforge.new`, a piped CLI, an
    extension command): a new truth, artifact, key link, `phase_spec` or `milestone_audit` adds a `no verdict` Error of
    a shape already present, and under `gsd-strict` so does a new plan (W311, W313, W315). A gap-closure plan's truths
    cannot be created by an agent at all. An agent that writes `.spec` files directly bypasses E080.
  - **An edit that stales a passing Error verdict is refused** (a truth's text, its plan's fields, a checked plan
    under strict). The worst case is gsd/E518: its `requirements` input is every requirement, so once a milestone
    audit's integration verdict passes, every requirement edit and creation is refused.
  - `check` is red from planning on: gsd/E501 is an Error from the first planned truth until it is verified.
  - The kernel also reports `(0 findings)` in every judged message (it counts a list of values, not of blocks).
  `@gsd/core` therefore ships fail-closed only when no verdict exists yet; once a pass exists it cannot be revoked. The
  smallest kernel change (K17): in `validate.rs::check`, a diagnostic whose code is a judged rule's goes to the
  reported-not-refused remainder, because it records a verdict that is owed, not a defect of the edit. Status
  derivations stay fail-closed, since they read `Judgement.passed`, never the diagnostic.
- **A `file()` input names a file-reference field only.** `gsd/E501` and the plan checks read no file as an input,
  because a plan, an artifact and a key link name files that do not exist yet and the host checks every file-reference
  field for existence (E016). The verifier cites the files it read (`cited`); a file it did not cite can change
  without staling its verdict. Likewise `gsd/E518`'s `requirements` input is every requirement: a selector cannot go
  on past a reference (`phase.milestone.requirements`).
- **Human-only on existing entities.** Closed for `override` in r40: the host's `edit_flags` checks the kind's writer
  on create, delete and retitle, and a *field's* writer on `set`, `unset` and `link`, so every field of `override` is
  marked human and an agent can no longer change or re-point one
  (`an_agent_cannot_edit_an_existing_override`). What stays: an agent can delete a milestone audit that carries a
  person's `debt_accepted` (gsd/E106 then refuses the gate), because the kind is not human; the general fix is K17.

## Credit

GSD is Copyright (c) 2026 Open GSD, MIT licence. Ported prompts keep their notice and name the upstream file and version
they came from.

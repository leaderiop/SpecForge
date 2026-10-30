# Acceptance: `@gsd/core` 1.0.0 against gsd-core 1.18.0

This is the checklist for the core program (r36 to r44). Each row counts the items of one family of the vendored
inventory (`parity/upstream/v1.18.0/`): how many `@gsd/core` builds, how many another `@gsd/*` extension owns, how many
do not apply to SpecForge, and how many are named and not built. `core/tests/parity_roster.rs`
(`the_acceptance_counts_match`) computes every number from the tables the checks already hold and fails when this file
disagrees. Every row but Capabilities is computed from tables of code; Capabilities is parsed from the capability table of
`core/README.md`, so that table is the source of its numbers.

| Family | Total | Built | Owned later | Not applicable | Named open | Where |
|---|---|---|---|---|---|---|
| Commands | 72 | 50 | 22 | 0 | 0 | `core/src/prompts/roster.rs` |
| Agents | 64 | 25 | 39 | 0 | 0 | `core/src/agents/roster.rs` |
| Hooks | 28 | 16 | 0 | 10 | 2 | `core/src/hooks/roster.rs` |
| Capabilities | 45 | 17 | 10 | 0 | 18 | `core/README.md` capability table |
| Workflows and references | 295 | 233 | 53 | 9 | 0 | `core/tests/parity_roster/assets.rs` |
| CLI modules | 282 | 168 | 22 | 92 | 0 | `core/tests/parity_roster/modules.rs` |
| Config keys | 211 | 161 | 35 | 15 | 0 | `core/spec/settings.md` |
| Gate-cutover verdicts | 171 | 92 | 51 | 14 | 14 | `core/tests/parity_rules/cutover.rs` |
| Codes | 63 | 61 | 0 | 0 | 2 | `core/tests/parity_rules/evidence.rs` |

Concepts (the model: phases, plans, milestones, requirements, decisions, UAT, verification) are the kinds of
`core/spec/*.md`; they have no inventory family, so no count. They are exercised by every row above.

## Status by family

- Commands: all 72 are accounted. The 50 core commands are `specforge gsd <command>` verbs or prompts; the rest belong
  to the other extensions. The `parity_roster` check fails on a command the snapshot has and the roster lacks.
- Agents: 25 of the 64 items (the core agents and their compact forms) are embedded and installed; the other 39 belong
  to `@gsd/ai`, `@gsd/ui`, `@gsd/codebase` and `@gsd/ops`.
- Hooks: 16 of the 28 upstream hook items are built (13 files ported as 15 hooks of `@gsd/core`, and the 3 Cursor
  renderings of events they port); 10 are dropped, each with its reason in `hooks/roster.rs` (7 not ported, and the
  Cursor `stop`, `subagent-start` and `subagent-stop` files, which no hook of ours feeds); the 2 Windsurf files are
  named and open (Windsurf is a later host). `core.spec_guard` is the kernel's and is
  wired by every install.
- Capabilities: 17 built (14 in `@gsd/core`, 3 as install renderers), 10 owned by an extension, 18 named and open: 16
  renderers for hosts SpecForge does not install for yet, and `claude-orchestration` and `external-job`, which need the
  `specforged` harness (only their settings keys exist).
- Workflows and references: 233 are embedded or replaced by a typed command, 53 are owned, 9 do not apply; each of the
  77 not embedded names why in `assets.rs`.
- CLI modules: 168 are ported or are the host half of a `specforge` command, 22 are owned, 92 do not apply (the argv
  router, the capability loader, the runtimes SpecForge does not install for).
- Config keys: of 211, 161 are declared in `@gsd/core`, 35 are owned by an extension and 15 do not apply, in
  `core/spec/settings.md`.
- Gate-cutover verdicts: 92 of the 171 are reproduced by a test or by construction (the typed commands refuse the
  argument shape); 51 belong to an extension, 14 are the gate engine, and 14 are core codes whose verdict is not reproduced: 11
  that no twin reaches, and 3 that a twin reaches and answers differently (`Diverges`, below). Each is named, with its
  reason, in `core/tests/parity_rules/cutover.rs`.
- Codes: 61 of the 63 codes fire and stay silent in a named test; W514 and E515 have no rule behind them yet.

## The four checks

Three run in the gate: `parity_roster` (what exists; a Ported module's file names it in its `//!` header),
`parity_rules` (every code fires and stays silent; the gate-cutover verdicts) and `parity_scenario` plus the goldens
(the taskboard twin answers as gsd-tools did). The fourth, `upgrade` (what a newer gsd-core changes), is a procedure,
not a gate step: `gsd/parity/upgrade.sh <gsd-core-dir> <tag>` writes
`gsd/parity/backlog/<tag>.md`.

## Verified and not verified

Verified by tests in the gate: the roster counts above, the install goldens for Claude Code, Codex and Cursor, the
taskboard goldens (19), the hook cases recorded from the upstream hook files, and the version marker (the install and render goldens hold `VERSION`; the crate and the handshake pin `1.0.0`).

Not verified:

- Codex and Cursor rendering are drafts. They are golden-tested against our own output, not run in those hosts.
- The published artefact: `specforge publish` is a user action and was not run.
- No real project of gsd-core's was run through both sides; the twin is the fixture `taskboard`.

## Known divergences (named, not fixed)

- The statusline is plain text (no colour: the kernel escapes control characters). `show_git` and
  `show_state_freshness` are inert.
- Injection scanning is advisory; `security.injection_blocking` belongs to `@gsd/security`, which a hook cannot read.
- The context monitor has no stopped-at record and no Stop or SubagentStop rows; the workflow marker lasts the session.
- `write_guard` measures bytes, not lines; `worktree_path_guard` is path based (case and relative paths are seen; a
  symlink into the main checkout is not: K19).
- `agent_isolation_guard` allows every dispatch: `@gsd/core` runs its executors one after another in the checkout, as
  upstream's sentinel `none` does, so there is no worktree isolation to demand (REVIEW-r44 C1). Its line is written for
  Cursor too and does nothing there.
- `validate_commit` lets `-F` and `--fixup` commits through (they have no message on the command line); a literal
  `fixup! ...` subject is allowed, which upstream does not do; `git commit -m $MSG` (unquoted) is blocked where
  upstream lets it through.
- Three gate-cutover verdicts are divergences: `tdd-review-checkpoint-red-only` and `tdd-review-checkpoint-mixed-violation`
  (upstream blocks a TDD plan with a RED and no GREEN, or no RED and GREEN commit; W403 judges a recorded GREEN with no
  RED before it and is silent otherwise) and `prohibition-enforcement-request-file-judgment-mode` (upstream flags a
  judgment-tier prohibition without evidence; E319 judges the test tier only).
- The 11 other unreached verdicts are named in `cutover.rs`; the 2 codes with no rule (W514, E515) in `core/README.md`.
- The "silent" evidence of 38 of the 61 built codes is the unedited twin (`rules_silent.rs`): it proves the code is not
  raised there, not that the rule judged a clean case.

## Needs a change outside `gsd/` (K19)

Each needs a crate, a gate, an ADR, a protocol change or a revendor, so none is done here:

- `specforge uninstall` leaves the empty host root directory (install crate).
- Hosts other than Claude Code, Codex and Cursor (the 16 named renderers).
- Git facts for hooks (`git add -f` blocking) need a host query the protocol lacks.
- Baseline re-verification (`re_verification.previous_*` and the baseline file view, `baseline-format` 3, ADR 0300).
- The hook facts: the real path of an out-of-project target (a symlink into the main checkout is not seen), `--cleanup`
  on a commit, and a host filter for a hook contribution (Cursor gets the isolation guard's line).
- A broken `specforge.json` makes `specforge install <host>` strip every gsd hook, agent and skill silently with exit 0
  (`specforge check` says E069): install, uninstall and `--check` should refuse with E069 and write nothing.
- `specforge uninstall` also removes what install did not create: a user's empty `settings.json`, empty `agents/` and
  `skills/` directories, `"mcpServers": {}`.
- A host without `specforge` on its PATH runs no guard (exit 127 does not block), as upstream's `node` lines do.
- A black-box twin for W514 and E515, and for the 14 verdicts above, needs rules the kernel does not have.

## What is not in the program

| Not in the program | What it would hold |
|---|---|
| `@gsd/security`, `@gsd/ai`, `@gsd/ui`, `@gsd/codebase`, `@gsd/ops` | 21 commands (ops 13, codebase 3, ai 2, ui 2, security 1), 20 agents, their capabilities, `security.injection_blocking`, the ui and codebase-drift gate-cutover verdicts, graphify |
| `@gsd/import` | the 22nd command, `/gsd-import` |
| `@gsd/path`, `@gsd/loop` | the gsd-path and gsd-loop profiles |
| The `specforged` harness | `/gsd-autonomous` unattended, `claude-orchestration`, `external-job` |
| More host renderers | opencode, copilot, windsurf, kilo, kimi, kimi-code, antigravity, cline, trae, qwen, hermes, codebuddy, augment, zcode, vscode, pi |
| Running Codex and Cursor | their goldens are the kernel's rendering, cross-checked against upstream's installer only |
| `gsd/W514`, `gsd/E515` | no rule behind them (K19) |
| The K19 items | the list above |
| Publishing | `specforge publish`, done by the user |

## Publishing

```sh
specforge publish --help
specforge publish gsd/target/wasm32-wasip2/release/gsd_core.wasm
```

The second line is the user's call: it needs the network and an account, and nothing here runs it.

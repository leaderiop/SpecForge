# Acceptance: `@gsd/core` 1.1.0 against gsd-core 1.19.0

This is the checklist for the core program (r36 to r44). Each row counts the items of one family of the vendored
inventory (`parity/upstream/v1.19.0/`): how many `@gsd/core` builds, how many another `@gsd/*` extension owns, how many
do not apply to SpecForge, and how many are named and not built. `core/tests/parity_roster.rs`
(`the_acceptance_counts_match`) computes every number from the tables the checks already hold and fails when this file
disagrees. Every row but Capabilities is computed from tables of code; Capabilities is parsed from the capability table of
`core/README.md`, so that table is the source of its numbers.

| Family | Total | Built | Owned later | Not applicable | Named open | Where |
|---|---|---|---|---|---|---|
| Commands | 72 | 50 | 22 | 0 | 0 | `core/tests/common/roster.rs` |
| Agents | 64 | 25 | 39 | 0 | 0 | `core/src/agents/roster.rs` |
| Hooks | 29 | 16 | 0 | 11 | 2 | `core/src/hooks/roster.rs` |
| Capabilities | 45 | 17 | 10 | 0 | 18 | `core/README.md` capability table |
| Workflows and references | 297 | 234 | 53 | 10 | 0 | `core/tests/parity_roster/assets.rs` |
| CLI modules | 288 | 170 | 21 | 97 | 0 | `core/tests/parity_roster/modules.rs` |
| Config keys | 212 | 161 | 35 | 16 | 0 | `core/spec/settings.md` |
| Gate-cutover verdicts | 171 | 92 | 51 | 14 | 14 | `core/tests/parity_rules/cutover.rs` |
| Codes | 63 | 63 | 0 | 0 | 0 | `core/tests/parity_rules/evidence.rs` |

Concepts (the model: phases, plans, milestones, requirements, decisions, UAT, verification) are the kinds of
`core/spec/*.md`; they have no inventory family, so no count. They are exercised by every row above.

## Status by family

- Commands: all 72 are accounted. The 50 core commands are `specforge gsd <command>` verbs or prompts; the rest belong
  to the other extensions. The `parity_roster` check fails on a command the snapshot has and the roster lacks.
- Agents: 25 of the 64 items (the core agents and their compact forms) are embedded and installed; the other 39 belong
  to `@gsd/ai`, `@gsd/ui`, `@gsd/codebase` and `@gsd/ops`.
- Hooks: 16 of the 29 upstream hook items are built (13 files ported as 15 hooks of `@gsd/core`, and the 3 Cursor
  renderings of events they port); 11 are dropped, each with its reason in `hooks/roster.rs` (8 not ported, among them
  the agent isolation guard and the path export, and the
  Cursor `stop`, `subagent-start` and `subagent-stop` files, which no hook of ours feeds); the 2 Windsurf files are
  named and open (Windsurf is a later host). `core.spec_guard` is the kernel's and is
  wired by every install.
- Capabilities: 17 built (14 in `@gsd/core`, 3 as install renderers), 10 owned by an extension, 18 named and open: 16
  renderers for hosts SpecForge does not install for yet, and `claude-orchestration` and `external-job`, which need the
  `specforged` harness (only their settings keys exist).
- Workflows and references: 234 are embedded or replaced by a typed command, 53 are owned, 10 do not apply; each of the
  78 not embedded names why in `assets.rs`.
- CLI modules: 170 are ported or are the host half of a `specforge` command, 21 are owned, 97 do not apply (the argv
  router, the capability loader, the runtimes SpecForge does not install for, `security.cjs`, the per-repository release
  tags).
- Config keys: of 212, 161 are declared in `@gsd/core`, 35 are owned by an extension and 16 do not apply, in
  `core/spec/settings.md`.
- Gate-cutover verdicts: 92 of the 171 are reproduced by a test or by construction (the typed commands refuse the
  argument shape); 51 belong to an extension, 14 are the gate engine, and 14 are core codes whose verdict is not reproduced: 11
  that no twin reaches, and 3 that a twin reaches and answers differently (`Diverges`, below). Each is named, with its
  reason, in `core/tests/parity_rules/cutover.rs`.
- Codes: 63 of the 63 codes fire and stay silent in a named test.

## `@gsd/security` 1.0.0 against gsd-core 1.19.0

Plan 16. The same families for the security items of the vendored inventory; `security/tests/parity_roster.rs`
(`the_acceptance_counts_match`) computes every number from its tables and `security/tests/parity_rules.rs` checks the
codes and the divergence tags below. The first table above counts `@gsd/core` and counts these items as owned later.

| Security family | Total | Built | Owned elsewhere | Not applicable | Named open | Where |
|---|---|---|---|---|---|---|
| Commands | 1 | 1 | 0 | 0 | 0 | `security/src/prompts.rs` |
| Agents | 2 | 2 | 0 | 0 | 0 | `security/src/agents.rs` |
| Workflows and references | 2 | 1 | 1 | 0 | 0 | `security/tests/parity_roster.rs` |
| CLI modules | 1 | 0 | 0 | 1 | 0 | `security/tests/parity_roster.rs` |
| Capabilities | 1 | 1 | 0 | 0 | 0 | `security/spec/README.md` |
| Hooks | 3 | 3 | 0 | 0 | 0 | `security/src/commands/hooks.rs` |
| Config keys | 4 | 4 | 0 | 0 | 0 | `security/src/settings.rs` |
| Codes | 2 | 2 | 0 | 0 | 0 | `security/tests/parity/evidence.rs` |

Suites: 74 cases are accounted, 67 of `secure-phase.test.cjs`, 5 of `injection-blocking-config.test.cjs` (#1577) and 2 of
`secure-phase-single-writer.test.cjs` (`security/tests/accounting.rs`, `security/tests/port.rs`). The suites and files of
the upstream tree about the node CLI's hardening library, its CI scans and its documentation are not applicable, each
with a reason in `parity_roster.rs`. Gate-cutover verdicts of security: 0.

## The four checks

Three run in the gate: `parity_roster` (what exists; a Ported module's file names it in its `//!` header),
`parity_rules` (every code fires and stays silent; the gate-cutover verdicts) and `parity_scenario` plus the goldens
(the taskboard twin answers as gsd-tools did). The fourth, `upgrade` (what a newer gsd-core changes), is a procedure,
not a gate step: `gsd/parity/upgrade.sh <gsd-core-dir> <tag> [<from-tag>]` writes
`gsd/parity/backlog/<tag>.md`. `backlog/v1.19.0.md` holds the 173 changes of 1.19.0 against 1.18.0, none unowned, and
what became of each.

## Verified and not verified

Verified by tests in the gate: the roster counts above, the install goldens for Claude Code, Codex and Cursor, the
taskboard goldens (19), the hook cases recorded from the upstream hook files, and the version marker (the install and render goldens hold `VERSION`; the handshake carries the crate's version, `1.1.0`). The taskboard goldens, the security vault goldens (17) and the secret-read guard's hook cases are recorded on 1.19.0 (`backlog/v1.19.0.md`, "Re-recorded on 1.19.0"); the three injection recordings are 1.18.0's, which 1.19.0 did not change.

Not verified:

- Codex and Cursor rendering are drafts. They are golden-tested against our own output, not run in those hosts.
- The published artefact: `specforge publish` is a user action and was not run.
- No real project of gsd-core's was run through both sides; the twin is the fixture `taskboard`.
- The commands the program docs name were checked with `--help` only (`publish` needs the network), not each on a
  temp project.
- The "ported" lists of every hook suite are written by hand next to their assertions (`port(...)` after each one);
  `accounting::account` checks each list against the vendored suite's titles, both ways. An accounted skip carries one
  of D-r44-2's six reasons or `diverges: [tag]`, the one amendment, for a difference listed under "Known divergences"
  by the same `[tag]`; the accounting fails on a tag that section does not hold.
- `hooks-opt-in.test.cjs` (80 cases) is vendored and accounted: a case is ported when it decides a `validate_commit` verdict,
  and skipped with a reason when it tests the installer, the opt-in, bash's word handling, or one of two divergences.
- Known limits of the checks (r44 post-fix minors): `secrets.rs`'s 1 MiB limit counts bytes, not UTF-16 units, and errs
  toward refusing; the Codex and Cursor agents and skills (130 files) are pinned by `{bytes, sha256}` in `tree.json`
  only, with no full-text golden read by a person (a drift is caught, a wrong first blessing is not);
  `config_keys_match_the_snapshot` checks a count, and the membership of the 212 keys is accounted by
  `settings_roster.rs`; a refused install can still write `.mcp.json` (claude) or `hooks.json` (codex).
- A `statusline.rs.bak` was committed in `0ba3b8c0b` and removed in `d665cd81b`; it is absent at HEAD and the history
  is not rewritten.

## Known divergences (named, not fixed)

- The statusline is plain text (no colour: the kernel escapes control characters). `show_git` shows the branch and the work-state markers (plain) after the directory;
  `show_state_freshness` is inert (there is no STATE.md stamp). `[update-banner]` The update banner is dropped (D-r44-13): specforge updates
  extensions through its lockfile. `[statusline-metrics]` Upstream's context monitor reads metrics the statusline
  writes to a bridge file; here the statusline computes them and the meter's arithmetic is tested with the statusline.
- Injection scanning answers a block for a HIGH scan only when `@gsd/security` is loaded and sets
  `security.injection_blocking`; without it the scan is advisory. The block never stops the read:
  on Claude Code it lands after the tool ran, as upstream's does, and Cursor cannot deny after a tool runs, so there it
  is advisory context (`[injection-after-read]`).
- `@gsd/security` (plan 16), by tag:
  - `[blocking-needs-security]` the key `security.injection_blocking` belongs to `@gsd/security`; upstream lets any project set it.
  - `[fragment-repointed]` the planner's threat-model fragment names the kinds `trust_boundary` and `threat`, not a `<threat_model>` block of a PLAN.md.
  - `[audit-advisory-every-check]` `gsd.security/W301` fires on every `check`, not once at close (core has no close verb).
  - `[accept-risk-human]` `accept_risk` is run by the person at a terminal (ADR 0301). `[accepted-risk-not-human-kind]`
    `accepted_risk` is not a human-only kind: the host refuses a human command's plan when it would create a
    human-only entity. Its `at` is a harness field instead, set only by `accept_risk` with its ledger line, and only a risk
    with an `at` counts: an agent's `specforge new accepted_risk` is refused `harness_field` with `--at` and accepts
    nothing without it, so the ship gate stays shut (REVIEW-security-part2 M1). The limit left is a hand edit of the
    `.spec` file plus a forged ledger line (the ledger digest has no secret, so it is an integrity check, not
    authentication). A candidate kernel change, narrower than a new kind: a `human` command's plan may write a
    human-only kind (`agent_facing` treats every command's plan as an agent's, `flags.rs`).
  - `[gate-parity-plus]` `security_gate` blocks more than upstream's gate: an audit that never ran (status `draft`, so
    not `verified`) blocks `SECURITY_SHIP_GATE_OPEN_THREATS` where upstream passes a `SECURITY.md` whose frontmatter says
    `threats_open: 0`, and a security model that does not load or check clean (an invalid enum, a reference that does not
    resolve, fewer threats than the last audit run counted) blocks `SECURITY_SHIP_GATE_UNREADABLE` with the reasons in
    `details.problems`. A `.spec` file that does not parse is seen only through what it takes with it; `specforge check`
    reports it (REVIEW-security-part2 m3 and the fail-closed contract).
  - `[audit-counts-per-entity]` an audit run's `total`, `closed` and `open` count threat entities: the supply-chain row
    `T-NN-SC`, which every plan keeps, is counted once per plan, and so is a duplicated ID, where upstream's register holds
    the supply-chain row once per phase (the vault twin: 6 against upstream's 3). The decision to append a run is the same
    (REVIEW-security-part2 N4).
  - `[no-superseded-plans]` no superseded-plan exclusion in the duplicate scan: a superseded plan is removed in the core model.
  - `[stride-enum]` the STRIDE category is an enum of six words, not free text; the importer maps letters.
  - `[retro-threat-in-plan]` a retroactive threat is written into a plan of the phase with `retroactive: true`, not held in SECURITY.md only.
  - `[threats-open-live]` `threats_open` is computed from the threats and the current `block_on`, not frozen at audit time.
  - `[security-absent-off]` without `@gsd/security` loaded there is no threat model, where upstream's agents read an absent `security_enforcement` as enabled.
  - `[verify-post-produces]` the `verify:post` step keeps upstream's `produces: ["SECURITY.md"]`, although plan 16's parity
    table said `security_audit`: the hook text is upstream's and agents read the name from it. `SECURITY.md` there
    stands for the phase's `security_audit`: `security-hooks verify:post --after-fingerprint <phase>` moves the step to
    `skippedHooks` (`reason: produces-present`) once the audit exists, as upstream's `producesEntryPresent` does for the
    file (#5105), so verify-work and autonomous do not run the auditor a second time. Without the flag `skippedHooks` is
    `[]` where upstream omits the key; core's jq fold absorbs both (REVIEW-security-part2 M3).
  - `[injection-key-explicit]` the vault fixture sets `security.injection_blocking: false` explicitly, because gsd-tools' `config-get`
    errors on an unset key where `specforge settings` answers the default: a recording choice, not a product divergence.
  - `[surface-peer-setting]` the slash `/gsd-secure-phase` is hidden by `@gsd/security`'s own `specforge.surface.audit_review`
    (a slash's `when` must be a setting of its own extension, W021), which follows `@gsd/core`'s setting of the same name
    through a preset when it is `false` (`/gsd-surface` writes core's). The two can be set apart by hand
    (REVIEW-security-part2 m5).
  - `[response-language-own]` `/gsd-secure-phase` reads `@gsd/security`'s own `response_language`, not `@gsd/core`'s: a
    prompt sees only its own extension's settings, and a preset follows only an enum or a bool (W021), so a person sets
    the language under both extensions. `workflow.text_mode` follows core's through a preset (REVIEW-security-part2 m1).
- The context monitor has no stopped-at record (`[stopped-at]`: /gsd-pause-work records it) and no Stop or SubagentStop rows; the workflow marker lasts the session.
- `[write-guard-bytes]` `write_guard` measures bytes, not lines; `worktree_path_guard` compares git toplevels as upstream does, so a symlink into
  the main checkout is refused (ADR 0320).
- `[stricter-relative-path]` `worktree_path_guard` also refuses a relative path that climbs out of the worktree into the
  main checkout (`../../../src/a.ts`), which upstream allows (its guard lets every relative `file_path` through); this
  is deliberate, for the bug the guard exists for (#260), and stricter than upstream. (REVIEW-r44 M4 had said upstream
  denies it; it does not.)
- `[no-isolation-guard]` There is no `agent_isolation_guard`: `@gsd/core` runs its executors one after another in the
  checkout, as upstream's sentinel `none` does, so there is no worktree isolation to demand and the guard would always
  allow (REVIEW-r44 C1). Not installing it saves every `Agent`/`Task` dispatch a process; `gsd-agent-isolation-guard.js`
  is dropped in the roster, until the `specforged` harness runs executors in worktrees.
- `[no-sentinel]` The write, read and workflow guards have no override environment variable and no sentinel file
  (D-r44-7).
- `validate_commit` lets `-F` and `--fixup` commits through (they have no message on the command line); a literal
  `fixup! ...` subject is allowed, which upstream does not do; `git commit -m $MSG` (unquoted) is blocked where
  upstream lets it through (`[unquoted-m]`, which also covers `-m "" -m "fix: x"`). `[dquoted-heredoc-tag]` A `<<"TAG"`
  heredoc resolves, where upstream's capture cannot deliver it and blocks it. `[truncated-heredoc]` A heredoc message with no terminator line is judged by its opener (upstream's capture can
  hand a cut body whose first line is complete, and resolves it).
- `[cleanup-exact]` `validate_commit` reads the cleanup directives of the commit's own words, not of the whole
  command line (`... && echo --cleanup=verbatim` resolves, which upstream refuses). Refusing `--cleanup=default` is
  upstream's own rule (any single mode but `whitespace` refuses), a change from the old gsd behaviour (D-K20c-3).
- `[cwd-branch]` `workflow_guard` judges a `git -C <dir> add -f` by the branch of the hook's cwd, not of `<dir>` (upstream
  probes the target); upgrade when the host can be asked another directory's branch (ADR 0320).
- The six commands that record what a person says (`answer_checkpoint`, `confirm_setup`, `set_uat_result`,
  `approve_validation`, `waive_window`, `dispose_refactor`) are run by the person at a terminal and refused to an agent
  (ADR 0301); upstream's agent transcribes the person's reply. `pause` stays an agent's. An automated UAT result
  (upstream's `auto_passed`, recorded without asking the person) is the agent's: `record_automated_uat` writes
  `source: automated`, and `set_uat_result --source automated` is refused with a hint naming it (REVIEW-k20a M1).
- Limit of the human-attested commands: the proof is that stdin is a terminal, and the kernel's `core.human_guard`
  (a deny-policy PreToolUse hook on the shell tool, installed on every host while a command is `human`) refuses a shell
  call that runs one (whole line, any case, `.exe`, `ext:command`, behind wrappers with options, keywords, braces,
  `$(…)`, `-c` and `eval`; an unreadable line that names specforge is refused), so an agent shell that is itself a pty
  (Cursor's agent terminal, Codex's pty exec) is stopped there. What is left is `script(1)` typed by a person on the
  agent's behalf, a script file or a variable that holds the command, and a host with no shell hook (REVIEW-k20a M2,
  REVIEW-k20c-part2 M1).
- `/gsd-phase --remove` of a phase whose truth holds a `verification` tier is refused `human_only`, for an agent and for
  a person's run of the command alike: the tier is a person's (`U`, D-K20a-4), so the person deletes that line in their
  editor and runs the remove again. Upstream deletes the phase regardless (REVIEW-k20a M3). A human-attested remove is
  not built.
- W514 fires on every `check`, once per open deviation window that names a proposal, while `workflow.windows_enforce` is
  off; upstream warns once per triggering `refactor evaluate` (also when broken-windows is absent). The text and the
  remediation match. E515 is SpecForge's own (gsd-plus) and has no upstream.
- A threshold's bounds are whole numbers (D-K20a-9): a warning between 0 and 1 and a critical between 99 and 100 are
  refused (`hooks.context_critical_threshold` refuses `99.5`), which GSD accepts.
- Three gate-cutover verdicts are divergences: `tdd-review-checkpoint-red-only` and `tdd-review-checkpoint-mixed-violation`
  (upstream blocks a TDD plan with a RED and no GREEN, or no RED and GREEN commit; W403 judges a recorded GREEN with no
  RED before it and is silent otherwise) and `prohibition-enforcement-request-file-judgment-mode` (upstream flags a
  judgment-tier prohibition without evidence; E319 judges the test tier only).
- The 11 other unreached verdicts are named in `cutover.rs`.
- The "silent" evidence of 38 of the 63 built codes is the unedited twin (`rules_silent.rs`): it proves the code is not
  raised there, not that the rule judged a clean case.

## Needs a change outside `gsd/`

Each needs a crate, a gate, an ADR, a protocol change or a revendor, so none is done here:

- Hosts other than Claude Code, Codex and Cursor (the 16 named renderers).
- A black-box twin for the 14 verdicts above needs rules the kernel does not have.

## What is not in the program

| Not in the program | What it would hold |
|---|---|
| `@gsd/ai`, `@gsd/ui`, `@gsd/codebase`, `@gsd/ops` | 20 commands (ops 13, codebase 3, ai 2, ui 2), 18 agents, their capabilities, the ui and codebase-drift gate-cutover verdicts, graphify |
| `@gsd/import` | the 22nd command, `/gsd-import` |
| `@gsd/path`, `@gsd/loop` | the gsd-path and gsd-loop profiles |
| The `specforged` harness | `/gsd-autonomous` unattended, `claude-orchestration`, `external-job` |
| More host renderers | opencode, copilot, windsurf, kilo, kimi, kimi-code, antigravity, cline, trae, qwen, hermes, codebuddy, augment, zcode, vscode, pi |
| Running Codex and Cursor | their goldens are the kernel's rendering, cross-checked against upstream's installer only |
| Publishing | `specforge publish`, done by the user |

## Publishing

```sh
specforge publish --help
specforge publish gsd/target/wasm32-wasip2/release/gsd_core.wasm
```

The second line is the user's call: it needs the network and an account, and nothing here runs it.

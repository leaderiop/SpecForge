# Acceptance: `@gsd/core` 1.0.0 against gsd-core 1.18.0

This is the checklist for the core program (r36 to r44). Each row counts the items of one family of the vendored
inventory (`parity/upstream/v1.18.0/`): how many `@gsd/core` builds, how many another `@gsd/*` extension owns, how many
do not apply to SpecForge, and how many are named and not built. `core/tests/parity_roster.rs`
(`the_acceptance_counts_match`) computes every number from the tables the checks already hold and fails when this file
disagrees; a row cannot drift from the code.

| Family | Total | Built | Owned later | Not applicable | Named open | Where |
|---|---|---|---|---|---|---|
| Commands | 72 | 50 | 22 | 0 | 0 | `core/src/prompts/roster.rs` |
| Agents | 64 | 25 | 39 | 0 | 0 | `core/src/agents/roster.rs` |
| Hooks | 28 | 21 | 0 | 7 | 0 | `core/src/hooks/roster.rs` |
| Capabilities | 45 | 19 | 10 | 0 | 16 | `core/README.md` capability table |
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
- Hooks: 21 of the 28 upstream hook items are built (15 hooks of `@gsd/core`, and the Cursor and Windsurf renderings of
  events they port); 7 are dropped, each with its reason in `hooks/roster.rs`. `core.spec_guard` is the kernel's and is
  wired by every install.
- Capabilities: 19 built (16 in `@gsd/core`, 3 as install renderers), 10 owned by an extension, 16 named renderers
  for hosts SpecForge does not install for yet.
- Workflows and references: 233 are embedded or replaced by a typed command, 53 are owned, 9 do not apply; each of the
  77 not embedded names why in `assets.rs`.
- CLI modules: 168 are ported or are the host half of a `specforge` command, 22 are owned, 92 do not apply (the argv
  router, the capability loader, the runtimes SpecForge does not install for).
- Config keys: of 211, 161 are declared in `@gsd/core`, 35 are owned by an extension and 15 do not apply, in
  `core/spec/settings.md`.
- Gate-cutover verdicts: 92 of the 171 are reproduced by a test or by construction (the typed commands refuse the
  argument shape); 51 belong to an extension, 14 are the gate engine, 14 are core codes whose content verdict no twin
  reaches (each is named, with its reason, in `core/tests/parity_rules/cutover.rs`).
- Codes: 61 of the 63 codes fire and stay silent in a named test; W514 and E515 have no rule behind them yet.

## The four checks, in the gate

`parity_roster` (what exists), `parity_rules` (every code fires and stays silent; the gate-cutover verdicts),
`parity_scenario` plus the goldens (the taskboard twin answers as gsd-tools did) and `upgrade` (what a newer gsd-core
changes). The upgrade check is a procedure, not a gate step: `gsd/parity/upgrade.sh <gsd-core-dir> <tag>` writes
`gsd/parity/backlog/<tag>.md`.

## Verified and not verified

Verified by tests in the gate: the roster counts above, the install goldens for Claude Code, Codex and Cursor, the
taskboard goldens (19), the hook cases recorded from the upstream hook files, and the version `1.0.0` in every marker.

Not verified:

- Codex and Cursor rendering are drafts. They are golden-tested against our own output, not run in those hosts.
- The published artefact: `specforge publish` is a user action and was not run.
- No real project of gsd-core's was run through both sides; the twin is the fixture `taskboard`.

## Known divergences (named, not fixed)

- The statusline is plain text (no colour: the kernel escapes control characters). `show_git` and
  `show_state_freshness` are inert.
- Injection scanning is advisory; `security.injection_blocking` belongs to `@gsd/security`, which a hook cannot read.
- The context monitor has no stopped-at record and no Stop or SubagentStop rows; the workflow marker lasts the session.
- `write_guard` measures bytes, not lines; `worktree_path_guard` is path based; `agent_isolation_guard` is Claude only.
- `validate_commit` lets `-F` and `--fixup` commits through (they have no message on the command line).
- The 14 gate-cutover verdicts no twin reaches are named in `cutover.rs`; the 2 codes with no rule (W514, E515) in
  `core/README.md`.

## Needs a change outside `gsd/` (K19)

Each needs a crate, a gate, an ADR, a protocol change or a revendor, so none is done here:

- `specforge uninstall` leaves the empty host root directory (install crate).
- Hosts other than Claude Code, Codex and Cursor (the 16 named renderers).
- Git facts for hooks (`git add -f` blocking) need a host query the protocol lacks.
- A black-box twin for W514 and E515, and for the 14 verdicts above, needs rules the kernel does not have.

## Publishing

```sh
specforge publish --help
specforge publish gsd/target/wasm32-wasip2/release/gsd_core.wasm
```

The second line is the user's call: it needs the network and an account, and nothing here runs it.

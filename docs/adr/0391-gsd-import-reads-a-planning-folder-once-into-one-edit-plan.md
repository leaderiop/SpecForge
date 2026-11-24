# `specforge gsd import` reads a `.planning/` folder once into one edit plan

**Status:** accepted (2026-11-24, round r6-breadth, Phase 13). **Builds on**
[ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (an extension writes through one edit
plan, applied all or nothing), [ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) (a command
declares the project files it reads, by static globs), [ADR 0287](0287-who-writes-a-field-is-declared.md) (who writes a
field), [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) and
[ADR 0377](0377-a-verdict-is-fresh-over-what-it-cites-and-owed-by-its-owner-state.md) (verdicts, their origin, what
owes one), [ADR 0301](0301-a-human-attested-command-runs-only-for-a-person-at-a-terminal.md) (human commands),
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) and
[ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D3 (vendored builtins, the
size rule), [ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) (`@gsd/core` holds exactly
gsd-core's kinds), [ADR 0372](0372-the-converter-gives-every-old-kind-one-fate-and-writes-gsds-ids.md) (the fate
table, the ledger, GSD's ids) and
[ADR 0390](0390-a-milestone-computes-its-api-breaks-and-release-evidence-and-two-guards-read-them.md) D9 (the twins
are measured by the harness's `codes()`).

## Context

- A project that ran upstream GSD holds its state in Markdown under `.planning/`: `PROJECT.md`, `REQUIREMENTS.md`,
  `ROADMAP.md`, `STATE.md`, and per phase its context, research, plans, summaries, verification, UAT and security
  files. `@gsd/core` holds the same state as `.spec` entities. Requirement IMP-01 asks for one command that moves a
  `.planning/` folder into `spec/gsd/` and names everything it could not move; IMP-02 measures it on the three parity
  fixtures (taskboard, ledger, vault) against their hand-written twins; IMP-03 makes it its own crate and blob.
- The owner settled the shape on 2026-11-24 (Phase 13's context): one command, no positional, the project root's
  `.planning/**`, one all-or-nothing edit plan plus a report, no mirror, verdicts never imported, re-import refused,
  the fixtures not edited.
- The host already enforces most of what an importer could get wrong: who writes a field (computed, context, harness,
  human, verdict), a verdict's origin, the revision of a new file, and the errors a plan may add (E080). The importer
  mirrors those refusals and reports them instead of meeting them as a refused plan.

This ADR is recorded before any `@gsd/import` code. Every later plan of the phase cites its D-numbers.

## D1. One way, once

`specforge gsd import` reads the project root's `.planning/**` once and answers ONE edit plan that writes `spec/gsd/`,
plus a report. There is no Markdown mirror and no later Markdown read: after the import, `.spec` is the only state, and
`.planning/` is the user's to keep or delete. The command never writes under `.planning/`.

## D2. The command

- **Name.** `specforge gsd import`, with no positional and no `--dir`. A file glob is declared at contribution time
  (ADR 0293), so a directory chosen at run time cannot become a read, and the SDK offers no optional positional (a
  non-required argument is a `--name` flag). The host's `--path` picks the project; `--format json` and `--dry-run` are
  the host's own.
- **Inputs.** Two static globs: `.planning/**` (text, up to the 4 MiB `TEXT_MAX_CEILING_BYTES` ceiling) and
  `specforge.json` (text).
- **Contribution.** `@gsd/import` contributes this one command and no kind. Its peers are `@gsd/core` (required) and
  `@gsd/security` (optional).

## D3. The fate table

ADR 0372's pattern, applied to files instead of old kinds. Every name of `gsd_kit::files::PLANNING_FILES`, plus
`config.json`, `state.json` and `.gitkeep`, and every known section, key and table row of the Tier 1 formats has
exactly one outcome among `mapped`, `computed`, `redundant`, `runtime`, `ignored`, `unmapped`, `needs-extension`,
`setting`, `authored-after-import` and `filled`. A ledger test counts it: a name or a section with no row, or with two,
fails the test.

- **Tier 1 (mapped).** `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md` (the flat form and the milestone-grouped one),
  `NN-CONTEXT.md`, `NN-DISCUSSION-LOG.md`, `NN-RESEARCH.md`, `NN-MM-PLAN.md`, `NN-MM-SUMMARY.md`,
  `NN-VERIFICATION.md`, `NN-UAT.md`, `NN-SECURITY.md` and the plans' threat models (both with `@gsd/security`, D11),
  and the `quick/*` plans and summaries.
- **Every other name** is `unmapped`, with the kit's own kinds or reason text, so the importer and the kit cannot
  disagree about a file.
- **Tier 2 sections inside Tier 1 files** (`key-decisions`, `next_readiness`, deviations, checkpoints, `prohibitions`,
  `user_setup`) are reported, not mapped, in this phase.
- **A section whose kit kind was removed** in R4 (persona, journey, term, risk) is reported "no target (ADR 0370)".
- `.gitkeep` is `ignored`; `state.json` is `runtime` (gsd-tools' derived state, which `STATE.md` and the graph compute).

## D4. Ids and files

- **Ids** are GSD's, as ADR 0372 D3 writes them: a requirement keeps the bold id written (`TASK-01`); an out-of-scope
  row, which carries none, is `OOS-<NN>` in row order; a project decision is `D-<NN>` in its table's row order; a phase
  is its integer part on two digits with a decimal kept (`02.1`); a plan is `<NN>-<MM>` from its file name; a quick
  plan is its `quick_id`; tasks, truths, artifacts and key links are `T<n>`, `TR<n>`, `AR<n>`, `KL<n>` in document
  order; summaries, contexts, discussions, research, verifications, UAT sessions and security audits are their file
  stems; threats keep their Threat ID cell. The milestone is `v1.0` when the roadmap names none, recorded `filled`.
- **Validation.** Every id is validated against the identifier grammar the host's create accepts before it is used. A
  source value that fails it is reported at its `file:line` and the entity is not created.
- **Files.** Every target file path is built only from validated ids, under `spec/gsd/`. A spec root other than `spec`
  (read from `specforge.json`) is refused with a message: the twins and the fixtures use `spec`, and the spec root is
  no input a command is given.

## D5. Verdicts are never imported

- **The host's rule.** The host accepts a `set_verdict` only from a command of the rule's own extension, and only on
  an entity already in the compiled graph (`only a command of {extension} writes a verdict for the rule '{rule}'`,
  `crates/app/ops/src/authoring/apply/verdict.rs`). `@gsd/import` is not `@gsd/core`, and the entities it creates are
  not in the graph its plan is checked against. No plan `@gsd/import` answers holds a verdict operation.
- **What follows.** Executed plans owe `gsd/E501` on each truth, artifact and key link after an import, so
  `specforge check` exits 1 until the verifier runs. That is the expected result, not a failure of the import. E516
  fires at a gate only, never in `check`.
- **The report.** It lists each verdict a `.planning` file recorded (a `passed` verification, a score) as owed, and
  prints, per phase, `/gsd-verify-work <NN>`, and per owed entity the runnable
  `specforge query '<entity>' --inputs gsd/E501 --format json` whose `basis` `specforge gsd record-verdict` takes, as
  the verifier records it (`gsd/core/assets/agents/gsd-verifier.md`). No basis can be printed ahead: a basis is a
  digest of today's cited files, and a legacy `passed` is no judgement of today's code.

## D6. Settings

- `config.json` is never written: the plan writes no `specforge.json`.
- The report prints one `specforge settings set <key> <value>` line per mappable key, each value POSIX single-quoted
  (`'` written `'\''`), because the line is meant to be pasted into a shell and the input is untrusted.
- A value holding a control character is reported, not printed.
- A secret string (the search providers' API keys) is reported with its environment variable's name only.
- A key of an extension the project does not load is `needs-extension`.
- `worktree.baseRef`, which GSD keeps outside `.planning/`, is reported, not read.

## D7. Harness fields

- **What is written.** `@gsd/import` writes the harness fields it has a `.planning` source for: `uat_test.result`,
  `threat.status`, `security_audit.asvs_level` and `created`, `accepted_risk.at`, and an `audit_run`'s counts and
  `at`. It writes them as a declared peer of their extension: the host lets a command of an extension whose handshake
  names the declaring extension a peer set its harness fields (`may_write_harness` in
  `crates/app/ops/src/authoring/apply/flags.rs`, which reads every peer, required or optional), and the host writes
  each `specforge-harness` ledger line.
- **Filled and reported.** A date-only `audit_run.at` is filled as `<date>T00:00:00Z` and reported `filled`.
  `summary.started` and `summary.completed` are reported only.
- **Never written.** Computed, context, human and verdict fields. A plan that sets one is refused whole by the host.
- **Not `.human()`.** `@gsd/core`'s `set_uat_result` is `.human()` (ADR 0301): it records a person's answer as they
  give it. The import command is not: it copies what a person already recorded in `.planning/`, and an agent may run an
  adoption. A UAT result the import writes is the earlier answer, carried, not a new one.
- **The premise.** D7 rests on the host accepting a peer's harness write. Phase 13's spike tests it before any importer
  code depends on it. Were it refused, D7 is amended before any importer code writes a harness field: the values would
  then be reported like verdicts, each with the command that records it.

## D8. The report

- File- and section-level: one entry per line, with `file:line`, the section, the outcome and the reason.
- Every echoed string is escaped: C0, C1, DEL and the Unicode bidi and format controls are written `\u{..}`. Each
  echoed value is capped.
- Deterministic: sorted, no clock, no absolute path.
- No content of an `unmapped` file is echoed: its name and reason only.
- `--format json` gives the same entries as JSON, beside the plan, in the host's `{output, plan, files_written}`.

## D9. The guest validates first, and caps

- **Mirror the host.** The guest refuses what the host would refuse (two entities with one id, an id the grammar
  refuses, a reference to nothing, a write the writer flags forbid) and reports it before answering, because a host
  E080 refusal drops the JSON report and leaves only the diagnostic.
- **Caps.** Plan operations and report entries are capped. A cap reached, or a host read that stopped early or failed
  on a mapped file, refuses the plan with "truncated at N" rather than write a partial project.
- **Dropped, never stubbed.** A record missing a required field that no `filled` rule fills is dropped and reported;
  its children are reported "parent not imported", never created with a dangling reference (ADR 0372 D4).
- **What the guest cannot see is the host's.** The host refuses a plan that adds any unjudged error (E080, `added` in
  `crates/app/ops/src/authoring/edit/validate.rs`). A path field naming a file that no longer exists (E016 on a complete
  plan's `files_modified`, a done task's `files`, any `context_refs`) therefore refuses the whole import, naming each
  path; the user corrects that path in `.planning/` and runs the import again. A static glob cannot list the project
  tree, so the guest does not guess which paths exist.

## D10. Re-import is refused

The command reads its graph and refuses when any `@gsd/core` entity exists, naming one (its kind, path and file). There
is no `--force`: to start over the user deletes those entities (by default `spec/gsd/`). Two imports run at once
cannot both land: the second plan's new files carry revision "absent", and the host's revision check refuses it.

## D11. `@gsd/security`

Threats, trust boundaries, the security audit, accepted risks and audit runs map only when `specforge.json` lists
`@gsd/security`. That is a text check of the file, not a registry query, and the report says so. Otherwise each such
file and each plan's threat model is `needs-extension`.

## D12. Packaging (IMP-03)

- Crate `gsd/import`, a named `BUILTINS` row in `Family::Gsd`, and its own blob under the 2,500,000 B per-blob cap.
- No `gsd-core` dependency: it would export a second component's handlers and link `@gsd/core`'s kinds, passes and
  prompts into the blob. `gsd-kit` is the shared crate, as it is for `@gsd/security`.
- `@gsd/core` re-vendors in the same single re-vendor, because the shared `gsd/Cargo.lock` is hashed into its record,
  and stays under its 5,800,000 B allowance (ADR 0366 D3).
- `TOTAL_BUDGET` (`crates/runtime/component/tests/blob_size.rs`) is re-measured in the blob's commit, by ADR 0373 D3.

## D13. The measure (IMP-02)

- **Codes.** Each of taskboard, ledger and vault imports to a project whose harness `codes()` equal a set pinned per
  fixture under `gsd-compat`, `gsd-strict` and `gsd-plus`: the twin's set (ADR 0390 D9), adjusted only by listed,
  reasoned differences. The one known today: the vault yields 2 `gsd.security/W302`, not the twin's 5, because the
  twin's closed `01-01/T-01-02` and both `T-01-SC` rows have no `.planning` source, and the importer does not invent a
  status.
- **Entities.** The imported entity set equals the twin's with computed fields, `context_pack` and verdicts excluded,
  and a two-way list of reasoned differences (a field with no `.planning` source, such as a vault threat's `files`, is
  "authored after import" on one side; a twin's invented title or id on the other). The list fails both ways: a
  difference not listed, and a listed one that no longer differs.
- **Check.** `specforge check` on an imported fixture exits 1 with exactly the owed judged codes (D5).
- The fixtures under `gsd/parity/fixtures/` are not edited: their gsd-ab pins and goldens stay.

## How this differs from ADR 0360's rejected `@gsd/import`

[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) rejected an `@gsd/import` as the way to
move this repository's own `spec/` from the old builtins to `@gsd/core`: that spec is not `.planning/` Markdown, an
importer cannot compile its old kinds, and a `gsd/` crate may not name the repository's `spec/`. That decision stands;
the one-time converter (`xtask/src/absorb/`, ADR 0372) did the move. The `@gsd/import` this ADR records is a different
thing: a user-facing, one-way reader of a user's `.planning/` folder, run in the user's project. It never reads or
writes the repository's own `spec/`, and it reads no old kind.

## Rejected

- **A `[<dir>]` positional, or `--dir`.** A glob is declared at contribution time, and the SDK has no optional
  positional (D2).
- **A YAML crate.** The frontmatter GSD writes is a small subset; a crate adds supply-chain surface, blob size and
  anchor, alias and tag handling the reader would have to refuse anyway. A bounded hand-written reader over
  `gsd_kit::markdown` reports an unsupported construct at its `file:line`.
- **A `@gsd/core` command that applies imported verdicts.** A legacy `passed` is no judgement of today's code, and the
  owner chose report-only (D5).
- **Depending on `gsd-core`.** Blob size and a double component export (D12).
- **Inventing the vault's missing threat statuses** to match the twin's five W302. The importer writes what
  `.planning/` holds (D13).
- **Writing `specforge.json`.** The report prints the settings lines instead (D6).
- **A `--force` re-import** (D10).

## Out of scope

- A reverse export to Markdown, and any mirror (D1).
- Re-reading `.planning/` after the import (D1).
- New `@gsd/core` kinds (ADR 0370).
- Reading a `.planning/` other than the project root's (D2).
- Upstream's `/gsd-import` slash command, which the roster gives `@gsd/import` later.

## Consequences

- A user adopts SpecForge on a GSD project with one command and a report that names every file, section and row it did
  not map, with a reason; nothing is dropped silently, and a test counts it (D3).
- An imported project with executed plans fails `specforge check` with owed `gsd/E501` until `/gsd-verify-work` runs,
  and the report prints the commands that clear it (D5).
- The builtins gain one blob, `gsd_import.wasm`; `TOTAL_BUDGET` and the `gsd_core` record move once, at the phase's
  single re-vendor (D12).
- The acceptance test pins each fixture's codes and entity differences, so a later change to `@gsd/core` that moves an
  imported project's codes is seen (D13).

# `@gsd/import`

**Status: built, 1.0.0, a vendored builtin** ([ADR 0391](../../docs/adr/0391-gsd-import-reads-a-planning-folder-once-into-one-edit-plan.md),
amended by [ADR 0392](../../docs/adr/0392-an-import-commands-plan-is-judged-for-host-errors-only.md)). One command,
`specforge gsd import`, reads a GSD project's `.planning/` folder once and answers one edit plan that writes
`spec/gsd/`, with a report of every file, section and row it did not map and why. One way only: after the import
`.spec` is the only state, no Markdown is read again, and nothing under `.planning/` is written. It declares no kind;
its peers are `@gsd/core` (required) and `@gsd/security` (optional).

## Running it

```sh
specforge add @gsd/core
specforge add @gsd/import
specforge add @gsd/security                    # optional: threats, trust boundaries, security audits
specforge gsd import --dry-run                 # the report, nothing written
specforge gsd import                           # the plan, applied all or nothing, and the report
specforge check                                # exits 1 until the owed verdicts are recorded (below)
```

- **No positional.** The command reads the project root's `.planning/**` (text, up to the 4 MiB read ceiling) and
  `specforge.json`, the two static globs it declares (ADR 0391 D2). The host's `--path` picks another project root.
- **`--dry-run`** prints the report and writes nothing. **`--format json`** gives the report as JSON beside the plan,
  in the host's `{output, plan, files_written}`.
- The same folder gives the same plan and the same report, byte for byte: files are read in path order, and the report
  is sorted, with no clock and no absolute path (D8).

## What it writes

One all-or-nothing edit plan under `spec/gsd/`; every file name is built from validated ids (D4):

| File | Holds |
|---|---|
| `spec/gsd/project.spec` | the project and its decisions (`PROJECT.md`) |
| `spec/gsd/requirements.spec` | the requirements, out-of-scope rows `OOS-<NN>` (`REQUIREMENTS.md`) |
| `spec/gsd/<milestone>.spec` | a milestone and its phases (`ROADMAP.md`); `v1.0` when the roadmap names none, reported `filled` |
| `spec/gsd/phases/<NN>.spec` | everything under one phase: contexts, discussions, research, plans and tasks, summaries, verifications, UAT sessions, threats and security audits |
| `spec/gsd/quick.spec` | quick plans and their summaries (`quick/*`) |

Ids are GSD's own (`TASK-01`, `D-01`, phase `02.1`, plan `01-02`, task `T1`, threat `T-01-02`). A value that is not
a valid id is reported at its `file:line` and the entity is not created. A record missing a required field is
dropped and reported, and its children are reported "parent not imported", never written with a dangling reference
(D9). The host also writes the `specforge-harness` ledger lines for the harness fields the import sets (a UAT result,
a threat's status, a security audit's level and date) and the `.gitattributes` merge line for that ledger (D7).

What the [file table](../core/spec/README.md#every-gsd-file-maps-to-a-kind) maps, the fate table
(`src/table.rs`) gives every known file, section, key and row exactly one outcome, and a test counts it (D3).
Threat models, `NN-SECURITY.md` and audit runs map only when `specforge.json` lists `@gsd/security` (a text check of
the file, which the report says); otherwise they are `needs-extension` (D11).

## The report

One line per file, section or row: `<file>:<line>  <section>  <outcome>  <reason>`. The ten outcomes are `mapped`,
`computed`, `redundant`, `runtime`, `ignored`, `unmapped`, `needs-extension`, `setting`, `authored-after-import` and
`filled`. Every echoed string is escaped (C0, C1, DEL and the bidi and format controls as `\u{..}`) and capped; an
`unmapped` file's content is never echoed, only its name and reason (D8). A title or value holding a control or bidi
character no `.spec` file can hold drops its record, reported. Then, each under its heading:

- **settings: run each line to keep the setting.** `config.json` is never written into `specforge.json` (D6). Each
  mappable key gets one `specforge settings set <extension> <key> <value>` line, the value POSIX single-quoted; a value
  holding a control character is reported, not printed. A search provider's API key is never printed: the line turns
  the provider on, and the report names the environment variable to export (`BRAVE_API_KEY`, …). A key of an
  extension the project does not load is `needs-extension`. `worktree.baseRef` lives in
  `.claude/settings.local.json`, outside `.planning/`, so it is reported, not read.
- **owed: record each verdict anew.** Verdicts are never imported (D5): `.planning/` records results, and a legacy
  `passed` is no judgement of today's code. Every imported entity a judged rule of `@gsd/core` or `@gsd/security`
  judges is listed, grouped by phase under the slash command that judges it (`/gsd-verify-work <NN>`,
  `/gsd-secure-phase <NN>`, `/gsd-plan-phase <NN>`, `/gsd-spec-phase <NN>`, `/gsd-audit-milestone`), each followed by
  `specforge query '<entity>' --inputs <code> --format json`, whose `basis` the recording command takes. The count line
  says whether `specforge check` exits 1 (an error rule such as `gsd/E501` is owed) or warns.
- **authored after import:** fields no `.planning` file gives, such as a vault threat's `files`.

The last line counts the entities per kind and the report entries.

## After the import, `check` exits 1

`specforge check` reports the owed judged codes until each verdict is recorded, so it exits 1 on any project with an
executed plan. That is the expected result. It also reports any rule error the imported records carry: the import is
an importing command (ADR 0392), so a namespaced rule error of a third-party extension, such as `gsd.security/E101` (a
threat id claimed by two plans), `gsd/E201` or `gsd/E312`, is written rather than refused, and the user fixes it in
`spec/gsd/` with SpecForge's own tools.

## Refusals

A refusal writes nothing and answers no plan:

- **Re-import.** Any `@gsd/core` entity already in the project, named by kind, path and file. There is no `--force`:
  delete those entities (by default `spec/gsd/`) and run it again. Two imports at once cannot both land: the host's
  revision check refuses the second (D10).
- **Another spec root.** `specforge.json` naming a `spec_root` other than `spec`.
- **Truncated.** "truncated at N": the host's read of `.planning/` stopped early, or the plan would pass its caps
  (2,000 operations, 20,000 report entries). Nothing is written in part (D9).
- **A read error** on a mapped file, which would leave its entities missing; no `.planning/` at the root; a
  `specforge.json` that is missing or not JSON.
- **A host error, E080.** The host still refuses a plan that adds a first-party error (ADR 0392): a path that no longer
  exists (E016 on a complete plan's `files_modified`, a done task's `files`, any `context_refs`), an unresolved
  reference. The message names each path; fix it in `.planning/` and run the import again.

## Never imported

Verdicts and scores (D5); computed, context, human and verdict fields, which the host refuses from any command (D7);
`config.json` into the manifest (D6); `state.json` (`runtime`); `.gitkeep` (`ignored`); Tier 2 sections (key decisions,
deviations, checkpoints, prohibitions, user setup), which are reported.

## The measure

Taskboard, ledger and vault ([../parity/fixtures/](../parity/fixtures/)), imported from `.planning/` alone, are held
against their hand-written twins (D13, `tests/cli/parity_import.rs`): the entity sets equal, with computed fields,
`context_pack` and verdicts left out, except 49 field differences listed with a reason each, and the list fails both
ways. Each fixture's harness codes are pinned under `gsd-compat`, `gsd-strict` and `gsd-plus`; the vault gives two
`gsd.security/W302`, not the twin's five (no `.planning` source gives the other statuses), and carries its two
`gsd.security/E101`. `check` owes exactly the pairs the report lists. The vault imports in under 30 s.

## What stays open

- The host's apply cost grows about quadratically with the plan: 2,000 creates applied in 38 s and 5,000 in 218 s on
  an idle machine, and 960 creates took 286 s under a loaded gate. `MAX_OPS` (2,000) caps one import; a larger
  `.planning/` is refused "truncated at 2000".
- Upstream's `/gsd-import` slash command is not contributed.

## Layout

`src/command.rs` (the command), `src/preflight.rs` (refusals before reading), `src/read/` (one reader per file),
`src/records.rs` and `src/emit.rs` (the plan), `src/table.rs` (the fate table), `src/report.rs`, `src/settings.rs`,
`src/owed.rs`, `src/limits.rs`. Tests: native under `tests/`, through the `specforge` binary under `tests/cli/`, which
loads the vendored builtin by name. The blob is `wasm/gsd_import.wasm`, rebuilt only by the re-vendor
([docs/agents/revendor.md](../../docs/agents/revendor.md)).

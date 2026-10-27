# Who writes a field is declared, and history is the git baseline

**Status:** accepted (2026-10-27, architecture round 31, plan K4). Supersedes [ADR 0009](0009-host-passes-read-declarations.md)
C (the build cache and `lifecycle_field`). Extends [ADR 0014](0014-the-session-keeps-itself-current.md),
[ADR 0019](0019-one-entity-snapshot-per-compile.md), [ADR 0030](0030-a-sessions-inputs-are-one-value.md),
[ADR 0047](0047-the-compiled-project-is-the-sessions-core.md), [ADR 0128](0128-one-project-change-over-held-files.md),
[ADR 0133](0133-recorded-inputs-are-one-table.md), [ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md),
[ADR 0184](0184-a-typed-edit-and-a-rename-write-one-planned-change.md) and
[ADR 0282](0282-an-unknown-type-name-is-the-hosts-e004-and-a-project-has-one-spec-block.md). Takes extension protocol 1.8.0.

## Context

A field is a field: the kernel could not tell a value an author types from one a pass derives (a verification's
`status`) or one only a command may set (a UAT test's `result`), and it kept no history to say that a decision's
`phase` must never change or that a discussion's entries only grow. The one history it had, `specforge-cache.json`,
was opt-in, per machine, and read by nothing but a lifecycle pass. Agents edit specs; without these checks nothing
stops one from writing a verdict or rewriting the past.

## Decisions

- **D1. A field declares its writer.** `writer` is absent (its author), `computed` (a compute pass of the
  declaring extension gives it; a value written by hand is E082) or `harness` (a command's plan sets it; E083 when
  the ledger holds no line for it). A computed value is held beside the graph (`Computed`), merged into the snapshot
  before the rules and shown by every surface as a field marked computed. A computed field is never required,
  scaffolded or completed. A harness field may be required.
- **D2. A field declares its history.** `immutable`: a value that differs from the baseline is E084; unset at the
  baseline, it may be set once. `append_only`: a list whose baseline items are not its prefix is E085. A kind may be
  `append_only`: an entity of it the baseline holds and the project does not is E085. A block member declares the
  same, except `harness`, which is a whole field's (W021). The value compared is the resolved value
  (`wire::bytes(wire::resolved(..))`: references are their targets' full paths), so a rename's respelling is no change.
- **D3. A compute pass.** A pass of phase `compute` declares `computes` (`kind.field`) and answers
  `PassOutput.values`; it runs after the graph is built and before the rules. A value for a field that is not
  `computed` is W165.
- **D4. The harness ledger.** `specforge-harness` at the project root, one line per recorded harness value:
  `path key digest`, sorted, `# specforge-harness 1` header, digest `sha256(path NUL key NUL value)`. It is a check
  input. A value is recorded when any line of its key matches, so a union merge of two branches stays valid.
  `/specforge-harness merge=union` goes into `.gitattributes` (init when an extension declares a harness field, the first ledger write, `harness gc`). W163 is a
  line that is not a line, I203 a file the host would write differently.
- **D5. Every host write keeps the ledger.** A `PlannedChange` carries a `LedgerChange`; `apply` takes
  `.specforge/write.lock`, re-reads the ledger and rewrites it (`Ledger::rewritten`) when another writer changed it,
  keeping that writer's lines. A rename or a scope move re-keys the lines under the entity and records
  `<new> moved:<old>`; the edited project is judged over the planned ledger.
- **D6. History is the git baseline.** A `History` port (locate, commit, merge-base, files) with a `git` CLI
  adapter; the baseline is the project compiled at `HEAD` (or the merge-base under `check --since <rev>`) in the
  current environment, cached in memory by commit and on disk by `(commit, environment)`. It is read only when a
  project declares history. Move records translate it across renames. I204 says there is no history (no repository,
  no commit, no common commit); W164 that it could not be read (git missing, shallow clone without a merge-base).
- **D7. A session watches the baseline's refs.** `HEAD`, the loose ref it names, `packed-refs` and the reftable list
  are recorded check inputs of their own class (`Class::Ref`), watched through their directories' own entries, so a
  commit re-checks the session even when the git directory is outside the project root.
- **D8. A pass's `previous` is the baseline of the kinds it names** (`baseline`), computed values included; the build
  cache, `check --cache`, `lifecycle_field`'s use and W144 are gone.

## Rejected

A flag per rule instead of per field: every extension would reimplement the check. Computed values written into the
graph: the incremental delta would mix derived and authored data. The written spelling as the value: each rename would
make a `phase` E084. A git library: the CLI is already how every user's git is configured, and no shipped crate links one.
Locking only by refusing the later plan (`file_changed`): parallel executors would retry one at a time.

## Consequences

Protocol 1.8.0; the product extension's blob and the builtins are rebuilt after K6. `check --cache` is a usage error and
W144 is retired. A plain `check` in CI compares `HEAD` with itself: judge a branch with `check --since origin/main` and
a full-depth clone. The LSP rename (a client-applied edit) does not carry the ledger until K5. Only a change that
rewrites the ledger takes the write lock; K5 widens it to every host write.

# An extension writes through an edit plan the host applies

**Status:** accepted (2026-10-27, architecture round 32, plan K5). Extends [ADR 0011](0011-extension-command-contract-and-the-product-surface.md),
[ADR 0013](0013-typed-extension-calls.md), [ADR 0017](0017-one-extension-surface-table.md),
[ADR 0022](0022-a-mutation-reports-what-it-wrote.md), [ADR 0023](0023-the-lsp-reads-a-document-through-one-module.md),
[ADR 0128](0128-one-project-change-over-held-files.md), [ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md),
[ADR 0162](0162-a-mutations-preview-is-a-fact-of-its-outcome.md),
[ADR 0184](0184-a-typed-edit-and-a-rename-write-one-planned-change.md) and
[ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md). Leaves
[ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md) D1 standing: a component is still granted no capability.
Takes extension protocol 1.9.0. Extended by [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (`set_verdict` is checked through the host's judged rules). Amended by [ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (a writing collector's plan is judged against the report its collect wrote). Amended by [ADR 0298](0298-an-agent-does-not-change-or-delete-what-a-person-wrote.md) (D7: an existing human-only entity is not edited, nor a person's value deleted). Extended by [ADR 0301](0301-a-human-attested-command-runs-only-for-a-person-at-a-terminal.md) (a human-attested command runs only for a person at a terminal).

## Context

Extension commands and tools only read. GSD's commands write: a plan, a summary, a verification, a pause. Granting a
component the file system would end the sandbox ADR 0037 proves; writing only in host code would put GSD's knowledge in
the core, which the zero-domain rule forbids.

## Decision

- **D1. A command or a collector that declares `writes` may return an edit plan**: typed operations (create an entity
  in a file, set or unset a field or a member path `field[i].member`, append to and remove from a list, link, unlink,
  retitle, add or remove a verify statement, delete, rename, set a verdict). A rename is K2's (its move record is
  written); moving an entity to another scope is a `set` of its scope field; there is no op that moves a block between
  files. Each op but `set_verdict` names the revision of its file: `sha256:<hex>` of the bytes the host compiled and
  sent in the command's input (`revisions`), or `absent` for a file it creates. Files the host edits on the plan's
  behalf (a rename's referrers) name none. A plan is applied only when its call exits 0. Ops apply in order, each over
  the texts the ones before it left, so a plan may renumber.
- **D2. The host is the only writer.** One apply step serves extension plans, typed edits, rename and collectors. It
  refuses a computed value (K4); a harness value set by anything but a plan from a command or collector of the field's
  extension or of an extension naming it as a peer (a core typed edit never sets one); and a human-only value in an
  agent-facing write (D7). It plans the ops with the typed edit engine and judges the result (E080, guard passes
  included). It commits all or nothing under the write lock, journaled, so a write interrupted by a crash is rolled
  back by the next writer (W167 while one is pending), and recompiles. The harness ledger's rewrite (K4 D6) is part of
  the same change. A plan naming a file whose revision changed is refused `stale_revision`. Plans touching disjoint
  files never conflict on write: the ledger is re-applied under the lock, and a plan committed after another write
  landed is re-judged and refused E080 when the union would add an error. So the executors of one wave each write
  their own plan's file in parallel.
- **D3. Every surface can show a plan before applying it**: the CLI's `--dry-run` on a writing command prints the
  diff, and MCP returns it under `dry_run`. A writing command's MCP tool is a mutation.
- **D4. Collectors write the same way**: a writing collector receives the graph and revisions, and its plan records
  harness values (test results, commits) and may create entities (GSD's commits); the host records their harness
  digests (K4). A collector never writes a computed value: it files a report a compute pass reads (K4 U1).
- **D5. Hand edits stay free**, and the flags are checked on compile whoever wrote the file. A pass of phase `guard`
  runs only when a write is judged, reads the project before the write as `previous`, and refuses it through E080;
  it never runs on compile.
- **D6. `set_verdict` writes a judged rule's verdict (K7)**: `{entity_id, rule, outcome, judge, model, findings,
  cited, basis}`; it replaces the entity's verdict for that rule, and `set` never writes one. The host refuses it
  unless the rule is judged and targets the entity's kind, the outcome is one the rule declares, the plan's command
  belongs to the rule's extension and `judge` is the rule's judge, and every cited ref exists. It resolves the rule's
  inputs and `cited` (K7 D8): a digest that differs from `basis` refuses `stale_revision` naming the input, so a
  verdict never covers content its judge did not see. Its `basis` is its revision: it names no file revision and is
  planned on the entity's current text. It writes `inputs` from its own digests and `at` from its clock, and records
  the verdict in `specforge-harness` (K4 D2) under the key `verdict:<rule>`. There is no remove op: a re-check
  replaces, `delete` takes the entity's verdicts with it, and a hand removal fires the rule. The host reads judged
  rules through a port K7 implements; until then every `set_verdict` is refused `not_judged`.
- **D7. A kind, a field or a member may be human-only** (`writer: "human"`; GSD's `override`). No extension plan and
  no MCP write sets one; a CLI typed edit does only from an interactive terminal; hand edits and the editor's own edits
  are free. Proven: no SpecForge write path an agent drives writes one. Not proven: who typed a file; an agent's raw
  write is the host agent's write guard's to deny (K9).
- **D8. The language server's rename** returns versioned source edits, and holds the rename's ledger change: it
  overlays it on its own checks, and writes it under the write lock once the rename lands on disk (dropped when the
  rename is undone). The same rename through `specforge.rename` writes its ledger change in its one apply.

## Rejected

Pre-opening project directories to components: a public registry's extensions would write anywhere under them. Writes
in a host-side GSD bridge only: domain knowledge in the core. Text patches or whole files as a plan: no flag check
without re-reading what they mean. Per-entity revisions: plans on one file would need rebasing, and the model gives
each plan its own file. Explicit MCP tools answering plans: they receive no project, so they have no revision. The
ledger as one more edit in the LSP rename's workspace edit: the client applies it blindly, with no lock, over a ledger
that may have changed.

## Consequences

A new return shape in the command and collector protocol (minor version): `EditPlan`, `writes`, `revisions`,
`GraphNode.file`, the writing collector's input, `writer: "human"`, phase `guard`. An apply step in ops shared by the
CLI and MCP, with the language server's one host write being its rename's ledger change. Revisions computed from the
compile's text when a writing command is called and from disk under the lock. Every host write under one lock and one
journal (W167). A `set_verdict` op beside `add_verify`, behind a judged-rules port. The rename (`specforge.rename`,
and `textDocument/rename` through D8) re-keys the ledger the same way; K4's move records must then be applied as one
substitution of the records newer than the baseline, so a renumbering is judged against the right entities.

## Implementation notes

The write lock is taken only for a project that has a `.specforge/` (a harness, a writing extension, a migration
record), so a plain project gets no new file from a write; `format` and `migrate` commit under it too, and `format`
compares each file with the text it read before it replaces it. Guard passes run through
`CompiledProject::judged_against(previous)`, never in a compile. `set_verdict` is refused `not_judged` until a host
implements `JudgedRules` (K7).

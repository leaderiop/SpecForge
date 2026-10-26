# A migration lays out through the formatter

**Status:** accepted (2026-10-23, architecture round 26, plan 06). Amended by [ADR 0266](0266-a-source-line-break-has-one-reader.md): a migration writes the file's line break around the blocks it lays out. Extends [ADR 0149](0149-a-file-states-a-kinds-shared-fields-once.md)
D12 (the hoist decides what moves; the formatter lays it out), [ADR 0186](0186-a-migration-is-judged-before-it-is-written.md)
D5 (the configuration is looked up after the environment, before the plan) and D7 (the plan writes what `format`
writes, as it asks the rule the judge applies), and [ADR 0169](0169-the-format-configuration-is-read-through-the-callers-disk.md)
D3 (a migration's disk). Cites [ADR 0021](0021-formatting-is-one-operation.md) D2 and [ADR 0147](0147-a-value-is-written-in-its-fields-one-form.md).

## Context

Every writer of `.spec` text laid out through the formatter with the file's `.specforgefmt.toml` (typed edits
through `block::canonical`, init, `format` and the LSP) but the `defaults` migration step, which wrote its block
with a two-space indent and keys padded by hand and left the blocks it shortened as they were: padded for the key it
removed. A project `format --check` passed failed it after `specforge migrate`, at the default configuration (the
padding) and under any other indent (the block). The step's own contract left the layout "to `specforge format`".

## Decision

- **D1. A migration lays out every block its steps changed.** After the header and the steps, each block of the
  planned text that is new or whose lines differ from the block of the same kind, id and rank in the text read is
  laid out by `specforge_formatter::format_range` (`specforge_migrate`'s `layout::lay_out`), from the last up. No other
  byte changes; a file no step changes is skipped as it is. The diff, the hashes, the record and the judge read the
  laid-out text.
- **D2. With the file's configuration and the project's forms.** `migrate_project` takes each source's
  `FormatConfig` from its caller; ops looks it up with one `Configs` per run over the run's bound
  (`caller.containment().disk()`), after the environment and before the plan; the forms are the environment's,
  those the plan already reads. Its W141 is the format run's to report.
- **D3. The steps write raw text.** The hoist's block is one text for the E077 probe and the insert, unpadded; its
  layout is the formatter's. A step never lays out.
- **D4. Layout is not judged.** No code, no refusal: the formatter keeps the graph
  (`formatting_semantic_preservation`) and the judge's graph comparison still refuses a plan that changed it.

## Rejected

- Formatting the whole changed file: it rewrites lines no step touched and disagrees with typed edits ("no byte
  outside the touched blocks changes").
- Laying out only the hoist's blocks: the titles step's output is canonical on canonical input, so excluding it buys
  nothing and needs the steps to report what they changed.
- The migrate crate reading `.specforgefmt.toml` (ADR 0169's reopen trigger: a configuration read outside ops), or a
  `RefCell<Configs>` per call (ADR 0169 D4).
- Reusing `block::canonical`: it lays out one entity it is about to read and refuses `block_unreadable`; the
  migration compares two texts.

## Consequences

- `specforge migrate` and MCP `specforge.migrate` leave a canonical project canonical; a block a step touched that was
  not canonical before comes out canonical (a header split over two lines is joined).
- Rust API: `specforge_migrate::migrate_project` takes `layout: &dyn Fn(&Path) -> FormatConfig`.
- Spec: `migration_canonical_form`; `load_format_config` pins the bound for a migration.

## What would reopen it

A migration step that must keep a layout the formatter would change (then it is a formatter rule first), or a
formatter that changes a graph.

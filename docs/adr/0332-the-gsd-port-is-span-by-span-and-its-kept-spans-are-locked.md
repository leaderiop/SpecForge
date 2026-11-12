# The gsd port is span by span, and its kept spans are locked

**Status:** accepted (2026-11-12, R3 of the Beyond Parity build plan). **Amends**
[ADR 0343](0343-a-gsd-block-calls-only-specforge.md) D4: the port is not line for line, and dead upstream text is
deleted, not blanked. No protocol change.

## Context

- ADR 0343 D4 kept every `@gsd/core` and `@gsd/security` asset upstream's text line for line: the asset and its
  upstream file had the same number of lines, and a line could differ only where a regex, the port rule's
  `served_by_specforge`, or a table keyed by line number allowed it.
- The tables were 13 files under `gsd/core/tests/common/` (`*_rewrites.rs`, 3,113 lines holding 1,699 tuples) and
  `@gsd/security`'s `REWRITTEN`. A rewrite had to keep its line count, and an edit that moved lines moved every row
  after it.
- Deleted upstream text stayed behind as blank padding: 3,914 lines in 90 assets that every prompt still carried.
- The family read Markdown with hand scanners: 26 lines in 15 files split on a backtick mark. They disagreed with
  CommonMark on 703 and 711 fence lines in 35 assets, and split multi-line and double-backtick code spans wrongly.

## Decisions

- **D1. The port is span by span.**
  - An asset's body, after its attribution header, is aligned with its upstream body by a patience diff. The
    upstream body is the tracked release's file; agents and commands lose their frontmatter.
  - Kept spans are equal runs. Own spans are the rest: replaced, inserted and deleted lines.
  - A kept run of only blank and fence-mark lines between two own spans folds into them.
  - Line counts may differ.
- **D2. The port lock.**
  - `gsd/core/tests/port.lock` records every span of every `@gsd/core` and `@gsd/security` asset, in order. A kept
    span records its upstream range and the hash of its text. An own span records its upstream range, its own
    length, the hash of the upstream text, the hash of its own words, and its group.
  - A test recomputes the lock and fails on any difference. `GSD_BLESS=1` rewrites it.
  - The words hash ignores blank lines, indentation and fence width, so a layout edit keeps its review; the own
    length still moves the lock.
- **D3. Span groups.** Every own span holds one group of a closed vocabulary:
  - computed groups (`ported-name`, `served`, `typed-edit`, `markdown`, in that precedence), which the test
    recomputes; `typed-edit` is D9's;
  - reviewed groups (`relic`, `answer`, `args`, `files`, `git`, `jq`, `prose`, `quick`, `review`, `thread`, `wait`,
    `workflow`, `agent`), which a bless carries from the row with the same asset, upstream hash and words hash;
  - `unreviewed`, which a bless writes for a span it cannot carry, and which fails.

  The per-line tables of ADR 0343 D4 and ADR 0351 D9 are gone. Their reasons are the groups.
- **D4. Quotes are implicit.** A kept span is the upstream quote. A link to
  `https://github.com/open-gsd/gsd-core/blob/v<tracked>/<path>` is the only other kind. There are no inline quote
  markers.
- **D5. Dead upstream text is deleted, not blanked.** A deletion is an own span with no asset line, or with one blank
  line between two non-blank ones. Its group says why.
- **D6. An upstream move is span by span.**
  - The move commit re-blesses with `GSD_BLESS=resync`, which marks `resync` each span whose upstream text moved.
  - `no_span_waits_for_resync` is ignored until the last re-port.
  - `upgrade.sh`'s backlog lists the lock spans each upstream change touches.
  - `PENDING_RESYNC` keeps only upstream test suites.
- **D7. One Markdown reader.**
  - `gsd_kit::markdown` (pulldown-cmark, CommonMark, no extension) is how the gsd family reads Markdown, in the
    guest and in tests. Its `tables` alone turns GFM tables on: a table's cells are what it reads, and no gsd code
    cuts a table row at `|` by hand.
  - It reads the text under a section tag again as Markdown, below the tag lines; a comment is never read again.
  - Every asset fence reads as written. A change that makes it so is a `markdown` span.
- **D8. The names ledger.**
  - A GSD file name is a name of `gsd_kit::files::PLANNING_FILES`: the names of the file table of
    `gsd/core/spec/README.md`, plus the names upstream's text keeps under `.planning/` that the table lacks.
  - Outside an upstream quote (D4), a name in own text is a row of `gsd/core/tests/common/planning_names.txt`. Own
    text is an asset's own spans, an agent's recording section and a prompt's Rust-authored lines.
  - The ledger only shrinks, and `.planning` is never a row.
  - The preamble's file table (held equal to the README), the Codex and Cursor headers (upstream's text, pinned to
    the snapshot) and the slash metadata are not own text.
- **D9. The write ledger and `typed-edit`.**
  - A line that tells an agent to write a GSD file is a row of `gsd/core/tests/common/markdown_writes.txt`, in kept
    and own text alike. In prose that is a write verb and a GSD file name in one sentence; in a shell fence, an output
    redirect or `tee` into one, the redirect's direction read by the SDK shell reader. Its row count is the port's
    write measure.
  - An own span whose upstream side has such a line, and whose own side has none and records a typed edit (the
    catalogue's `edit` tool or a `specforge gsd` writer), is computed `typed-edit`.
- **D10. The tool counter.**
  - The distinct graph tools of the engine-tool catalogue
    ([ADR 0334](0334-engine-tools-are-one-catalogue-below-the-surfaces.md)) named in the 44 claude-code renders and
    the installed claude-code agent bodies, each body cut at its generated `## SpecForge tools` section, may not fall
    below `AT_LEAST` in `gsd/core/tests/cli/rendered_lines.rs`, and a rise raises it in the same commit. It was 6 when
    the counter landed.
  - An MCP name counts anywhere; a CLI line counts only as a command in code.

## Rejected

- Line for line with blank padding: the padding cost every prompt, and the tables moved with every edit.
- A reason per row: a group says why; the lock diff shows the span.
- Inline quote markers: they cost tokens in every prompt.
- A file-level resync list for assets: the lock names the exact spans.
- An exact hash of the own text: a layout edit would lose its review.
- GFM tables everywhere: a `|` would split a code span outside a table; only `tables` reads them.
- A bless for the ledgers: it would let them grow.
- Folding the literal `.planning` checks into the names ledger: they are stricter (kept text, every host, no
  ledger).
- Counting the generated tools section: it is the catalogue rendered, not a prompt using it.
- CLI names in bare prose: they are not a call.
- A hand list of the named tools: the catalogue is the dictionary.
- comrak: an arena AST over GFM, and more crates; an offset iterator is all the family needs.
- A blank line inserted after every section tag: 125 files touched to suit a reader.

## Consequences

- Every asset edit re-blesses the lock and reviews its new spans.
- A later group is an amendment of D3.
- ADR 0351 D9's `thread_rewrites.rs` is now the `thread` group.

## What would reopen it

- Lock churn that outweighs the edits it reviews (patience misalignment).
- An upstream format that is not CommonMark.

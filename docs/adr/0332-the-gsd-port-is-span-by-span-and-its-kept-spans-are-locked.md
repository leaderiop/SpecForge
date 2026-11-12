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
  - computed groups (`ported-name`, `served`, `markdown`), which the test recomputes;
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
    guest and in tests.
  - It reads the text under a section tag again as Markdown, below the tag lines; a comment is never read again.
  - Every asset fence reads as written. A change that makes it so is a `markdown` span.

## Rejected

- Line for line with blank padding: the padding cost every prompt, and the tables moved with every edit.
- A reason per row: a group says why; the lock diff shows the span.
- Inline quote markers: they cost tokens in every prompt.
- A file-level resync list for assets: the lock names the exact spans.
- An exact hash of the own text: a layout edit would lose its review.
- GFM tables: a `|` would split a code span.
- comrak: an arena AST over GFM, and more crates; an offset iterator is all the family needs.
- A blank line inserted after every section tag: 125 files touched to suit a reader.

## Consequences

- Every asset edit re-blesses the lock and reviews its new spans.
- A later group is an amendment of D3.
- ADR 0351 D9's `thread_rewrites.rs` is now the `thread` group.

## What would reopen it

- Lock churn that outweighs the edits it reviews (patience misalignment).
- An upstream format that is not CommonMark.

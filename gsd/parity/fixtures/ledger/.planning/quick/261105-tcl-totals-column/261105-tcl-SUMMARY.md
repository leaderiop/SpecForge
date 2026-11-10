---
quick_id: 261105-tcl
slug: totals-column
key-files:
  created: []
  modified: [src/lib.rs, src/main.rs, tests/totals.rs]
duration: 4min
completed: 2026-11-05
status: complete
---

# Quick Task 261105-tcl: show totals in a fixed-width column — Summary

**`ledger total` prints its total right-aligned in a column of fourteen.**

## Accomplishments
- `column(cents)` in `src/lib.rs`, and `ledger total` printing through it
- A test that pins the column's width

## Task Commits

1. **Task 1: the total's column** - `3f9c2a1` (feat)

## Deviations from Plan

None - plan executed exactly as written.

## Verification

`cargo test --test totals a_total_prints_in_a_column_of_fourteen` passes.

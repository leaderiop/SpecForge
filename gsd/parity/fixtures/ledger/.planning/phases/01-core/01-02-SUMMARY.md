---
phase: 01-core
plan: 02
subsystem: entries
tags: [rust, cli]

requires: [01-01]
provides:
  - "total"
  - "ledger add and ledger total"
affects: [persistence, listing]

tech-stack:
  added: []
  patterns: ["the command line is a thin switch over src/lib.rs"]

key-files:
  created: [src/main.rs, tests/totals.rs]
  modified: [src/lib.rs]

key-decisions:
  - "The total is an i64 of cents"

requirements-completed: [LEDG-02]

coverage:
  - id: D1
    description: "The total is exact, beyond the range of an i32 too"
    requirement: "LEDG-02"
    verification:
      - kind: unit
        ref: "tests/totals.rs#the_total_sums_every_entry"
        status: pass
      - kind: unit
        ref: "tests/totals.rs#total_beyond_i32_is_exact"
        status: pass
    human_judgment: false
  - id: D2
    description: "The command line prints the total of a store"
    human_judgment: true
    rationale: "The output wording is a judgement a person makes at the terminal"

duration: 9min
completed: 2026-11-04
status: complete
---

# Phase 1: Core — Plan 02 Summary

**`total` as an `i64` of cents, and `ledger add` and `ledger total` over it.**

## Performance

- **Duration:** 9 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `total` sums every entry exactly, beyond `i32::MAX` too
- `src/main.rs` with `add` and `total`

## Task Commits

1. **Task 1: the total** - `c41a7f2` (feat)
2. **Task 2: the command line** - `5e09b3d` (feat)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

Persistence can write the entries `add` makes.

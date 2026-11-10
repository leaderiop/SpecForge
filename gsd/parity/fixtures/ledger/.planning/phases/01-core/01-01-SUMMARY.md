---
phase: 01-core
plan: 01
subsystem: entries
tags: [rust, std]

requires: []
provides:
  - "Entry and add"
affects: [01-02, persistence]

tech-stack:
  added: []
  patterns: ["an entry is plain data: { cents, memo }"]

key-files:
  created: [Cargo.toml, Cargo.lock, src/lib.rs, tests/entries.rs]
  modified: []

key-decisions:
  - "Amounts are integer cents, never floats"

requirements-completed: [LEDG-01]

coverage:
  - id: D1
    description: "An added entry keeps its cents and its memo"
    requirement: "LEDG-01"
    verification:
      - kind: unit
        ref: "tests/entries.rs#an_entry_keeps_its_cents_and_memo"
        status: pass
      - kind: unit
        ref: "tests/entries.rs#a_memo_with_a_tab_is_refused"
        status: pass
    human_judgment: false

duration: 7min
completed: 2026-11-03
status: complete
---

# Phase 1: Core — Plan 01 Summary

**Entries in integer cents with `add`, and the store's one-line format.**

## Performance

- **Duration:** 7 min
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `Entry`, `add`, `line` and `parse` in `src/lib.rs`
- Tests that pin an entry, a refused memo and a store line's round trip

## Task Commits

1. **Task 1 and 2: the entry model and add** - `8b1d0e4` (feat)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

The total and the command line can build on the entries.

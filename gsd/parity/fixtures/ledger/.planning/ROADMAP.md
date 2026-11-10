# Roadmap: Ledger

## Overview

Ledger grows from entries and a total to a store that keeps them, then to a listing.
Phase 1 is the entry model and the total, phase 2 makes it durable, phase 3 lists the entries.

## Phases

- [x] **Phase 1: Core** - Entries can be added and totalled
- [ ] **Phase 2: Persistence** - Entries are saved to disk and read back
- [ ] **Phase 3: Listing** - Entries are listed in the order they were added

## Phase Details

### Phase 1: Core

**Goal**: Entries can be added and totalled
**Depends on**: Nothing (first phase)
**Requirements**: [LEDG-01, LEDG-02]
**Success Criteria** (what must be TRUE):
  1. A user can add an entry in integer cents with a memo
  2. The total of the entries is exact to the cent

**Plans**: 2/2 plans complete

Plans:
- [x] 01-01: The entry model and `add`
- [x] 01-02: The total

### Phase 2: Persistence

**Goal**: Entries are saved to disk and read back
**Depends on**: Phase 1
**Requirements**: [LEDG-03]
**Success Criteria** (what must be TRUE):
  1. An added entry is written to `ledger.tsv`
  2. A restarted ledger shows the entries it had

**Plans**: 1 plan

Plans:
- [ ] 02-01: Save and load `ledger.tsv`

### Phase 3: Listing

**Goal**: Entries are listed in the order they were added
**Depends on**: Phase 2
**Requirements**: [LEDG-04]
**Success Criteria** (what must be TRUE):
  1. `ledger list` shows every entry, oldest first
  2. An empty ledger lists nothing and says so

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core | 2/2 | Complete    | 2026-11-04 |
| 2. Persistence | 0/1 | Not started | - |
| 3. Listing | 0/0 | Not started | - |

# Roadmap: Taskboard

## Overview

Taskboard grows from an in-memory list to a board that keeps its tasks on disk and answers questions about them.
Phase 1 is the core model, phase 2 makes it durable, phase 3 makes it useful to read.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Core** - Tasks can be added and completed
- [ ] **Phase 2: Persistence** - Tasks are saved to disk and read back
- [ ] **Phase 3: Listing** - Open tasks are listed and filtered

## Phase Details

### Phase 1: Core

**Goal**: Tasks can be added and completed
**Depends on**: Nothing (first phase)
**Requirements**: [TASK-01, TASK-02]
**Success Criteria** (what must be TRUE):
  1. A user can add a task with a title
  2. A user can mark a task complete by its id

**Plans**: 2/2 plans complete

Plans:
- [x] 01-01: The task model and `add`
- [x] 01-02: `complete` and the command line

### Phase 2: Persistence

**Goal**: Tasks are saved to disk and read back
**Depends on**: Phase 1
**Requirements**: [STORE-01, STORE-02]
**Success Criteria** (what must be TRUE):
  1. Every change to the board is written to `tasks.json`
  2. A restarted board shows the tasks it had
  3. A title comes back exactly as it was written

**Plans**: 1 plan

Plans:
- [ ] 02-01: Save and load `tasks.json`

### Phase 02.1: Hotfix: titles survive a reload (INSERTED)

**Goal**: A title is written to disk and read back unchanged
**Requirements**: TBD
**Depends on:** Phase 2
**Success Criteria** (what must be TRUE):
  1. A title with a leading space or a line break comes back unchanged
  2. An empty title is refused

**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd-plan-phase 02.1 to break down)

### Phase 3: Listing

**Goal**: Open tasks are listed and filtered
**Depends on**: Phase 2
**Requirements**: [TASK-03, LIST-01]
**Success Criteria** (what must be TRUE):
  1. `taskboard list` shows the open tasks, oldest first
  2. `taskboard list <word>` shows only tasks whose title has the word

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core | 2/2 | Complete    | 2026-03-03 |
| 2. Persistence | 0/1 | Not started | - |
| 3. Listing | 0/0 | Not started | - |

# Roadmap: Vault

## Overview

Vault grows from a sealed file to a store that can hand a secret to someone else.
Phase 1 is the sealed store and its command line, phase 2 is sharing.

## Phases

- [x] **Phase 1: Core** - Secrets can be stored and read back
- [ ] **Phase 2: Sharing** - A secret can be shared with another person

## Phase Details

### Phase 1: Core

**Goal**: Secrets can be stored and read back
**Depends on**: Nothing (first phase)
**Requirements**: [SEC-01, SEC-02]
**Success Criteria** (what must be TRUE):
  1. A user can store a secret under a name
  2. A user can read a stored secret back by its name

**Plans**: 2/2 plans complete

Plans:
- [x] 01-01: The sealed store
- [x] 01-02: The command line

### Phase 2: Sharing

**Goal**: A secret can be shared with another person
**Depends on**: Phase 1
**Requirements**: [SHARE-01]
**Success Criteria** (what must be TRUE):
  1. A user can export one secret, sealed for a recipient

**Plans**: 1 plan

Plans:
- [ ] 02-01: Export a secret for a recipient

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core | 2/2 | Complete    | 2026-10-20 |
| 2. Sharing | 0/1 | Not started | - |

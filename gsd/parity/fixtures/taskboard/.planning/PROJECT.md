# Taskboard

## What This Is

A small board for tracking tasks from the command line. A user adds a task with a title, marks it complete and lists
what is left. It is a Node 20 script with no dependencies, built to prove the GSD model in SpecForge.

## Core Value

A task is never lost.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] A user can add a task with a title
- [ ] A user can mark a task complete
- [ ] Tasks survive a restart
- [ ] A user can list the open tasks and filter them

### Out of Scope

- Sharing a board by link — needs accounts, which v1 does not have

## Context

Built to prove the GSD model in SpecForge. Small enough that one person reads all of it.

## Constraints

- **Tech stack**: Node 20, no dependencies — it ships as one script

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| ES modules, no build step | `node src/cli.js` must just run | — Pending |
| Tasks keep an integer id | Short to type: `taskboard done 3` | — Pending |

---
*Last updated: 2026-03-02 after initialization*

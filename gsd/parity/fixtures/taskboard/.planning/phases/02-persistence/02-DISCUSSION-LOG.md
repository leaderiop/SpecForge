# Phase 2: Persistence - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-04
**Phase:** 2-persistence
**Areas discussed:** Storage format, Titles

---

## Storage format

| Option | Description | Selected |
|--------|-------------|----------|
| One JSON file | `tasks.json` next to the script, rewritten on every change | ✓ |
| SQLite | A database file, which needs a native dependency | |

**User's choice:** One JSON file
**Notes:** The project ships as one script with no dependencies, so SQLite is out.

---

## Titles

| Option | Description | Selected |
|--------|-------------|----------|
| Stored as typed | Keep every character the user wrote | ✓ |
| Trimmed | Drop leading and trailing space | |

**User's choice:** Stored as typed
**Notes:** A title is the user's own words.

---

## Claude's Discretion

The indent of the JSON file.

## Deferred Ideas

Keeping the file in step across two machines (SYNC-01, v2).

---

*Phase: 02-persistence*
*Discussion log generated: 2026-03-04*

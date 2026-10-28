# Phase 2: Persistence - Research

**Researched:** 2026-03-04
**Domain:** File-backed storage for a small Node 20 command line tool
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- D-01: Tasks are kept in a single JSON file, `tasks.json`, in the working directory
- D-02: The file is written to a temporary name and renamed over `tasks.json`
- D-03: A title is stored exactly as typed, with no trimming

### Claude's Discretion
- The indent of the JSON file

### Deferred Ideas (OUT OF SCOPE)
- Keeping the file in step across two machines
</user_constraints>

<architectural_responsibility_map>
## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Persist the board | Database/Storage | — | The file is the only store |

</architectural_responsibility_map>

<research_summary>
## Summary

A board of a few hundred tasks fits in one JSON file. Node's own `fs` module reads and writes it, and a write to a temporary name followed by a rename keeps the file whole if the process dies mid-write.

**Primary recommendation:** `JSON.stringify` the board and `fs.renameSync` a temporary file over `tasks.json`.
</research_summary>

<standard_stack>
## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| node:fs | 20 | Read and write `tasks.json` | Built in, no dependency |

</standard_stack>

<architecture_patterns>
## Architecture Patterns

### Pattern 1: Write then rename
**What:** Write the new content to a temporary file in the same directory, then rename it over the target.
**When to use:** Every save.

### Anti-Patterns to Avoid
- **Writing in place:** a crash mid-write leaves a truncated file.
</architecture_patterns>

<dont_hand_roll>
## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Two processes saving at once | A lock file protocol | One process per board | Lock files go stale after a crash |

</dont_hand_roll>

<common_pitfalls>
## Common Pitfalls

### Pitfall 1: Torn writes
**What goes wrong:** A crash during `writeFileSync` leaves a half-written `tasks.json`.
**Why it happens:** `writeFileSync` truncates the file before it writes.
**How to avoid:** Write then rename.
**Warning signs:** `JSON.parse` throws on start.
</common_pitfalls>

<open_questions>
## Open Questions (RESOLVED)

1. **Is one file enough for a thousand tasks?** — RESOLVED: yes, D-01 keeps one file and a thousand titles are under 100 KB.
   - What we know: a task is a few dozen bytes.
   - What's unclear: how large a board grows in practice.
   - Recommendation: revisit if a board passes ten thousand tasks.
</open_questions>

<sources>
## Sources

### Primary (HIGH confidence)
- https://nodejs.org/docs/latest-v20.x/api/fs.html - `renameSync` replaces the target atomically on one filesystem
</sources>

---

*Phase: 02-persistence*
*Research completed: 2026-03-04*
*Ready for planning: yes*

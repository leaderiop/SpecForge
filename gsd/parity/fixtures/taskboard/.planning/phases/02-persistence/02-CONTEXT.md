# Phase 2: Persistence - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Tasks are saved to disk after every change and read back when the board starts. Listing and filtering are phase 3.

</domain>

<decisions>
## Implementation Decisions

### Storage format
- **D-01:** Tasks are kept in a single JSON file, `tasks.json`, in the working directory
- **D-02:** The file is written to a temporary name and renamed over `tasks.json`, so a crash never leaves half a file

### Titles
- **D-03:** A title is stored exactly as typed, with no trimming

### Claude's Discretion
- The indent of the JSON file

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements are fully captured in decisions above

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/tasks.js`: the task list and its operations

### Established Patterns
- ES modules, no build step

### Integration Points
- `src/cli.js` loads the board before each command and saves it after

</code_context>

<deferred>
## Deferred Ideas

- Keeping the file in step across two machines — v2, SYNC-01

</deferred>

---

*Phase: 02-persistence*
*Context gathered: 2026-03-04*

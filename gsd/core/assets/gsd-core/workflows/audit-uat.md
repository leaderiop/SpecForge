<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/audit-uat.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Cross-phase audit of all UAT and verification files. Finds every outstanding item (pending, skipped, blocked, human_needed), optionally verifies against the codebase to detect stale docs, and produces a prioritized human test plan.
</purpose>

<process>

<step name="initialize">
Run the CLI audit:

```bash
# the commands below are specforge commands: there is no launcher to define
AUDIT=$(specforge query "$MILESTONE" --depth 3 --kinds uat_test,human_item --format json)
```

Parse JSON for `results` array and `summary` object.

If `summary.total_items` is 0 AND `summary.parse_gap_files` is 0:
```
## All Clear

No outstanding UAT or verification items found across all phases.
All tests are passing, resolved, or diagnosed with fix plans.
```
Stop here.

If `summary.total_items` is 0 but `summary.parse_gap_files` is greater than 0, this is NOT all-clear — one or more files have test blocks that could not be parsed and may be hiding outstanding work. Continue to the `categorize` and `present` steps below; `present` renders the Unparsed section from these `parse_gap: true` entries even though `items` is empty for a mixed project.
</step>

<step name="categorize">
Group items by what's actionable NOW vs. what needs prerequisites:

**Testable Now** (no external dependencies):
- `pending` — tests never run
- `human_uat` — human verification items
- `skipped_unresolved` — skipped without clear blocking reason

**Needs Prerequisites:**
- `server_blocked` — needs external server running
- `device_needed` — needs physical device (not simulator)
- `build_needed` — needs release/preview build
- `third_party` — needs external service configuration

For each item in "Testable Now", use Grep/Read to check if the underlying feature still exists in the codebase:
- If the test references a component/function that no longer exists → mark as `stale`
- If the test references code that has been significantly rewritten → mark as `needs_update`
- Otherwise → mark as `active`
</step>

<step name="present">
If `summary.parse_gap_files` is greater than 0, render this section FIRST (before the `## UAT Audit Report` below, or standalone if `total_items` is also 0). Filter `results` for entries with `parse_gap: true`:
```
## Unparsed UAT Files ({parse_gap_files} files)

| Phase | File | Unparsed Blocks |
|-------|------|------------------|
| {phase} | {file_path} | {unparsed_blocks} |
...

These files have `### N.` test blocks with no readable `result:` line. Fix the file's structure, then re-run the audit.
```
This fires for ANY project with `parse_gap_files > 0`, including a mixed project where other phases also have real outstanding `items` — the two sections are not mutually exclusive. An outstanding item does not stop mattering because its phase belongs to an already-archived milestone (a deferred human-UAT scenario or a `skipped` live-stack test is exactly what gets archived still-open), so an archived phase's parse gap is listed in this same table, unfiltered by `archived_milestone`. If `total_items` is 0, stop after this section (there is no `## UAT Audit Report` to present). Otherwise continue below.

Present the audit report:

```
## UAT Audit Report

**{total_items} outstanding items across {total_files} files in {phase_count} phases**

### Testable Now ({count})

| # | Phase | Test | Description | Status |
|---|-------|------|-------------|--------|
| 1 | {phase} | {test_name} | {expected} | {active/stale/needs_update} |
...

### Needs Prerequisites ({count})

| # | Phase | Test | Blocked By | Description |
|---|-------|------|------------|-------------|
| 1 | {phase} | {test_name} | {category} | {expected} |
...

### Stale (can be closed) ({count})

| # | Phase | Test | Why Stale |
|---|-------|------|-----------|
| 1 | {phase} | {test_name} | {reason} |
...

---

## Recommended Actions

1. **Close stale items:** `/gsd:verify-work {phase}` — mark stale tests as resolved
2. **Run active tests:** Human UAT test plan below
3. **When prerequisites met:** Retest blocked items with `/gsd:verify-work {phase}`
```
</step>

<step name="test_plan">
Generate a human UAT test plan for "Testable Now" + "active" items only:

Group by what can be tested together (same screen, same feature, same prerequisite):

```
## Human UAT Test Plan

### Group 1: {category — e.g., "Billing Flow"}
Prerequisites: {what needs to be running/configured}

1. **{Test name}** (Phase {N})
   - Navigate to: {where}
   - Do: {action}
   - Expected: {expected behavior}

2. **{Test name}** (Phase {N})
   ...

### Group 2: {category}
...
```
</step>

</process>

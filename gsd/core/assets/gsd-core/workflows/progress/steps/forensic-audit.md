<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/progress/steps/forensic-audit.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<step name="forensic_audit">
**Forensic Integrity Audit** — only runs when `--forensic` is present in ARGUMENTS.

If `--forensic` is NOT present in ARGUMENTS: skip this step entirely. Default progress behavior (standard report + routing) is unchanged.

If `--forensic` IS present: after the standard report and routing suggestion have been displayed, append the following audit section.

---

## Forensic Integrity Audit

Running 7 deep checks against project state...

Run each check in order. For each check, emit ✓ (pass) or ⚠ (warning) with concrete evidence when a problem is found.

**Check 1 — STATE vs artifact consistency**

Read STATE.md `status` / `stopped_at` fields (from the STATE snapshot already loaded). Compare against the artifact count from the roadmap analysis. If STATE.md claims the current phase is pending/mid-flight but the artifact count shows it as complete (all PLAN.md files have matching SUMMARY.md files), flag inconsistency. Emit:
- ✓ `STATE.md consistent with artifact count` — if both agree
- ⚠ `STATE.md claims [status] but artifact count shows phase complete` — with the specific values

**Check 2 — Orphaned handoff files**

Check for existence of:
```bash
specforge export --kinds pause,handoff --pick '.nodes[].id' || true   # by kind: a pause or handoff outside a phase is linked to nothing
```
A `pause` entity is the handoff marker: there is no marker file.

Emit:
- ✓ `No orphaned handoff files` — if none found
- ⚠ `Orphaned handoff files found` — list each file path, add: `→ Work was paused mid-flight. Read the handoff before continuing.`

**Check 3 — Deferred scope drift**

Search the phases' `context`, `discussion`, `summary`, `verification` and `handoff` entities for the patterns below with `specforge search "<pattern>" --kinds context,discussion,summary,verification,handoff`, and the phases' goals with:
```bash
while IFS= read -r ph; do goal=$(specforge query "$ph" --depth 0 --pick '.nodes[0].fields.goal'); case "$goal" in (*"defer to Phase"*|*"future phase"*|*"out of scope Phase"*|*"deferred to Phase"*) echo "$ph" ;; esac; done < <(specforge query "$MILESTONE" --depth 3 --kinds phase --pick '.nodes[kind=phase].id') || true
```

For each match, extract the referenced phase number. Cross-reference against ROADMAP.md phase list. If the referenced phase number is NOT in ROADMAP.md, flag as deferred scope not captured.

Emit:
- ✓ `All deferred scope captured in ROADMAP` — if no mismatches
- ⚠ `Deferred scope references phase(s) not in ROADMAP` — list: file, reference text, missing phase number

**Check 4 — Memory-flagged pending work**

Check whether a `lesson` entity records pending work:
```bash
specforge export --kinds lesson --pick '.nodes[].id' || true
```

If found, grep for entries containing: `pending`, `status`, `deferred`, `not yet run`, `backfill`, `blocking`.

Emit:
- ✓ `No memory entries flagging pending work` — if none found or no MEMORY.md
- ⚠ `Memory entries flag pending/deferred work` — list the matching lines (max 5, truncated at 80 chars)

**Check 5 — Blocking operational todos**

Check for pending todos:
```bash
echo "no pending todos: todos are not part of @gsd/core"
```

For files found, scan for keywords indicating operational blockers: `script`, `credential`, `API key`, `manual`, `verification`, `setup`, `configure`, `run `.

Emit:
- ✓ `No blocking operational todos` — if no pending todos or none match operational keywords
- ⚠ `Blocking operational todos found` — list the file names and matching keywords (max 5)

**Check 6 — Uncommitted code**

```bash
specforge git status --except-spec --pick '.tracked[]' --first 10 2>/dev/null
```

If output is non-empty (modified/staged files outside the spec), flag as uncommitted code.

Emit:
- ✓ `Working tree clean` — if no modified files outside the spec
- ⚠ `Uncommitted changes in source files` — list up to 10 file paths

**Check 7 — Unresolved deferred items**

The milestone's executor discoveries, with their status: the deferred ideas whose `origin` is `execution` (an executor
records each out-of-scope discovery as one, per `agents/gsd-executor.md`). An idea deferred while a phase's context
was set (`origin` `discussion`) is parked for a later phase on purpose and is not listed:
```bash
specforge query "$MILESTONE" --depth 3 --kinds deferred_idea --pick '.nodes[fields.origin=execution]' --fields id,fields.status,fields.text || true
```

An idea is RESOLVED only when its `status` is `resolved`; every other one — including one with no `status` at all —
is UNRESOLVED and must be surfaced (fail-safe: never silently drop a possibly-open item).

Emit:
- ✓ `No unresolved deferred items` — if the milestone has no deferred idea, or every one is `resolved`
- ⚠ `Unresolved deferred items found` — list each unresolved idea's phase, id and text (max 5 per phase, truncated at 80 chars)

---

After all 7 checks, display the verdict:

**If all 7 checks passed:**
```
### Verdict: CLEAN

The standard progress report is trustworthy — proceed with the routing suggestion above.
```

**If 1 or more checks failed:**
```
### Verdict: N INTEGRITY ISSUE(S) FOUND

The standard progress report may not reflect true project state.
Review the flagged items above before acting on the routing suggestion.
```

Then for each failed check, add a concrete next action:
- Check 2 (orphaned handoff): `Pick the paused work up with /gsd:resume-work, which reads the handoff`
- Check 3 (deferred scope): `Create each missing phase with /gsd:phase, or drop the reference that defers to it`
- Check 4 (memory pending): `Review the flagged memory entries and resolve or clear them`
- Check 5 (blocking todos): `Complete the operational steps in the pending todos before continuing`
- Check 6 (uncommitted code): `Commit or stash the uncommitted changes before advancing`
- Check 7 (unresolved deferred items): `Address each deferred item, or fold it into the roadmap as a phase`
- Check 1 (STATE inconsistency): `Reconcile the phase with /gsd:verify-work ${PHASE}`
</step>

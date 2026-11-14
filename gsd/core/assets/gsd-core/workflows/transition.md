<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/transition.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<internal_workflow>

**This is an INTERNAL workflow — NOT a user-facing command.**

There is no `/gsd-transition` command. This workflow is invoked automatically by
`execute-phase` during auto-advance, or inline by the orchestrator after phase
verification. Users should never be told to run `/gsd-transition`.

**Valid user commands for phase progression:**
- `/gsd:discuss-phase {N}` — discuss a phase before planning
- `/gsd:plan-phase {N}` — plan a phase
- `/gsd:execute-phase {N}` — execute a phase
- `/gsd:progress` — see roadmap progress

</internal_workflow>

<required_reading>

**Read these files NOW:**

1. the computed project state
2. the `project` entity
3. the `milestone` and `phase` entities
4. the current phase's `plan` entities
5. the current phase's `summary` entities

</required_reading>

<purpose>

Mark current phase complete and advance to next. This is the natural point where progress tracking and PROJECT.md evolution happen.

"Planning next phase" = "current phase is done"

</purpose>

<process>

<step name="post_completion_mode" priority="first">

**Invocation mode — read this FIRST.** This workflow runs two ways:

1. **Standalone transition** (normal path): the phase is being marked complete AND
   transitioned by this workflow. Run EVERY step below in order — `verify_completion`,
   `update_roadmap_and_state` (which calls the computed phase status), then the
   post-processing.

2. **Post-completion delegation** (invoked by `execute-phase` after its auto-chain
   completion — #1526): verification already passed in execute-phase's
   `verify_phase_goal`, and the phase's status is computed from it, so there is nothing
   to re-run. SKIP `verify_completion`. Run `cleanup_handoff` (stale handoffs are still
   cleared post-completion) and `update_roadmap_and_state` (a read of the phases), then
   BEGIN at `evolve_project` and run every step from there through `offer_next_phase`
   (the post-processing parity set: graduation scan, accumulated context).
   `archive_prompts` is a documented no-op in either mode.

Detect post-completion mode when the caller states that phase completion and
verification have already run. When in doubt, run standalone (mode 1) — it is
idempotent enough to be safe, just slower.
</step>

<step name="load_project_state" priority="first">

Before transition, read project state:

```bash
specforge query "$MILESTONE" --kinds milestone,phase --format context 2>/dev/null || true
specforge query "$PROJECT" --depth 0 --format context 2>/dev/null || true
```

Parse current position to verify we're transitioning the right phase.
Note accumulated context that may need updating after transition.

</step>

<step name="verify_completion">

Check current phase has all plan summaries:

```bash
specforge query "$PHASE_ID" --kinds plan --format brief 2>/dev/null || true
specforge query "$PHASE_ID" --depth 2 --kinds summary --format brief 2>/dev/null || true
```

**Verification logic:**

- Count PLAN files
- Count SUMMARY files
- If counts match: all plans complete
- If counts don't match: incomplete

<config-check>

```bash
specforge settings --format json 2>/dev/null || true
```

</config-check>

**Check for verification debt in this phase:**

```bash
# the commands below are specforge commands: there is no launcher to define
# #5118: the verification status is read through its ONE owner — never with
# awk/sed/grep over the report (that bypassed the closed enum, the staleness
# check and the phase-pinned report resolution, #3492). A non-zero exit is the
# owner's hard error (a status outside passed | gaps_found | human_needed).
VERIFY_STATUS=$(specforge query "$PHASE_ID" --kinds verification --pick '.nodes[kind=verification].fields.status') || { echo "verification.status refused this phase's report — see the error above." >&2; exit 1; }; VERIFY_STATUS=${VERIFY_STATUS%%$'\n'*}; VERIFY_STATUS=${VERIFY_STATUS:-pending}
```

**If VERIFY_STATUS is not `passed`:**

Stop before confirming:

```
Verification incomplete: ${VERIFY_STATUS}

Resolve before transition. Review: `/gsd:audit-uat`
```

This preliminary check blocks obviously unresolved verification early, ahead
of the authoritative gate below. the computed phase status (in
`update_roadmap_and_state`) remains the authoritative stale-aware gate and
fail-closes unless canonical verification status is `passed`.

**If all plans complete:**

<if mode="yolo">

```
⚡ Auto-approved: Transition Phase [X] → Phase [X+1]
Phase [X] complete — all [Y] plans finished.

Proceeding to mark done and advance...
```

Proceed directly to cleanup_handoff step.

</if>

<if mode="interactive" AND="gates.confirm_transition != false">

Ask: "Phase [X] complete — all [Y] plans finished. Ready to mark done and move to Phase [X+1]?"

Wait for confirmation before proceeding.

</if>

**If plans incomplete:**

**SAFETY RAIL.** Skipping incomplete plans is destructive — ALWAYS prompt regardless of mode.

Present:

```
Phase [X] has incomplete plans:
- {phase}-01-SUMMARY.md ✓ Complete
- {phase}-02-SUMMARY.md ✗ Missing
- {phase}-03-SUMMARY.md ✗ Missing

⚠️ Safety rail: Skipping plans requires confirmation (destructive action)

Options:
1. Continue current phase (execute remaining plans)
2. Mark complete anyway (skip remaining plans)
3. Review what's left
```

Wait for user decision.

</step>

<step name="cleanup_handoff">

Check for lingering handoffs:

```bash
specforge query "$PHASE_ID" --kinds handoff --format brief 2>/dev/null || true
```

If found, delete them — phase is complete, handoffs are stale.

</step>

<step name="update_roadmap_and_state">

There is nothing to write: the phase's status is computed (complete once its verification passed), and the roadmap's checkboxes, plan counts and progress follow from it. Read the milestone's phases:

```bash
specforge gsd progress-report --format json
```

Extract from result: `phases[].number`, `phases[].name`, `phases[].status`.

The next phase is the first phase after this one whose `status` is not `Complete`. When there is none, this was the last phase of the milestone.

</step>

<step name="archive_prompts">

If prompts were generated for the phase, they stay in place.
The `completed/` subfolder pattern from create-meta-prompts handles archival.

</step>

<step name="evolve_project">

Evolve the project to reflect learnings from the completed phase: the `project` entity, its `requirement` entities and its `key_decisions`. Each change is a typed edit (`specforge schema --kind requirement`, likewise `project` and `decision`, lists the fields; `specforge new requirement --help` prints them as flags); the transition's commit is the project's "last updated". A refusal names its code: read it with `specforge explain <code>`.

**Read phase summaries:**

```bash
_SUMMARIES=( $(specforge query "$PHASE_ID" --depth 2 --kinds summary --pick '.nodes[kind=summary].id') )
if [ -n "${_SUMMARIES[0]}" ]; then for s in "${_SUMMARIES[@]}"; do specforge query "$s" --depth 0 --format context; done; fi
```

**Assess requirement changes:**

1. **Requirements validated?**
   - Any Active requirements shipped in this phase?
   - Nothing to write: a requirement's `status` is computed from the phases and verifications that cover it, so one this phase verified reads validated.

2. **Requirements invalidated?**
   - Any Active requirements discovered to be unnecessary or wrong?
   - Move it out of scope with its reason, once per requirement:

   ```bash
   REQ_ID="{the requirement's id, e.g. AUTH-02}"
   IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
   {why it was invalidated}
   TEXT
   specforge set "$REQ_ID" reason "$REASON" || exit 1
   specforge set "$REQ_ID" tier out_of_scope || exit 1
   ```

3. **Requirements emerged?**
   - Any new requirements discovered during building?
   - Record each as a `v1` requirement of the phase that will build it. A `v1` requirement must be in a phase (`gsd/E101`), so it is recorded `v2`, mapped into the phase, then made `v1`:

   ```bash
   REQ_ID="{CATEGORY-NN: the next free id of its category}"
   IFS= read -rd '' TEXT <<'TEXT' || true; TEXT=${TEXT%$'\n'}
   {the requirement: user-centric, testable, atomic}
   TEXT
   IFS= read -rd '' CATEGORY <<'TEXT' || true; CATEGORY=${CATEGORY%$'\n'}
   {its category, e.g. Authentication}
   TEXT
   BUILT_BY="{the phase that will build it, e.g. v1.0/03}"
   specforge new requirement "$REQ_ID" "$TEXT" --in spec/requirements.spec --text "$TEXT" --category "$CATEGORY" --tier v2 || exit 1
   specforge link "$BUILT_BY" requirements "$REQ_ID" || exit 1
   specforge set "$REQ_ID" tier v1 || exit 1
   ```

4. **Decisions to log?**
   - Read the decisions of the phase: `specforge query "$PHASE_ID" --depth 2 --kinds decision --format context`.
   - Record one that holds for the whole project as a project decision, linked into the project's `key_decisions`:

   ```bash
   DECISION_ID="{the next free project decision id, e.g. D-07}"
   IFS= read -rd '' DECISION <<'TEXT' || true; DECISION=${DECISION%$'\n'}
   {the decision}
   TEXT
   IFS= read -rd '' RATIONALE <<'TEXT' || true; RATIONALE=${RATIONALE%$'\n'}
   {why it was chosen, and its outcome if known}
   TEXT
   PROJECT=$(specforge export --kinds project --pick '.nodes[0].id') || exit 1
   [ -n "$PROJECT" ] || { echo "no project entity: run /gsd:new-project first" >&2; exit 1; }
   PROJECT_FILE=$(specforge query "$PROJECT" --depth 0 --pick '.nodes[0].file') || exit 1
   specforge new decision "$DECISION_ID" "$DECISION" --in "$PROJECT_FILE" --origin project --text "$DECISION" --rationale "$RATIONALE" --outcome pending || exit 1
   specforge link "$PROJECT" key_decisions "$DECISION_ID" || exit 1
   ```

5. **"What This Is" still accurate?**
   - If the product has meaningfully changed, update the description
   - Keep it current and accurate

   ```bash
   IFS= read -rd '' WHAT <<'TEXT' || true; WHAT=${WHAT%$'\n'}
   {what this is now: 2-3 sentences on what it does and who it is for, in the user's words}
   TEXT
   PROJECT=$(specforge export --kinds project --pick '.nodes[0].id') || exit 1
   [ -n "$PROJECT" ] || { echo "no project entity: run /gsd:new-project first" >&2; exit 1; }
   specforge set "$PROJECT" what "$WHAT" || exit 1
   ```

**Step complete when:**

- [ ] Phase summaries reviewed for learnings
- [ ] Each invalidated requirement is out of scope, its reason recorded
- [ ] Emerged requirements recorded in the phase that builds them
- [ ] New decisions logged with rationale
- [ ] "What This Is" updated if product changed

</step>

<step name="graduation_scan">

Scan LEARNINGS.md files from recent phases for recurring patterns and surface promotion candidates to the developer.

**Invoke the graduation helper:**

```text
@~/.claude/gsd-core/workflows/graduation.md
```

This step is fully delegated to `graduation.md`. It handles guard checks (feature flag, window size, threshold), clustering, filtering on each lesson's status, HITL prompting, promotion as project decisions, and each lesson's disposition.

**This step is always non-blocking:** graduation candidates are surfaced for the developer's decision; no action is required to continue the transition. If the graduation scan produces no qualifying clusters, it prints a single `[graduation: no qualifying clusters]` line and returns.

**Step complete when:**

- [ ] graduation.md guard checks passed (or skipped with silent no-op)
- [ ] Recurring clusters surfaced (or `[graduation: no qualifying clusters]` printed)
- [ ] Each cluster resolved as Promote / Defer / Dismiss (or all skipped)

</step>

<step name="review_accumulated_context">

Review the accumulated context: the open blockers and this phase's decisions, `specforge query "$MILESTONE" --depth 2 --kinds blocker,decision --format context`.

**Decisions:**

- Note recent decisions from this phase (3-5 max)
- One that holds for the whole project is recorded as a project decision in `evolve_project` (item 4)

**Blockers/Concerns:**

- Review blockers from completed phase
- One this phase addressed is resolved: `specforge gsd resolve-blocker <blocker> "<how it was resolved>"`
- One still relevant for the future stays open: its `phase` names where it came from
- A new concern from the completed phase's summaries is raised: `specforge gsd raise-blocker "<the concern>" --phase "$PHASE_ID"`

**Step complete when:**

- [ ] Recent decisions noted (project-wide ones recorded as project decisions)
- [ ] Addressed blockers resolved
- [ ] New concerns from completed phase raised as blockers

</step>

<step name="offer_next_phase">

**MANDATORY: Verify milestone status before presenting next steps.**

**Use the phases read in `update_roadmap_and_state`:**

- a phase after this one whose `status` is not `Complete` → More phases remain → Go to **Route A**
- none → Last phase done → **Check for workstream collisions first**

The next phase's `number` and `name` give you the next phase details.

If you need additional context, use:
```bash
ROADMAP=$(specforge query "$MILESTONE" --kinds phase --pick '.nodes[kind=phase]')   # one phase node per line: its number is the last segment of its id, roadmap_complete is fields.status "complete"
```

This returns all phases with goals, disk status, and completion info.

**Section-manifest gate (#2994):** `specforge` needs no setup — fetch the dedicated `init.transition` bundle for the workstream-collision-check gate below:

```bash

INIT_TRANSITION=$(specforge query "$PHASE_ID" --depth 1 --format json)

```

No init bundle prints a section manifest, so `section_manifest` is null and every section below is read; another active workstream is a `workstream` entity whose `status` is `active`.

---

<!-- gsd:section id="workstream-collision-check" when="state:workstream-active" -->
If `section_manifest` (from `INIT_TRANSITION`) is `null` or `"workstream-collision-check"` is in its `included` list: read and execute `gsd-core/workflows/transition/steps/workstream-collision-check.md`. Otherwise (flat mode) skip — do not read the file; go directly to **Route B**.
<!-- /gsd:section -->

---

**Route A: More phases remain in milestone**

The next phase's name is in the phases read above; its goal is the phase entity's `goal`.

**Check if next phase has CONTEXT.md:**

```bash
NEXT_PHASE="{next_phase}"   # the next phase's number as its id writes it (02, 02.1), from the phases read above
NEXT_CONTEXT=$(specforge query "$MILESTONE/$NEXT_PHASE" --depth 1 --kinds context --pick '.nodes[kind=context].id' --first 1)   # its context entity; empty when it has none
```

**If next phase exists:**

<if mode="yolo">

**If CONTEXT.md exists:**

```
Phase [X] marked complete.

Next: Phase [X+1] — [Name]

⚡ Auto-continuing: Plan Phase [X+1] in detail
```

Exit skill and invoke SlashCommand("/gsd:plan-phase [X+1] --auto")

**If CONTEXT.md does NOT exist:**

```
Phase [X] marked complete.

Next: Phase [X+1] — [Name]

⚡ Auto-continuing: Discuss Phase [X+1] first
```

Exit skill and invoke SlashCommand("/gsd:discuss-phase [X+1] --auto")

</if>

<if mode="interactive" AND="gates.confirm_transition != false">

**If CONTEXT.md does NOT exist:**

```
## ✓ Phase [X] Complete

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase [X+1]: [Name]** — [Goal from ROADMAP.md]

`/clear` then:

`/gsd:discuss-phase [X+1]` — gather context and clarify approach

---

**Also available:**
- `/gsd:plan-phase [X+1]` — skip discussion, plan directly
- `/gsd:plan-phase --research-phase [X+1]` — investigate unknowns

---
```

**If CONTEXT.md exists:**

```
## ✓ Phase [X] Complete

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase [X+1]: [Name]** — [Goal from ROADMAP.md]
<sub>✓ Context gathered, ready to plan</sub>

`/clear` then:

`/gsd:plan-phase [X+1]`

---

**Also available:**
- `/gsd:discuss-phase [X+1]` — revisit context
- `/gsd:plan-phase --research-phase [X+1]` — investigate unknowns

---
```

</if>

---

**Route B1: Workstream done, other workstreams still active**

This route is reached when this was the last phase AND the collision check found
other active workstreams. Do NOT suggest completing the milestone or advancing
to the next milestone — other workstreams are still working.

**Clear auto-advance chain flag** — workstream boundary is the natural stopping point:

```bash
: # workflow._auto_chain_active is not a setting here: the chain lasts as long as the conversation, so there is nothing to set or clear
```

<if mode="yolo">

Override auto-advance: do NOT auto-continue to milestone completion.
Present the blocking information and stop.

</if>

Present (all modes):

```
## ✓ Phase {X}: {Phase Name} Complete

This workstream's phases are complete. Other workstreams are still active:

| Workstream | Status | Phase | Progress |
|------------|--------|-------|----------|
| {name}     | {status} | {current_phase} | {completed_phases}/{phase_count} |
| ...        | ...    | ...   | ...      |

---

## Next Steps

Archive this workstream:

`/gsd:workstreams complete {current_ws_name}`

See overall milestone progress:

`/gsd:workstreams progress`

<sub>Milestone completion will be available once all workstreams finish.</sub>

---
```

Do NOT suggest `/gsd:complete-milestone` or `/gsd:new-milestone`.
Do NOT auto-invoke any further slash commands.

**Stop here.** The user must explicitly decide what to do next.

---

**Route B: All phases complete (milestone ready to close)**

**This route is only reached when:**
- this was the last phase AND no other active workstreams exist (or flat mode)

**Clear auto-advance chain flag** — milestone boundary is the natural stopping point:

```bash
: # workflow._auto_chain_active is not a setting here: the chain lasts as long as the conversation, so there is nothing to set or clear
```

<if mode="yolo">

```
Phase {X} marked complete.

🎉 Milestone {version} is 100% complete — all {N} phases finished!

⚡ Auto-continuing: Complete milestone and archive
```

Exit skill and invoke SlashCommand("/gsd:complete-milestone {version}")

</if>

<if mode="interactive" AND="gates.confirm_transition != false">

```
## ✓ Phase {X}: {Phase Name} Complete

🎉 Milestone {version} is 100% complete — all {N} phases finished!

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Complete Milestone {version}** — archive and prepare for next

`/clear` then:

`/gsd:complete-milestone {version}`

---

**Also available:**
- Review accomplishments before archiving

---
```

</if>

</step>

</process>

<implicit_tracking>
Progress tracking is IMPLICIT: planning phase N implies phases 1-(N-1) complete. No separate progress step—forward motion IS progress.
</implicit_tracking>

<partial_completion>

If user wants to move on but phase isn't fully complete:

```
Phase [X] has incomplete plans:
- {phase}-02-PLAN.md (not executed)
- {phase}-03-PLAN.md (not executed)

Options:
1. Mark complete anyway (plans weren't needed)
2. Defer work to later phase
3. Stay and finish current phase
```

Respect user judgment — they know if work matters.

**If marking complete with incomplete plans:**

- Nothing to update: the roadmap's plan count is computed, so it reads "2/3 plans complete" (not "3/3")
- Note in transition message which plans were skipped

</partial_completion>

<success_criteria>

Transition is complete when:

- [ ] Current phase plan summaries verified (all exist or user chose to skip)
- [ ] Any stale handoffs deleted
- [ ] The next phase read from the milestone's phases
- [ ] The project evolved with typed edits (requirements, decisions, description if needed)
- [ ] Blockers resolved or raised
- [ ] User knows next steps

</success_criteria>

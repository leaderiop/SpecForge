<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/resume-project.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<trigger>
Use this workflow when:
- Starting a new session on an existing project
- User says "continue", "what's next", "where were we", "resume"
- Any planning operation when the spec already exists
- User returns after time away from project
</trigger>

<purpose>
Instantly restore full project context so "Where were we?" has an immediate, complete answer.
</purpose>

<required_reading>
@~/.claude/gsd-core/references/continuation-format.md
</required_reading>

<process>

<step name="initialize">
Load all context in one call:

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the commands below are specforge commands: there is no launcher to define
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
INIT=$(specforge gsd progress-report --format json)
if [[ "$INIT" == @file:* ]]; then INIT=$(cat "${INIT#@file:}"); fi
```

Parse JSON for: `milestone_version`, `milestone_name`, `phases`, `percent`, `total_plans`, `total_summaries` (what `progress-report` prints). The routing facts below are not in it, they come from the graph: `state_exists` is `milestone_version` non-null (the project state is computed); `roadmap_exists` is `phases` non-empty; `project_exists` and `planning_exists` are `[ -n "$(specforge export --kinds project --pick '.nodes[].id')" ]`; `requirements_exists` is the same with `--kinds requirement`; `init_incomplete` is a project without a roadmap or without requirements; `has_interrupted_agent` is false (this extension keeps no agent registry).

**If `init_incomplete` is true (#4040 — interrupted bootstrap):** the spec exists but initialization never finished — the `requirement` entities, the `milestone` and `phase` entities or the project were never created. This is NOT a state-reconstruction case (the project state is computed). Route to initialization recovery: resume `/gsd:new-project`, which continues from the first missing entity and keeps the existing `project` entity and any already-created ones. Do not proceed to load_state.

**If `state_exists` is true:** Proceed to load_state
**If `state_exists` is false but `roadmap_exists` or `project_exists` is true (and `init_incomplete` is false):** Offer to reconstruct STATE.md
**If `planning_exists` is false:** This is a new project - route to /gsd:new-project
</step>

<step name="load_state">

Read and parse STATE.md, then PROJECT.md:

```bash
specforge gsd progress-report
specforge query "$PROJECT" --depth 0 --format context
```

**From STATE.md extract:**

- **Project Reference**: Core value and current focus
- **Current Position**: Phase X of Y, Plan A of B, Status
- **Progress**: Visual progress bar
- **Recent Decisions**: Key decisions affecting current work
- **Pending Todos**: Ideas captured during sessions
- **Blockers/Concerns**: Issues carried forward
- **Session Continuity**: Where we left off, any resume files

**From PROJECT.md extract:**

- **What This Is**: Current accurate description
- **Requirements**: Validated, Active, Out of Scope
- **Key Decisions**: Full decision log with outcomes
- **Constraints**: Hard limits on implementation

</step>

<step name="check_incomplete_work">
Look for incomplete work that needs attention:

```bash
# #2962: zsh aborts the block on an unmatched for-list glob (nomatch); bash passes it through. nullglob both.
shopt -s nullglob 2>/dev/null; setopt NULL_GLOB 2>/dev/null

# Check for structured handoff (preferred — machine-readable)
specforge export --kinds pause,handoff,blocker --format graph || true   # the pauses, their handoffs and the blockers, by kind (a project is linked to none of them)

# Pauses and handoffs are entities: the query above finds them wherever they sit.
# Entities, not files (#4805, #3689):

#  - the query reads them from the spec, so no shell glob is involved;



# Pauses are found by the query, not by searching files.
: # (there are no continue-here files: a handoff is an entity)



# (no async-job manifests: the external-job harness is not part of @gsd/core)



: # (there are no async-job manifests: the external-job harness is not part of @gsd/core)



# Check for plans without summaries (incomplete execution)
for plan in $(specforge query "$MILESTONE" --depth 3 --kinds plan,summary --pick '.nodes[kind=plan].id' | grep -vxF -f <(specforge query "$MILESTONE" --depth 3 --kinds plan,summary --pick '.nodes[kind=summary].fields.plan')); do   # each plan no summary names
  [ -n "$plan" ] || continue


  echo "Incomplete: $plan"
done 2>/dev/null || true

# Check for interrupted agents (use has_interrupted_agent and interrupted_agent_id from init)
if [ "$has_interrupted_agent" = "true" ]; then
  echo "Interrupted agent: $interrupted_agent_id"
fi
```

**If a `pause` entity exists:**

- This is the primary resumption source — the `handoff` entity from `/gsd:pause-work`
- Read its `state`, `current_state`, `task` and `next_action`
- Check its `blockers` — surface these immediately
- Check its `completed_work` for `in_progress` items — these need attention first
- Validate the uncommitted files in its `context` against `git status` — flag divergence
- Use `context` to restore mental model
- Flag: "Found a handoff — resuming at {next_action}"
- **After successful resumption, run `specforge gsd resume <pause>`** — it removes the pause and its handoff (a one-shot record)

**If a `pause` has no handoff:**

- This is a mid-plan resumption point
- Read the pause's `reason` and `plan`
- Flag: "Found mid-plan checkpoint"

**If PLAN without SUMMARY exists:**

- Execution was started but not completed
- Flag: "Found incomplete plan execution"

**If interrupted agent found:**

- Subagent was spawned but session ended before completion
- Read agent-history.json for task details
- Flag: "Found interrupted agent"
  </step>

<step name="present_status">
Present complete project status to user:

```
### PROJECT STATUS

Building: [one-liner from PROJECT.md "What This Is"]
Phase: [X] of [Y] - [Phase name]
Plan:  [A] of [B] - [Status]
Progress: [██████░░░░] XX%
Last activity: [date] - [what happened]

[If incomplete work found:]
⚠️  Incomplete work detected:
    - [pause entity or incomplete plan]

[If interrupted agent found:]
⚠️  Interrupted agent detected:
    Agent ID: [id]
    Task: [task description from agent-history.json]
    Interrupted: [timestamp]

    Resume with: Task tool (resume parameter with agent ID)

[If pending todos exist:]
📋 [N] pending todos — /gsd:capture --list to review

[If blockers exist:]
⚠️  Carried concerns:
    - [blocker 1]
    - [blocker 2]

[If alignment is not ✓:]
⚠️  Brief alignment: [status] - [assessment]
```

</step>

<step name="determine_next_action">
Based on project state, determine the most logical next action:

**An async-job manifest does not exist here** (the external-job harness is not part of @gsd/core): skip this branch.








**If interrupted agent exists:**
→ Primary: Resume interrupted agent (Task tool with resume parameter)
→ Option: Start fresh (abandon agent work)

**If a `pause` entity exists:**
→ Primary: Resume from the handoff (highest priority — specific task/blocker context), then `specforge gsd resume <pause>`
→ Option: Discard the handoff and reassess from the spec

**If a `pause` has no handoff:**
→ Fallback: Resume from checkpoint
→ Option: Start fresh on current plan

**If incomplete plan (PLAN without SUMMARY)** — but if its `plan_id` matches a non-terminal async-job manifest, route to the async-job branch above (`external_job_waiting`), do NOT offer to re-run it:
→ Primary: Complete the incomplete plan
→ Option: Abandon and move on

**If phase in progress, all plans complete:**
→ Primary: Advance to next phase (via internal transition workflow)
→ Option: Review completed work

**If phase ready to plan:**
→ Check if CONTEXT.md exists for this phase:

- If CONTEXT.md missing:
  → Primary: Discuss phase vision (how user imagines it working)
  → Secondary: Plan directly (skip context gathering)
- If CONTEXT.md exists:
  → Primary: Plan the phase
  → Option: Review roadmap

**If phase ready to execute:**
→ Primary: Execute next plan
→ Option: Review the plan first
</step>

<step name="offer_options">
Present contextual options based on project state:

```
What would you like to do?

[Primary action based on state - e.g.:]
1. Resume interrupted agent [if interrupted agent found]
   OR
1. Execute phase (/gsd:execute-phase {phase} ${GSD_WS})
   OR
1. Discuss Phase 3 context (/gsd:discuss-phase 3 ${GSD_WS}) [if CONTEXT.md missing]
   OR
1. Plan Phase 3 (/gsd:plan-phase 3 ${GSD_WS}) [if CONTEXT.md exists or discuss option declined]

[Secondary options:]
2. Review current phase status
3. Check pending todos ([N] pending)
4. Review brief alignment
5. Something else
```

**Note:** When offering phase planning, check for CONTEXT.md existence first:

```bash
specforge query "$MILESTONE/XX" --depth 2 --kinds context --pick '.nodes[kind=context].id' || true
```

If missing, suggest discuss-phase before plan. If exists, offer plan directly.

Wait for user selection.
</step>

<step name="route_to_workflow">
Based on user selection, route to appropriate workflow.

Resume-specific exception: do **not** emit `/clear then:` here. Resume is already a session-entry flow, so the next command should be shown directly.

- **Execute plan** → Show direct next command:
  ```
  ---

  ## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

  **{phase}-{plan}: [Plan Name]** — [objective from PLAN.md]

  `/gsd:execute-phase {phase} ${GSD_WS}`

  ---
  ```
- **Plan phase** → Show direct next command:
  ```
  ---

  ## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

  **Phase [N]: [Name]** — [Goal from ROADMAP.md]

  `/gsd:plan-phase [phase-number] ${GSD_WS}`

  ---

  **Also available:**
  - `/gsd:discuss-phase [N] ${GSD_WS}` — gather context first
  - `/gsd:plan-phase --research-phase [N] ${GSD_WS}` — investigate unknowns

  ---
  ```
- **Advance to next phase** → ./transition.md (internal workflow, invoked inline — NOT a user command)
- **Check todos** → Read the pending todos, present summary
- **Review alignment** → Read PROJECT.md, compare to current state
- **Something else** → Ask what they need
</step>

<step name="update_session">
The session continuity is computed from the spec: nothing is written before proceeding to the routed workflow.












</step>

</process>

<reconstruction>
The project state is computed from the spec, so there is nothing to reconstruct. If the spec holds no project, route to /gsd:new-project.
















  </reconstruction>

<quick_resume>
If user says "continue" or "go":
- Load state silently
- Determine primary action
- Execute immediately without presenting options

"Continuing from [state]... [action]"
</quick_resume>

<success_criteria>
Resume is complete when:

- [ ] Project state loaded
- [ ] Incomplete work detected and flagged
- [ ] Clear status presented to user
- [ ] Contextual next actions offered
- [ ] User knows exactly where project stands
- [ ] Session continuity: the pause is removed once resumed
      </success_criteria>

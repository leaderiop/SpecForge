<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/pause-work.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Create a `handoff` entity and a `pause` entity to preserve complete work state across sessions. The `handoff` holds the structured state and the human-readable context; `/gsd:resume-work` reads both.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<process>

<step name="detect">
## Context Detection

Determine what kind of work is being paused and set the handoff destination accordingly:

```bash
# Check for active phase
phase=$(specforge gsd progress-report --pick '.phases[].status' | grep -nx 'In Progress' | head -1 | cut -d: -f1); phase=${phase:+$(specforge gsd progress-report --pick ".phases[$((phase - 1))].number")}
phase=${phase:+$(basename "$(dirname "$phase")")}

# Check for active spike
spike=""   # spikes are not part of @gsd/core: there is none to detect
spike=${spike:+$(basename "$(dirname "$spike")")}

# Check for active sketch
sketch=""   # sketches are not part of @gsd/core: there is none to detect
sketch=${sketch:+$(basename "$(dirname "$sketch")")}

# Check for active deliberation
deliberation=""   # deliberations are not part of @gsd/core: there is none to detect
```

- **Phase work**: an active phase → the `handoff` and the `pause` are scoped to that phase
- **Spike work**: spikes are not part of @gsd/core → the project's handoff
- **Sketch work**: sketches are not part of @gsd/core → the project's handoff
- **Deliberation work**: deliberations are not part of @gsd/core → the project's handoff
- **Research work**: research notes exist but no phase → the project's handoff
- **Default**: no detectable context → the project's handoff, note the ambiguity in `<current_state>`

If phase is detected, proceed with phase handoff path. Otherwise use the first matching non-phase path above.
</step>

<step name="gather">
**Collect complete state for handoff:**

1. **Current position**: Which phase, which plan, which task
2. **Work completed**: What got done this session
3. **Work remaining**: What's left in current plan/phase
4. **Decisions made**: Key decisions and rationale
5. **Blockers/issues**: Anything stuck
6. **Human actions pending**: Things that need manual intervention (MCP setup, API keys, approvals, manual testing)
7. **Background processes**: Any running servers/watchers that were part of the workflow
8. **Files modified**: What's changed but not committed
9. **Outstanding async external jobs**: the external-job harness is not part of @gsd/core — skip this item.
10. **Blocking constraints**: Anti-patterns or methodological failures encountered during this session that a resuming agent MUST be aware of before proceeding. Only include items discovered through actual failure — not warnings or predictions. Assign each constraint a `severity`:
   - `blocking` — The resuming agent MUST demonstrate understanding before proceeding. The discuss-phase and execute-phase workflows will enforce a mandatory understanding check.
   - `advisory` — Important context but does not gate resumption.

Ask user for clarifications if needed via conversational questions.

**Also inspect SUMMARY.md files for false completions:**
```bash
# Check for placeholder content in existing summaries
for s in $(specforge query "$MILESTONE" --depth 3 --kinds summary --pick '.nodes[kind=summary].id'); do specforge query "$s" --depth 0 --pick '.nodes[0].fields.one_liner' | grep -Eq 'To be filled|placeholder|TBD' && echo "$s"; done || true
```
Report any summaries with placeholder content as incomplete items.
</step>

<step name="write_structured">
**Write the structured handoff as a `handoff` entity:**

```bash
# the commands below are specforge commands: there is no launcher to define
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
```

```bash
specforge new handoff HO1 "{context-name} paused at {X}/{Y}" --in spec/handoff.spec --phase "$MILESTONE/{phase_number}" --state in_progress --current_state "{where exactly are we}" --next_action "{specific first action when resuming}" --context "{mental state, approach, what you were thinking}"


































```

The `completed_work` blocks, `remaining_work`, `decisions_made` (references to `decision` entities) and `blockers` (references to `blocker` entities) of the handoff are authored through the MCP tool `specforge.edit`. The `pause` that follows names the handoff, so `/gsd:resume-work` finds both.

**The uncommitted files are measured, never asserted (#3968).** Put them in the handoff's `context`, from an actual call,
not from memory — a narrated `[]` over a dirty tree is how 14 plans' worth of uncommitted
code went invisible in the wild:
```bash
UNCOMMITTED=$(git status --porcelain)
# One array entry per line ("XY path"); truncate the list at 50 entries and note the
# elided count, but NEVER round it to empty — a non-empty porcelain output is the single
# most load-bearing fact a resume session needs.
```
</step>

<step name="write">
**The sections below are the shape of the handoff's `current_state` and `context` text** (the `handoff` entity holds them: there is no `.continue-here.md`):

```markdown
---
context: [phase|spike|sketch|deliberation|research|default]
phase: XX-name
task: 3
total_tasks: 7
status: in_progress
last_updated: [timestamp from date -u]
---

# BLOCKING CONSTRAINTS — Read Before Anything Else

> These are not suggestions. Each constraint below was discovered through failure.
> Acknowledge each one explicitly before proceeding.

- [ ] CONSTRAINT: [name] — [what it is] — [structural mitigation required]

**Do not proceed until all boxes are checked.**

_If no constraints have been identified yet, remove this section._

## Critical Anti-Patterns

| Pattern | Description | Severity | Prevention Mechanism |
|---------|-------------|----------|---------------------|
| [pattern name] | [what it is and how it manifested] | blocking | [structural step that prevents recurrence — not acknowledgment] |
| [pattern name] | [what it is and how it manifested] | advisory | [guidance for avoiding it] |

**Severity values:** `blocking` — resuming agent must pass understanding check before proceeding. `advisory` — important context, does not gate resumption.

_Remove rows that do not apply. The discuss-phase and execute-phase workflows parse this table and enforce a mandatory understanding check for any `blocking` rows._

<current_state>
[Where exactly are we? Immediate context]
</current_state>

<completed_work>

Completed Tasks:
- Task 1: [name] - Done
- Task 2: [name] - Done
- Task 3: [name] - In progress, [what's done]
</completed_work>

<remaining_work>

- Task 3: [what's left]
- Task 4: Not started
- Task 5: Not started
</remaining_work>

<decisions_made>

- Decided to use [X] because [reason]
- Chose [approach] over [alternative] because [reason]
</decisions_made>

<blockers>
- [Blocker 1]: [status/workaround]
</blockers>

## Required Reading (in order)
<!-- List documents the resuming agent must read before acting -->
1. [document] — [why it matters]
1. `METHODOLOGY.md` (if it exists) — project analytical lenses; apply before any assumption analysis

## Critical Anti-Patterns (do NOT repeat these)
<!-- Mistakes discovered this session that must be structurally avoided -->
- [ANTI-PATTERN]: [what it is] → [structural mitigation]

## Infrastructure State
<!-- Running services, external state, environment specifics -->
- [service/env]: [current state]

## Pre-Execution Critique Required
<!-- Fill in ONLY if pausing between design and execution (e.g. spike design done, not yet run) -->
- Design artifact: [path]
- Critique focus: [key questions the critic should probe]
- Gate: Do NOT begin execution until critique is complete and design is revised

<context>
[Mental state, what were you thinking, the plan]
</context>

<next_action>
Start with: [specific first action when resuming]
</next_action>
```

Be specific enough for a fresh Claude to understand immediately.

Use `date -u` for the `last_updated` field:
```bash
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
```
</step>

<step name="commit">
```bash
specforge gsd pause "[reason]" --by "[who]" --phase "$MILESTONE/[phase]" --handoff HO1 && git add spec && git commit -m "docs(pause): [context-name] paused at [X]/[Y]" && specforge collect --runner gsd-git --yes
```
</step>

<step name="confirm">
```
✓ Handoff created:
  - the `handoff` entity HO1 (structured and human-readable)
  - the `pause` entity PS1 (what a teammate who pulls the repository sees)

Current state:

- Context: [phase|spike|deliberation|research]
- Location: [XX-name or SPIKE-NNN]
- Task: [X] of [Y]
- Status: [in_progress/blocked]
- Blockers: [count] ({human_actions_pending count} need human action)
- Committed handoff

To resume: /gsd:resume-work

```
</step>

</process>

<success_criteria>
- [ ] Context detected (phase/spike/deliberation/research/default)
- [ ] Handoff entity created in the scope of the detected context
- [ ] Required Reading, Anti-Patterns, and Infrastructure State sections filled
- [ ] Pre-Execution Critique section filled if pausing between design and execution
- [ ] Committed handoff
- [ ] User knows location and how to resume
</success_criteria>

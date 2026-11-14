<!-- Ported from gsd-core v1.19.0 gsd-core/references/revision-loop.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Revision Loop Pattern

Standard pattern for iterative agent revision with feedback. Used when a checker/validator finds issues and the producing agent needs to revise its output.

---

## Pattern: Check-Revise-Escalate (max 3 iterations)

This pattern applies whenever:
1. An agent produces output (plans, imports, gap-closure plans)
2. A checker/validator evaluates that output
3. Issues are found that need revision

### Flow

```
prev_issue_count = Infinity
iteration = 0
previous_conflict_property = null
conflict_return_count = 0

LOOP:
  1. Run checker/validator on current output
  2. Read checker results
  3. If PASSED or only INFO-level issues:
     -> Accept output, exit loop
  4. If BLOCKER or WARNING issues found:
     a. If iteration + 1 > 3:
        -> Escalate to user (see "After 3 Iterations" below)
     b. Parse issue count from checker output
     c. If issue_count >= prev_issue_count:
        -> Escalate to user: "Revision loop stalled (issue count not decreasing)"
     d. prev_issue_count = issue_count
     e. Re-spawn the producing agent with checker feedback appended
     f. If the agent returns REVISION_CONFLICT:
        -> conflict_return_count += 1
        -> If conflict_return_count >= 3:
             escalate through the iteration-cap gate
        -> If it names the same required_property as the previous conflict:
             escalate as a stall (the resolution did not take)
           Else: previous_conflict_property = current required_property
             resolve it (see "Conflict Return" below) and go to step e.
             Do NOT increment iteration -- the conflict was not a failed attempt.
        Else: previous_conflict_property = null (a normal revision ends the conflict chain --
             a LATER, unrelated conflict on the same property must not be misread as a repeat)
     g. iteration += 1
     h. After revision completes, go to LOOP

The increment is step g, AFTER the producing agent returns. An iteration counted at step a is
already spent by the time a REVISION_CONFLICT comes back, so it cannot then be withheld, and the
cap would punish the agent for correctly refusing to apply incompatible advice.
```

### Issue Count Tracking

Track the number of BLOCKER + WARNING issues returned by the checker on each iteration. If the count does not decrease between consecutive iterations, the producing agent is stuck and further iterations will not help. Break early and escalate to the user.

Display iteration progress before each revision spawn:
`Revision iteration {N}/3 -- {blocker_count} blockers, {warning_count} warnings`

### Re-spawn Prompt Structure

When re-spawning the producing agent for revision, pass the checker's YAML-formatted issues. The checker's output contains a `## Issues` heading followed by a YAML block. Parse this block and pass it verbatim to the revision agent.

The field names are the plan-checker's schema (`agents/gsd-plan-checker.md` → `<issue_structure>`):
`plan`, `dimension`, `severity`, `required_property`, `description`, `task`, `fix_hint`. There is no
`suggested_fix` field and no `finding` or `affected_field` field — those names were drift, and every
producer now emits the schema above.

```
<checker_issues>
The issues below are in YAML format. Each has: dimension, severity,
required_property, description, fix_hint.

BINDING: required_property (the invariant that must hold), description (the
evidence it does not), severity. NON-BINDING: fix_hint -- ONE example route to
the property, never an instruction.

Satisfy the required_property of ALL BLOCKER issues. Satisfy WARNING issues
where feasible.

{YAML issues block from checker output -- passed verbatim}
</checker_issues>

<revision_instructions>
Address ALL BLOCKER and WARNING issues identified above.
- For each BLOCKER: make required_property true. Its fix_hint is one example
  route; a smaller or different mechanism that makes the same property true
  addresses the issue in full -- report which mechanism you used.
- For each WARNING: address or explain why it's acceptable
- Before editing, re-check locked decisions, active capability guidance, and
  constraints the existing output already encodes. If a fix_hint would
  contradict one of those, or the property is unreachable without breaking one,
  do NOT apply it and do NOT work around it: return REVISION_CONFLICT naming
  the conflict and the alternatives considered, having addressed every
  non-conflicting issue.
- Do NOT introduce new issues while fixing existing ones
- Preserve all content not flagged by the checker
This is revision iteration {N} of max 3. Previous iteration had {prev_count}
issues. You must reduce the count or the loop will terminate.
</revision_instructions>
```

### Conflict Return (REVISION_CONFLICT)

A revision agent that returns `REVISION_CONFLICT` has not failed and has not stalled. Handle it
BEFORE the iteration counter and the stall check — a conflict is not resolvable by re-running the
same loop, so spending retry budget on it only exhausts the cap:

**This protocol is shared.** Every revision-bearing workflow follows it — `plan-phase`, `quick`,
`ui-phase`, and `verify-work`'s gap-plan loop. `plan-phase` @-imports this reference and states
only its own bindings (counter name, artifact path, next step). The other three do not import it,
so they restate the operative rules inline; this section is the authority they must agree with.

1. **Do not spend budget.** Do NOT increment the iteration counter and do NOT update
   `prev_issue_count`. Do NOT re-spawn the checker yet — the conflict is not a revised output.
2. **Record** the conflict as a `revision_conflict` of the plan's phase, open until it is resolved:

```bash
PLAN_ID="{the plan's id, e.g. v1.0/01/01-02}"
IFS= read -rd '' DIMENSION <<'TEXT' || true; DIMENSION=${DIMENSION%$'\n'}
{the dimension the conflict blocks, as the agent returned it}
TEXT
IFS= read -rd '' PROPERTY <<'TEXT' || true; PROPERTY=${PROPERTY%$'\n'}
{the property the revision cannot reach}
TEXT
IFS= read -rd '' CONFLICTS_WITH <<'TEXT' || true; CONFLICTS_WITH=${CONFLICTS_WITH%$'\n'}
{what it conflicts with}
TEXT
IFS= read -rd '' ALTERNATIVES <<'TEXT' || true; ALTERNATIVES=${ALTERNATIVES%$'\n'}
{the agent's alternatives; empty when it named none}
TEXT
specforge gsd record-conflict "$PLAN_ID" --dimension "$DIMENSION" --property "$PROPERTY" --conflicts-with "$CONFLICTS_WITH" --alternatives "$ALTERNATIVES" || exit 1
```

   `CONFLICTS_WITH` names the locked decision D-nn, CLAUDE.md rule or plan constraint, and
   `ALTERNATIVES` the agent's alternatives (an empty heredoc when it named none: the conflict then records none). The
   same open conflict is reused, not recorded twice, and each value is a command argument, so
   agent-authored text cannot forge a second conflict. An open conflict blocks convergence even if
   this run is abandoned. A workflow with no phase (`quick`) skips this step.
3. **Resolve** — present the conflict and its alternatives to the user and ask which to take
   (pattern: `gsd-core/references/gate-prompts.md`): adopt a named alternative / override the
   named constraint and apply the hint / amend the constraint itself. Each option resolves the
   conflict. Accepting the output with the blocker still open is NOT offered here — the blocking
   `required_property` still fails, and that choice belongs to the cap escalation.
4. **Close** — the workflow that recorded the conflict resolves it once the resolution has been
   applied, with `specforge gsd resolve-conflict "<chosen resolution>" --conflict <conflict>`
   (`--plan <plan>` resolves every open conflict of the plan). Readers only read. A conflict left open
   is a live blocker, never a stale artifact.
5. **Re-spawn** with the chosen resolution, then re-evaluate the return from the top of this
   handler — never fall through to the checker spawn. A second conflict is still a conflict, not
   a revised output, and handing it to the checker would check the conflict message.

**Bounded — two ways, because one is evadable.** Not incrementing must not make this path
unbounded:

- **Repeat.** A conflict naming the SAME `required_property` twice in a row means the chosen
  resolution did not take. Stop re-spawning; escalate as a stall.
- **Total.** Count every conflict return in this revision loop, whatever property each names. On
  the THIRD, stop and escalate — an agent that alternates property names never trips the repeat
  rule, so the repeat rule alone leaves the loop unbounded. This total is what actually bounds the
  path; the repeat rule just catches the common case sooner.

Both escalate through the same gate the iteration cap uses. A conflict still never consumes a
revision iteration — the cap on conflicts is separate from, and additional to, the cap on
revisions.

**No workflow hands a conflict to a loop and returns.** Asking the user is the route everywhere;
recording is in addition to asking, never instead of it. `plan-phase` in particular never invokes
`/gsd:plan-review-convergence` — it runs *inside* that loop, so invoking it would be a cycle, and
"was I invoked by convergence?" is not a question the orchestrator can answer at runtime.

### After 3 Iterations

If issues persist after 3 revision cycles:

1. Present remaining issues to the user
2. Use gate prompt (pattern: yes-no from `gsd-core/references/gate-prompts.md`):
   question: "Issues remain after 3 revision attempts. Proceed with current output?"
   header: "Proceed?"
   options:
     - label: "Proceed anyway"   description: "Accept output with remaining issues"
     - label: "Adjust approach"  description: "Discuss a different approach"
3. If "Proceed anyway": accept current output and continue
4. If "Adjust approach" or "Other": discuss with user, then re-enter the producing step with updated context

### Workflow-Specific Variations

| Workflow | Producer Agent | Checker Agent | Notes |
|----------|---------------|---------------|-------|
| plan-phase | gsd-planner | gsd-plan-checker | Revision prompt via planner-revision.md |
| execute-phase | gsd-executor | gsd-verifier | Post-execution verification |
| discuss-phase | orchestrator | gsd-plan-checker | Inline revision by orchestrator |

---

## Important Notes

- **INFO-level issues are always acceptable** -- they don't trigger revision
- **Each iteration gets a fresh agent spawn** -- don't try to continue in the same context
- **Checker feedback must be inlined** -- the revision agent needs to see exactly what failed
- **Don't silently swallow issues** -- always present the final state to the user after exiting the loop
- **A remediation hint is an example, not an order** -- an issue satisfied through a smaller valid
  mechanism is addressed, and counts as resolved for the issue-count and stall checks

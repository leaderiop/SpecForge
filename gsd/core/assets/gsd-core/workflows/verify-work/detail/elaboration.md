<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/verify-work/detail/elaboration.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# verify-work.md — deferred elaboration

Read in full when `workflow.compact_content` is `false` (the default) — see
`gsd-core/references/compact-content-gate.md` for the check and resolution rule this
spine defers to. Each `§` below is the full text the spine condenses at the point it
names.

## § 1 — reconcile_gaps

**Gap-closure state is computed (#1921):**

When verify-work resumes after `/gsd:execute-phase --gaps-only`, a gap whose fix plan has executed already reads resolved: no step re-diagnoses it as a fresh blocker or spawns a new gap plan for it, and the verification state is kept. For each `uat_gap` of the phase:
1. A plan whose `gap_closure` names the gap, once complete, resolves it: the gap's computed `status` reads `resolved`, `resolved_by` names that plan and `resolved_at` its summary's completion. Edit nothing.
2. A gap no plan names, or whose plan has not completed, reads `status: open` (still open).

`reconcile_gaps`' line prints the open gaps; the resolved ones are the phase's other gaps. Announce:
```
Reconciled gap-closure state: {resolved_count} gap(s) resolved by executed plans, {open_count} still open.
```

Resolved gaps are NOT re-diagnosed and do NOT spawn new gap plans. A later report of the same behavior as still broken is a new issue (a regression) with a new `uat_gap`.

## § 2 — resume_from_file

**Resume testing from the session:**

**First run `reconcile_gaps`** (above): gaps already fixed by `/gsd:execute-phase --gaps-only` read `resolved` before testing resumes (#1921).

Read the session's tests with `present_test`'s lines.

(The find-pending / zero-pending guard clause is stated verbatim in the spine — a pre-existing
drift guard, `tests/verify-work-auto-transition.test.cjs` bug #1716, requires the two sentences
adjacent with nothing between them.)

Announce:
```
Resuming: Phase {phase} UAT
Progress: {passed + issues + skipped}/{total}
Issues found so far: {issues count}

Continuing from Test {N}...
```

The pending test is not recorded anywhere (#5105 C2): only its answer is, so a resume that finds a pending test and stops without its answer writes nothing.
Then continue to `present_test` with it.

## § 3 — diagnose_issues, plan_gap_closure, verify_gap_plans, revision_loop (the gap-closure sub-flow)

This whole sub-flow only runs when UAT testing found UNRESOLVED issues (`complete_session` routes
here based on `unresolved_issues`, not the raw `issues` count — #4983, see § 4); a session with
zero unresolved issues never reaches it, including a resumed session whose issues were all
verified-resolved by an executed gap-closure plan.

### diagnose_issues

**Diagnose root causes before planning fixes:**

```
---

{N} issues found. Diagnosing root causes...

Spawning parallel debug agents to investigate each issue.
```

- Load diagnose-issues workflow
- Follow @~/.claude/gsd-core/workflows/diagnose-issues.md
- Spawn parallel debug agents for each issue
- Collect root causes
- Record each root cause on its `uat_gap` (diagnose-issues' fence)
- Proceed to `plan_gap_closure`

Diagnosis runs automatically - no user prompt. Parallel agents investigate simultaneously, so overhead is minimal and fixes are more accurate.

### plan_gap_closure

**Auto-plan fixes from diagnosed gaps:**

Display:
```
### GSD ► PLANNING FIXES

◆ Spawning planner for gap closure... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

Spawn gsd-planner in --gaps mode:

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`planner_model`, `checker_model`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

````
Agent(
  prompt="""
<planning_context>

**Phase:** {phase_number}
**Mode:** gap_closure

<required_reading>
specforge context gsd/planner "{PHASE_ID}"
specforge query {PHASE_ID} --depth 3 --kinds uat_session,uat_test,uat_gap --format context
Run the lines above first: the planner pack, with the project state and the roadmap among its sections, then the UAT with its diagnoses.
</required_reading>

${AGENT_SKILLS_PLANNER}

</planning_context>

<downstream_consumer>
Output consumed by /gsd:execute-phase
Plans must be executable prompts.

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

**Gap linkage (#1921):** each created plan MUST name the `uat_gap` entities it addresses in its `gap_closure` field. A gap is then resolved once that plan completes: its `status` is computed, so `/gsd:verify-work` never re-diagnoses it as a fresh blocker.
</downstream_consumer>
""",
  subagent_type="gsd-planner",
  model="{planner_model}",
  description="Plan gap fixes for Phase {phase}"
)
````

(The "stop working, wait for the subagent" orchestrator rule is stated in the spine, not repeated here.)

On return:
- **PLANNING COMPLETE:** Proceed to `verify_gap_plans`
- **PLANNING INCONCLUSIVE:** Report and offer manual intervention

### verify_gap_plans

**Verify fix plans with checker:**

Display:
```
### GSD ► VERIFYING FIX PLANS

◆ Spawning plan checker... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

Initialize: `iteration_count = 1`

Spawn gsd-plan-checker:

```
Agent(
  prompt="""
<verification_context>

**Phase:** {phase_number}
**Phase Goal:** Close diagnosed gaps from UAT

<required_reading>
- `specforge context gsd/plan-checker <id>` for each fix plan the planner created (each plan's check pack: the plan, its tasks and must-haves, its phase, requirements and decisions)
</required_reading>

${AGENT_SKILLS_CHECKER}

</verification_context>

<expected_output>
Return one of:
- ## VERIFICATION PASSED — all checks pass
- ## ISSUES FOUND — structured issue list
</expected_output>
""",
  subagent_type="gsd-plan-checker",
  model="{checker_model}",
  description="Verify Phase {phase} fix plans"
)
```

(The "stop working, wait for the subagent" orchestrator rule, and the on-return handling for
VERIFICATION PASSED / ISSUES FOUND, are stated in the spine — the ISSUES FOUND handler's exact
wording is pinned by `tests/plan-checker-coupling.test.cjs`.)

### revision_loop

The full conflict-handling contract (non-binding `fix_hint`, the BEFORE-editing constraint
re-check, `## REVISION_CONFLICT` routing, the same-property/THIRD-conflict stall bound, and the
max-iteration escalation) is stated verbatim in the spine — a pre-existing drift guard
(`tests/revision-remediation-binding.test.cjs`, #3771) pins it there across every revision
orchestrator in the repo. This section adds only the surrounding `Agent()` scaffolding:

```
Agent(
  prompt="""
<revision_context>

**Phase:** {phase_number}
**Mode:** revision

<required_reading>
- `specforge context gsd/planner "{PHASE_ID}"` (the planner pack: the existing plans among its sections)
</required_reading>

${AGENT_SKILLS_PLANNER}

**Checker issues:**
{structured_issues_from_checker}

</revision_context>

<instructions>
The plans are in the planner pack. Make targeted updates to address checker issues. (See the spine for
the binding/non-binding contract and REVISION_CONFLICT handling stated above this point.)
</instructions>
""",
  subagent_type="gsd-planner",
  model="{planner_model}",
  description="Revise Phase {phase} plans"
)
```

## § 4 — complete_session's `unresolved_issues` count (#4983)

**Why this exists:** a test's `result: issue` is the historical record of what the person reported, and it stays `issue` after an executed gap-closure plan resolves the test's gap. Counting raw `result: issue` tests would route a resumed session whose issues are ALL resolved straight back into diagnosis, and the gap-closure sub-flow (§ 3) has no path back to promotion: the sibling of #4546 (a deliberately deferred `skipped` test) for the `issue`-plus-resolved case.

**`unresolved_issues` definition:** a test counts toward `unresolved_issues` when its `result` is `issue` and it is still open: it has no `uat_gap`, or one of its gaps reads `status: open`. A gap reads `resolved` only when a plan whose `gap_closure` names it has completed (its `resolved_by` and `resolved_at` are computed from that plan and its summary), so a gap cannot be marked resolved by hand. It is the rule `specforge gsd verification-debt` applies: the `blockers` it lists for the phase are its open tests, and the open ones whose `result` is `issue` are the `unresolved_issues`. A test with a later regression (a second `uat_gap`, per § 1) counts as resolved only when EVERY gap of the test is resolved: one open regression is enough to keep the test unresolved.

**Where this plugs in:** `complete_session`'s "Count results" step computes `unresolved_issues` alongside `pending_count` and `blocked_count`, and its issues-routing decision (`diagnose_issues` vs. proceeding toward promotion) reads `unresolved_issues`, not the raw `issues` count of the session's `totals`. The raw `issues` count is unchanged and still drives the presented summary numbers.

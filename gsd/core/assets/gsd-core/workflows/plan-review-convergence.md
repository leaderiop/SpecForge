<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-review-convergence.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Cross-AI plan convergence loop — automates the manual chain:
gsd-plan-phase N → gsd-review N --codex → gsd-plan-phase N --reviews → gsd-review N --codex → ...
Plan-phase runs inline (bare Skill at depth 0) so it can spawn gsd-planner/gsd-plan-checker at depth 1.
Review runs inside an isolated Agent (leaf skill — Bash only, no sub-agents needed).
Orchestrator only does: init, loop control, parse CYCLE_SUMMARY for HIGH and actionable non-HIGH counts, stall detection, escalation.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.

@$HOME/.claude/gsd-core/references/revision-loop.md
@$HOME/.claude/gsd-core/references/gates.md
@$HOME/.claude/gsd-core/references/agent-contracts.md
</required_reading>

<process>

## 1. Parse and Normalize Arguments

The host has parsed the phase, the reviewer flags (the declared reviewer lane flags, plus `--all`), `--max-cycles N`, `--text` and `--ws`; the lines below read them.

```bash

# #4748: canonical phase grammar (digits, optional [A-Z], dotted segments) —
# `12A` / `23.1.2` extract whole instead of truncating to `12` / `23.1`.
PHASE="{{arg:phase|}}"

# #2315: do NOT default REVIEWER_FLAGS to --codex here. The default is resolved
# against review.default_reviewers in step 1.5 (after the config gate) so a bare
# invocation respects the configured reviewer lineup per ADR-0011 / ADR-0015.

MAX_CYCLES="{{arg:max_cycles|3}}"

```

## 1.5. Config Gate (feature disabled by default)

An explicit dispatch from `/gsd:autonomous --converge` carries `--override-gate` (#4600): the
operator's explicit flag overrides this gate for that run. Standalone invocation carries no such
flag and remains gated by the config below.

```bash

# the commands below are specforge commands: there is no launcher to define
# --override-gate is appended ONLY by the autonomous --converge dispatch (#4600); it must be
# it is a declared flag the host parses, so no other argument can carry it in.
if [ "{{arg:override_gate|false}}" = true ]; then
  CONVERGENCE_ENABLED="override"
else
  CONVERGENCE_ENABLED="{{setting:workflow.plan_review_convergence|false}}"   # workflow.plan_review_convergence (the settings)
fi
```

**If `CONVERGENCE_ENABLED` is `"override"`:** note it and continue — "Convergence was requested
explicitly (`--converge` on the autonomous command); the `workflow.plan_review_convergence` gate
is bypassed for this run (#4600)."

**If `CONVERGENCE_ENABLED` is not `"true"`:** Display and exit:

```text
gsd-plan-review-convergence is disabled (workflow.plan_review_convergence=false).

This feature automates the plan→review→replan loop using external AI reviewers.
Enable it with:

  gsd config-set workflow.plan_review_convergence true

Then re-run: /gsd:plan-review-convergence {PHASE}
```

```bash

# Reviewer flags are DERIVED from the declared lane roster (#2800/#2272), never hand-listed.
# Three surfaces used to enumerate them independently and had drifted: --coderabbit was missing
# from all three, --qwen/--cursor/--kimi-code from this one, and the old unanchored
# `grep -q '\-\-agy'` matched INSIDE --antigravity, appending both for one user flag.
# `--all` is a selection control, not a lane, so it stays literal.
# This block reads the arguments only; the commands below are specforge commands —
# do not move it back above the preamble in a future edit.
REVIEWER_FLAGS=""
[ "{{arg:claude|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --claude"; [ "{{arg:codex|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --codex"; [ "{{arg:coderabbit|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --coderabbit"
[ "{{arg:opencode|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --opencode"; [ "{{arg:qwen|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --qwen"; [ "{{arg:cursor|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --cursor"
[ "{{arg:antigravity|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --antigravity"; [ "{{arg:agy|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --agy"; [ "{{arg:ollama|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --ollama"
[ "{{arg:lm_studio|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --lm-studio"; [ "{{arg:llama_cpp|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --llama-cpp"
[ "{{arg:kimi_code|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --kimi-code"; [ "{{arg:all|false}}" = true ] && REVIEWER_FLAGS="$REVIEWER_FLAGS --all"

# #2315: Resolve reviewer selection when no explicit flag was given.
# The pre-fix bug unconditionally set REVIEWER_FLAGS="--codex" in step 1, BEFORE
# the config gate — silently overriding any configured review.default_reviewers
# (and, transitively, review.reviewer_instances). gsd-review sees the injected
# --codex as an explicit flag (precedence rule 1) and never reaches rule 3
# (review.default_reviewers). ADR-0011 and ADR-0015 both assume convergence
# respects review.default_reviewers on the no-flag path.
#
# After the fix: leave REVIEWER_FLAGS empty when default_reviewers is configured
# so gsd-review applies review.default_reviewers itself (rule 3). Only fall back
# to --codex when no default is configured, preserving the pre-fix default for
# unconfigured users (#2315 AC3). REVIEWER_DISPLAY mirrors the resolved value
# so the startup banner reflects what will actually run (#2315 AC4).
if [ -z "$REVIEWER_FLAGS" ]; then

  DEFAULT_REVIEWERS=$(specforge settings --extension @gsd/core --pick '.extensions[0].settings[name=review.default_reviewers].value[]')   # one slug per line

    if [ -n "$DEFAULT_REVIEWERS" ]; then
      : # leave REVIEWER_FLAGS empty — gsd-review applies review.default_reviewers itself
      REVIEWER_DISPLAY="review.default_reviewers ($(specforge settings --extension @gsd/core --pick '.extensions[0].settings[name=review.default_reviewers].value[]' --join ', '))"
    else
      REVIEWER_FLAGS="--codex"
      REVIEWER_DISPLAY="--codex (default; configure review.default_reviewers to change)"
    fi

else
  # Strip the leading space accumulated by the parse block so the banner renders
  # "Reviewers: --codex" not "Reviewers:  --codex" (#2315 review nit).
  REVIEWER_DISPLAY="${REVIEWER_FLAGS# }"
fi
```

## 2. Initialize

```bash
INIT=$(specforge query "$PHASE_ID" --depth 1 --format json)

```

Read from the answer: the node whose `id` is `$PHASE_ID` is the phase (its number, two digits, is the last segment of that id; `name` is its field; a phase not in the graph is an E003 error), and its `plan` entities are among the other nodes (`plan_count` is their number; `has_plans` is whether there is one). `commit_docs`, `workflow.text_mode` and `response_language` are settings: `specforge settings --format json --extension @gsd/core`.

**If `response_language` is set:** All user-facing output — narration between tool calls, status updates, progress notes, findings, questions, and report prose — should be in `{response_language}`.

Set `TEXT_MODE=true` if `--text` is present in $ARGUMENTS OR `text_mode` from init JSON is `true`. When `TEXT_MODE` is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number.

## 3. Validate Phase + Pre-flight Gate

```bash
PHASE_INFO=$(specforge query "$PHASE_ID" --depth 0 --pick '.nodes[0]')   # the phase node, compact: found when the query does not refuse it, phase_number is the last segment of its id, the rest are its fields
```

**If `found` is false:** Error with available phases. Exit.

Display startup banner:

```text
### GSD ► PLAN CONVERGENCE — Phase {phase_number}

 Reviewers: {REVIEWER_DISPLAY}
 Max cycles: {MAX_CYCLES}
```

## 4. Initial Planning (if no plans exist)

**If `has_plans` is true:** Skip to step 5. Display: `Plans found: {plan_count} PLAN.md files — skipping initial planning.`

**If `has_plans` is false:**

Display: `◆ No plans found — running initial planning inline... (plan-phase runs here in the orchestrator — no output until planning is complete, ~1–5 min; expected, not a freeze)`

```text
Skill(skill="gsd-plan-phase", args="{PHASE}")
```

Run plan-phase **inline** (do NOT wrap it in Agent()). The convergence orchestrator runs at depth 0 with Agent available, so inline plan-phase can spawn gsd-planner and gsd-plan-checker at depth 1 — the one level of nesting that works on Claude Code. Wrapping plan-phase in Agent() would push it to depth 1 where the Agent tool is absent, preventing it from spawning any sub-agents. Wait until plan-phase completes and the plans are committed before continuing.

After plan-phase completes, verify plans were created. This asks "did initial
planning write files to disk" — a planner-produced-nothing check, not
outstanding-work counting — so it takes every plan of the phase (#3218), the progress
report of this phase alone, by its id:
```bash
PLAN_COUNT=$(specforge gsd progress-report --phase "$PHASE_ID" --pick '.phases[0].plans')   # every plan of the phase: a plan has no superseded status
```

If PLAN_COUNT == 0: Error — initial planning failed. Exit.

Display: `Initial planning complete: ${PLAN_COUNT} plans written.`

## 5. Convergence Loop

Initialize loop variables:

```text
cycle = 0
prev_unresolved_count = Infinity
```

### 5a. Review (Spawn Agent)

Increment `cycle`.

Display: `◆ Cycle {cycle}/{MAX_CYCLES} — spawning review agent... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)`

```text
Agent(
  description="Cross-AI review Phase {PHASE} cycle {cycle}",
  prompt="Run /gsd:review for Phase {PHASE}.

Execute: Skill(skill='gsd-review', args='--phase {PHASE} {REVIEWER_FLAGS}')

Complete the full review workflow. Do NOT return until the review round is recorded and committed.

IMPORTANT — recording contract (required):
The round you record holds this cycle's CURRENT concerns: convergence counts its open HIGH and open
actionable concerns from the graph (`open.high`, `open.actionable` of the round), never from your
return message. Each concern that remains unresolved is an item of the round's `concerns`.

Consensus gate (applies to NEWLY RAISED HIGHs only; evaluate before the recording rules below):
  This gate engages ONLY when 2 or more reviewers actually ran and produced a review this
  cycle. With exactly one reviewer, skip this entire gate — a single reviewer's HIGH always counts,
  exactly as before.

  Classify each newly raised HIGH by what the claim ASSERTS, not by whether it happens to contain a
  file:line citation:
    - EXISTENCE-CLASS — asserts that a named symbol, file, path, flag, commit, or ID exists,
      is absent, or says something specific ("X does not exist", "the plan cites Y which is missing",
      "file Z contains Q").
    - JUDGMENT-CLASS — asserts a design or correctness property ("no idempotency on retried writes",
      "race between A and B", "missing rate limit"). A judgment-class HIGH stays judgment-class even
      when it cites a file for context.

  A HIGH raised by 2+ reviewers is corroborated and always counts.

  For a HIGH raised by exactly ONE reviewer:
    - EXISTENCE-CLASS — counts only if the source-grounding pass independently confirms it against
      real project source, or another reviewer raised the same or a materially overlapping concern
      (i.e. it is one of the round's agreed concerns).
    - JUDGMENT-CLASS — counts UNLESS that reviewer's own section OPENS with an evidence-quality
      discount marker blockquote: `[reviewed-without-source-citations]` or
      `[reviewed-without-repo-access]`, or the reviewer is a diff-only lane (CodeRabbit). The marker
      must be the LEADING blockquote of that reviewer's section — a review that merely quotes a
      marker while discussing it is NOT marked. Corroboration by another reviewer overrides the
      marker and the HIGH counts.

  Judgment-class findings are deliberately NOT subject to corroboration. Different reviewers catch
  materially different classes of issue, so requiring two of them to independently raise the same
  architectural concern would suppress exactly the findings a multi-reviewer setup exists to surface.

  FAIL OPEN: if EVERY reviewer that ran this cycle carries a discount marker, this gate does not
  apply at all — count as if it were absent. A gate must never manufacture convergence out of a
  cycle in which nothing was verified.

  A HIGH suppressed by this gate is still recorded, its text tagged `(single-reviewer, unconfirmed)`,
  with severity `medium` and `actionable` false: counted in neither, never silently dropped, and
  never removed from the round.

  This gate governs HIGHs only. Actionable non-HIGH concerns are unaffected.

Recording rules:
  RECORD as a concern of this round:
    - Newly raised HIGHs in this cycle (subject to the consensus gate above)
    - PARTIALLY RESOLVED HIGHs: concern acknowledged and a mitigation is in progress, but not yet verified/completed
    - Previously raised HIGHs that are still unresolved
    - Actionable MEDIUM/LOW concerns, with `actionable` true

  DO NOT RECORD as a concern:
    - FULLY RESOLVED HIGHs: concern addressed with verification complete (closed ticket, verification log, or reviewer sign-off)
    - HIGH mentions in retrospective/summary tables comparing cycles
    - Quoted excerpts from prior reviews referencing past HIGH items

  A MEDIUM/LOW concern already incorporated into a plan's task, action, acceptance criteria, verify command, truth, threat, artifact, or a deferral disposition is recorded with `actionable` false.

Definitions:
  PARTIALLY RESOLVED — concern acknowledged and mitigation is in progress but not yet verified/completed (e.g., open ticket exists but fix not landed).
  FULLY RESOLVED — concern addressed with verification complete (closed ticket, verification log, or explicit reviewer sign-off confirming closure).
  ACTIONABLE — a non-HIGH review finding that would be invisible to /gsd:execute-phase unless a plan incorporates it or a disposition defers it.

Your final response names the review round you recorded.",
  mode="auto"
)
```

### Source-grounding pass (config: `plan_review.source_grounding`, default on)

Run this pass unless `plan_review.source_grounding` is `false`. It verifies every symbol the plan cites against the project source before approval, catching hallucinated symbols at review time instead of execution time.

1. **Enumerate cited symbols.** List every referenced symbol by kind, quoting the plan line for each (coverage must be auditable): decorators (`@name`), classes/methods (`Class.method`), functions (`module.function`), CLI flags (`--name`), file paths, dataclass/struct fields.
2. **Exclude new artifacts.** Do NOT verify symbols the plan declares under its "Artifacts this phase produces" section — those are created by this phase, not references to existing code.
3. **Resolve each remaining symbol** using the effective authority adapter (resolved deterministically — see step 4a):
   - `grep` — ripgrep / Read the source; confirm the name appears as a real declaration.
   - `intel` — consult the spec / `api-map.json` (only when `intel.enabled`).
   Record one verdict per symbol: **VERIFIED** (quote `file:line`), **MISSING** (adapter can check this language/kind and the symbol is absent), **AMBIGUOUS** (multiple candidates), or **UNCHECKABLE** (adapter cannot analyze this language/kind — e.g. non-JS under `intel`, or any signature under `grep`). Never treat UNCHECKABLE as verified or missing.
4a. **Resolve effective authority** (deterministic — replaces manual `intel.enabled` reasoning):
   ```bash
   EFFECTIVE_AUTHORITY="grep"   # the intel adapter is not part of @gsd/core: grep is the only authority
   ```
4. **Severity & gating** — classify each symbol's verdict using the seam (do not apply the table manually):
   ```bash
   # For each symbol, e.g.:
   RESULT='{"severity":"needs-acknowledgement","hardBlock":false}'   # the drift guard is not part of @gsd/core: under grep a missing symbol never hard-blocks
   # $RESULT is JSON: {"severity":"…","hardBlock":true|false}
   ```
   - `hardBlock: true` (HIGH at authority `lsp`/`scip`) — stops the review cycle immediately; do not proceed until the plan author resolves the missing symbol.
   - `hardBlock: false`, severity `needs-acknowledgement` — plan proceeds only if the author confirms the symbol is genuinely new or dynamically resolved, and that acknowledgement is recorded.
   - `AMBIGUOUS` → MEDIUM. `UNCHECKABLE` → INFO.
   - Signature mismatches cannot be asserted under `grep`/`intel`; report the signature as UNCHECKABLE.
5. **Coverage block.** Record every UNCHECKABLE/skipped symbol and why as an `unchecked` item of the round (the fence after the drift pass) — a clean review must never silently mean "nothing was checked."

### Cross-artifact fact-drift pass (same gate: `plan_review.source_grounding`)

Run this pass whenever the source-grounding pass ran — it is the second axis of the same drift guard, gated by the same `plan_review.source_grounding` key and adding no config surface of its own. Where source-grounding asks *"does this symbol exist in the source?"*, this asks *"does the project state the same fact in two planning artifacts, and do the two disagree?"* Because each phase runs in a fresh context, an agent typically reads only one artifact and trusts it, so a stale duplicate silently steers it wrong.

**Key on knowledge, not on similar text.** DRY is about a single authoritative representation of a piece of *knowledge*. Two passages that merely read alike, or that restate one fact at different levels of detail, are NOT drift. Only a contradiction is.

1. **Phase status — decided by the seam, not by judgment.** Do not eyeball this axis:

   ```bash
   DRIFT='{"verdict":"uncheckable"}'   # the drift guard is not part of @gsd/core, and STATE is computed: record it in the coverage block
   # $DRIFT is JSON: {"verdict":"consistent|lag|drifted|uncheckable","stateStatus":…,"roadmapStatus":…}
   ```

   - `drifted` — STATE.md and ROADMAP.md contradict each other. Report it; the authority is STATE.md.
   - `lag` — one lifecycle step apart between non-terminal statuses. NOT a finding.
   - `consistent` — nothing to report.
   - `uncheckable` — a document was absent or carried a status outside both vocabularies. Record it in the coverage block; never read it as consistent.

   Completeness is terminal: when exactly one side says the phase is complete, the verdict is `drifted` and never `lag`, however few steps apart the two words look.

2. **Pair up the remaining facts by judgment.** The authority column names the source of truth, so a finding can say which side to keep:

   | Fact class | Artifact pair | Authority | Decided by |
   |---|---|---|---|
   | Success criteria / must-have truths | ROADMAP.md Success Criteria ↔ PLAN.md `must_haves.truths` | ROADMAP.md | judgment |
   | Requirement IDs | ROADMAP.md `**Requirements:**` ↔ PLAN.md task requirement refs | ROADMAP.md | judgment |
   | Phase status | STATE.md status ↔ ROADMAP.md phase state | STATE.md | step 1 (deterministic) |
   | Glossary / domain term | CONTEXT.md `Decisions` ↔ PLAN.md usage of the term | CONTEXT.md | judgment |

3. **Judge each judgment pair.** FLAG only when ALL THREE hold:

   1. both sides name the *same* fact — same requirement ID, same success criterion, or the same defined term; and
   2. the two representations *contradict*, one asserting what the other denies, rather than differing in wording or in level of detail; and
   3. the pair is one of the judgment pairs above.

4. **Record.** Record each finding as a concern of the round with `drift` true and no reviewers (the fence below), its `evidence` quoting both locations and its text naming the divergence and the authority, so the author can collapse the two copies to a single source of truth.

**Do NOT flag:** a wording-only difference that asserts the same thing; a fact that appears in one artifact only — single-source is the target state, not a finding; a PLAN that ADDS a truth beyond the roadmap Success Criteria, which is sanctioned (plans may add, never subtract); a `lag` verdict from step 1 — two non-terminal statuses a single lifecycle step apart, in either direction, since the two statuses are recorded at different steps and either can lead as readily as trail (a disagreement about *completion* is never lag, and step 1 already reports it as `drifted`); anything under the context's `discretion` or its deferred ideas, which are non-authoritative by design.

**Report once, not twice — these belong to `gsd-plan-checker`:** a PLAN that omits a roadmap Success Criterion is scope reduction (Dimension 7b); a requirement ID the ROADMAP never defines is requirement coverage (Dimension 1); two PLAN.md files in one phase disagreeing is cross-plan data contracts (Dimension 9).

**Severity: advisory, never a blocker.** This pass never sets `hardBlock`, and its findings contribute to neither `HIGH_COUNT` nor `ACTIONABLE_COUNT` — a project carrying pre-existing drift must still be able to converge, or an advisory check becomes an endless replan loop.

**Coverage, never silence.** If one side of a pair is absent (the phase's status or its context), that axis is skipped and the skip is recorded as an `unchecked` item of the round. A clean pass must never mean "nothing was compared."

**Record the coverage block and the drift findings** on the round the review agent recorded, filling the additions
(the `--round` and `--review` options of `specforge gsd record-review`; no line when there is nothing to add):

```bash
PHASE_ID="{the phase id, e.g. v1.0/02}"
ROUND_JSON='{the additions as JSON, e.g. {"unchecked": [{"symbol": "--flag", "why": "grep cannot check a signature"}], "concerns": [{"plan": "v1.0/02/02-01", "severity": "low", "text": "the divergence and its authority", "drift": true, "evidence": ["<file:line>", "<file:line>"]}]}}'
REVIEW_ID=$(specforge query "$PHASE_ID" --depth 1 --kinds review --pick '.nodes[kind=review].id' --last 1) || exit 1
[ -n "$REVIEW_ID" ] || { echo "ERROR: no review recorded for this cycle — /gsd:review recorded no round of $PHASE_ID." >&2; exit 1; }
specforge gsd record-review "$PHASE_ID" --round "$REVIEW_ID" --review "$ROUND_JSON" || exit 1
```

### 5b. Read the Open Concerns from the Graph

The counts are the latest recorded round's, computed: a concern is open until a planner's disposition
(`addressed` or `deferred`) rules on it, and drift findings count in neither. Never count them from the review
agent's return message or from a review's text.

**Open plan-revision conflicts are part of the converged condition (#3771).** A plan-revision conflict is a checker
`fix_hint` that contradicted a locked decision, capability guidance, or an existing plan constraint: a
`revision_conflict` entity `/gsd:plan-phase` records (`gsd record-conflict`) together with the alternatives the
planner considered, and that convergence reads and never writes. Evaluate it BEFORE the converged branch below, or a
run would print the success banner over a conflict nobody resolved:

```bash
PHASE_ID="{the phase id, e.g. v1.0/02}"
REVIEW_ID=$(specforge query "$PHASE_ID" --depth 1 --kinds review --pick '.nodes[kind=review].id' --last 1) || exit 1
if [ -z "$REVIEW_ID" ]; then
  echo "ERROR: no review recorded for this cycle — /gsd:review recorded no round of $PHASE_ID." >&2
  exit 1
fi
HIGH_COUNT=$(specforge query "$REVIEW_ID" --depth 0 --pick '.nodes[0].fields.open.high') || exit 1
ACTIONABLE_COUNT=$(specforge query "$REVIEW_ID" --depth 0 --pick '.nodes[0].fields.open.actionable') || exit 1
OPEN_CONFLICTS=$(specforge query "$PHASE_ID" --depth 1 --kinds revision_conflict --pick '.nodes[fields.status=open].id' --count) || exit 1
HIGH_COUNT=${HIGH_COUNT:-0}   # a round with no open concern of a kind prints nothing for it
ACTIONABLE_COUNT=${ACTIONABLE_COUNT:-0}
UNRESOLVED_COUNT=$((HIGH_COUNT + ACTIONABLE_COUNT))
OPEN_LINES=""
while IFS=$'\x1f' read -r severity title; do
  OPEN_LINES="${OPEN_LINES}- ${severity}: ${title}"$'\n'
done < <(specforge query "$REVIEW_ID" --depth 1 --kinds review_concern --pick '.nodes[fields.status=open]' --fields fields.severity,title)
echo "REVIEW_ID=$REVIEW_ID HIGH_COUNT=$HIGH_COUNT ACTIONABLE_COUNT=$ACTIONABLE_COUNT OPEN_CONFLICTS=$OPEN_CONFLICTS"
```

A conflict's status is computed from its resolution, which only `/gsd:plan-phase` records; no other agent writes a
`revision_conflict`, since writing one forges the state of a blocking gate. Readers read. If `OPEN_CONFLICTS` > 0, convergence has NOT been
achieved regardless of the counts: skip the converged branch and continue to 5c so the next cycle
arbitrates. Escalation at `MAX_CYCLES` is unchanged and still terminates the loop, so an
unresolvable conflict escalates rather than deadlocking.

**If HIGH_COUNT == 0 and ACTIONABLE_COUNT == 0 and OPEN_CONFLICTS == 0 (converged):**

```bash
# STATE is computed: the plan entities are the record that the phase is planned
```

Display:
```text
### GSD ► CONVERGENCE COMPLETE ✓

 Phase {phase_number} converged in {cycle} cycle(s).
 No HIGH concerns remaining.
 No actionable MEDIUM/LOW review concern remains open.

 Review: {REVIEW_ID}
 Next: /gsd:execute-phase {PHASE}
```

Exit — convergence achieved.

**If HIGH_COUNT > 0 or ACTIONABLE_COUNT > 0 or OPEN_CONFLICTS > 0:** Continue to 5c.

### 5c. Stall Detection + Escalation Check

Display: `◆ Cycle {cycle}/{MAX_CYCLES} — {HIGH_COUNT} HIGH, {ACTIONABLE_COUNT} actionable non-HIGH review concerns, {OPEN_CONFLICTS} open plan-revision conflicts found`

**Stall detection:** If `UNRESOLVED_COUNT >= prev_unresolved_count`:
```text
⚠ Convergence stalled — unresolved review concern count not decreasing
  ({UNRESOLVED_COUNT} unresolved concerns, previous cycle had {prev_unresolved_count})
```

**Max cycles check:** If `cycle >= MAX_CYCLES`:

**If `OPEN_CONFLICTS` > 0 (#3771): "Proceed anyway" is never offered.** An open plan-revision
conflict is a blocker — this loop's whole purpose is to surface it rather than let a success
banner paper over it, so escalation cannot end in the same silent acceptance a HIGH/actionable
concern can. Only "Manual review" is available:

If `TEXT_MODE` is true, present as plain text:
```text
Plan convergence did not complete after {MAX_CYCLES} cycles.
{OPEN_CONFLICTS} open plan-revision conflict(s) remain — these are blockers and cannot be accepted:

{OPEN_LINES}

Review the concerns: specforge query {REVIEW_ID} --depth 1 --format context

To replan manually:  /gsd:plan-phase {PHASE} --reviews
To restart loop:     /gsd:plan-review-convergence {PHASE} {REVIEWER_FLAGS}
```
Exit workflow.

**Otherwise (`OPEN_CONFLICTS` == 0):**

If `TEXT_MODE` is true, present as plain-text numbered list:
```text
Plan convergence did not complete after {MAX_CYCLES} cycles.
{HIGH_COUNT} HIGH concerns and {ACTIONABLE_COUNT} actionable non-HIGH concerns remain:

{OPEN_LINES}

How would you like to proceed?

1. Proceed anyway — Accept plans with remaining review concerns and move to execution
2. Manual review — Stop here, review the recorded round and address concerns manually

Enter number:
```

Otherwise use AskUserQuestion:
```js
AskUserQuestion([
  {
    question: "Plan convergence did not complete after {MAX_CYCLES} cycles. {HIGH_COUNT} HIGH concerns and {ACTIONABLE_COUNT} actionable non-HIGH concerns remain:\n\n{OPEN_LINES}\n\nHow would you like to proceed?",
    header: "Convergence",
    multiSelect: false,
    options: [
      { label: "Proceed anyway", description: "Accept plans with remaining review concerns and move to execution" },
      { label: "Manual review", description: "Stop here — review the recorded round and address concerns manually" }
    ]
  }
])
```

If "Proceed anyway": Display final status and exit.
If "Manual review":
```text
Review the concerns: specforge query {REVIEW_ID} --depth 1 --format context

To replan manually:  /gsd:plan-phase {PHASE} --reviews
To restart loop:     /gsd:plan-review-convergence {PHASE} {REVIEWER_FLAGS}
```
Exit workflow.

### 5d. Replan (Inline)

**If under max cycles:**

Update `prev_unresolved_count = UNRESOLVED_COUNT`.

Display: `◆ Replanning inline with review feedback... (plan-phase runs here in the orchestrator — no output until replanning is complete, ~1–5 min; expected, not a freeze)`

```text
Skill(skill="gsd-plan-phase", args="{PHASE} --reviews --skip-research")
```

Run plan-phase **inline** (do NOT wrap it in Agent()). Same rationale as step 4: the convergence orchestrator runs at depth 0 with Agent available, so inline plan-phase can spawn gsd-planner and gsd-plan-checker at depth 1. Wrapping in Agent() pushes plan-phase to depth 1 where the Agent tool is absent — the replan loop can never produce a revised plan when HIGHs are found. This is the root cause of bug #936. Actionable MEDIUM/LOW findings must be incorporated into the plans' tasks or explicitly deferred/rejected on the relevant plan before convergence can complete. The same holds for any open `## Plan-Revision Conflicts` entry (#3771): the replan must resolve it by adopting one of its recorded alternatives, overriding the named constraint, or amending the constraint — and mark the entry resolved. Re-running the planner against an unchanged conflict cannot resolve it and only burns a cycle. Wait until plan-phase completes (outputs '## PLANNING COMPLETE') and the updated plans are committed before continuing.

After plan-phase completes → go back to **step 5a** (review again).

</process>

<success_criteria>
- [ ] Config gate checked before running — exits with enable instructions if workflow.plan_review_convergence is false (an explicit `--override-gate` dispatch — how `/gsd:autonomous --converge` invokes this workflow — bypasses it for that run, #4600)
- [ ] Initial planning via inline Skill("gsd-plan-phase") if no plans exist — NOT wrapped in Agent() (bug #936: depth-1 Agent has no Agent tool)
- [ ] Review via Agent → Skill("gsd-review") — isolated Agent is correct; gsd-review is a Bash leaf with no sub-agent spawns
- [ ] Replan via inline Skill("gsd-plan-phase --reviews") — NOT wrapped in Agent(); inline lets plan-phase spawn gsd-planner/gsd-plan-checker at depth 1
- [ ] Orchestrator only does: init, config gate, loop control, read the open HIGH and actionable non-HIGH counts of the latest round, stall detection, escalation
- [ ] HIGH and actionable non-HIGH counts read from the latest recorded round's `open` (never from the agent's return message or a review's text)
- [ ] Review agent prompt defines the recording contract with PARTIALLY/FULLY RESOLVED/ACTIONABLE definitions
- [ ] Abort with clear error if no review round is recorded for the cycle
- [ ] The review Agent fully completes gsd-review before returning (plan-phase runs inline — no Agent wrap)
- [ ] Loop exits on: no HIGH concerns, no actionable non-HIGH concerns, and OPEN_CONFLICTS == 0 (converged) OR max cycles (escalation)
- [ ] OPEN_CONFLICTS read from the phase's `revision_conflict` entities and evaluated BEFORE the converged branch writes state or prints the banner
- [ ] Stall detection reported when total unresolved review concern count is not decreasing
</success_criteria>

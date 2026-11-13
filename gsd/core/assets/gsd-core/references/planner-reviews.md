<!-- Ported from gsd-core v1.19.0 gsd-core/references/planner-reviews.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Reviews Mode — Planner Reference

Triggered when orchestrator sets Mode to `reviews`. Replanning from scratch with the recorded review round as additional context.

**Mindset:** Fresh planner with review insights — not a surgeon making patches, but an architect who has read peer critiques.

**Execution contract:** the review round is audit trail and feedback input, not a second execution contract. /gsd:execute-phase primarily consumes the plan entities plus the normal phase context. Every open review concern must therefore be incorporated into the relevant plan or deferred by a disposition of that plan.

### Step 1: Load the Review Round
Read the round from `<required_reading>` (`specforge query <review> --depth 1 --format context`). Read:
- Per-reviewer feedback (strengths, concerns, suggestions)
- Consensus Summary (agreed concerns = highest priority to address)
- Divergent Views (investigate, make a judgment call)

### Step 2: Categorize Feedback
Group review feedback into:
- **Must address**: HIGH severity consensus concerns
- **Must represent in PLAN.md**: actionable MEDIUM/LOW findings that require task, action, acceptance criteria, verify command, must_haves, threat-model, artifact, stale-path, or execution-contract changes
- **Should address**: MEDIUM severity concerns from 2+ reviewers that improve quality but do not change the executable contract
- **Consider**: Individual reviewer suggestions, LOW severity items

### Step 3: Plan Fresh with Review Context
Create new plans following the standard planning process, but with review feedback as additional constraints:
- Each HIGH severity consensus concern MUST have a task that addresses it
- Each open actionable MEDIUM/LOW concern MUST either appear in the relevant plan's executable content or be deferred by a disposition
- Note in task actions: "Addresses review concern: {concern}" for traceability

### Step 4: Return
Use standard PLANNING COMPLETE return format, adding a reviews section:

```markdown
### Review Feedback Addressed

| Concern | Severity | How Addressed |
|---------|----------|---------------|
| {concern} | HIGH | Plan {N}, Task {M}: {how} |

### Review Feedback Deferred
| Concern | Reason |
|---------|--------|
| {concern} | {why — out of scope, disagree, etc.} |
```

### Step 5: Record the dispositions (#3806)

The two tables above are the planner's return payload; each of their rows is also recorded as a
**disposition** of the plan that rules on it, which is the canonical Review Dispositions Ledger.
`gsd-core/workflows/plan-phase.md` (`<review_incorporation_contract>`) and
`agents/gsd-plan-checker.md` (Review Incorporation dimension) both point back to this section for
the ledger's shape rather than restating it — this is the one place it is defined.

## Review Dispositions Ledger

The ledger is one `disposition` per concern a plan rules on (`specforge new disposition --help`): its `concern` is
the `review_concern` id, its `outcome` `addressed` or `deferred` (with the `reason` when deferred), its `round` the
reviews-mode round number and its `reviewer` who raised the concern. A disposition is what turns the concern's computed `status` from `open`. Fill the values, one line per
concern, each with the plan's next disposition id:

```bash
PLAN_ID="{the plan that rules on the concern, e.g. v1.0/02/02-01}"
DISPOSITION_ID="{the plan's next disposition id, e.g. DP1}"
CONCERN_ID="{the review_concern id, e.g. v1.0/02/RV1/RC1}"
ROUND="{the reviews-mode round number, e.g. 1}"
REVIEWER="{who raised the concern, e.g. codex}"
FEEDBACK="{the concern, in a few words}"
OUTCOME="{addressed or deferred}"
REASON="{how it was addressed, or why it was deferred}"
PLAN_FILE=$(specforge query "$PLAN_ID" --depth 0 --pick '.nodes[0].file') || exit 1
specforge new disposition "$DISPOSITION_ID" "$FEEDBACK" --in "$PLAN_FILE" --plan "$PLAN_ID" --round "$ROUND" --reviewer "$REVIEWER" --concern "$CONCERN_ID" --feedback "$FEEDBACK" --outcome "$OUTCOME" --reason "$REASON" || exit 1
```

**Anchoring.** A disposition names its concern and round by id, so it stays anchored when the next round is
recorded. `{Concern}` and `{Reason}` stay free text; do not invent a reviewer/severity enum — the
reviewer roster is capability-owned and open to third-party additions (see each capability's
`reviewer.reviewsSection`).

**Append-only.** A later round never edits or deletes a prior round's dispositions. To overturn a prior
round's verdict, record a new disposition for the current round's concern whose reason names the
round and concern it supersedes (e.g. "Supersedes Round 1 Deferred: {concern} — now addressed in
Plan 3"); the concern's status reads the highest round.

**Out of scope for this contract.** A deterministic lint/check verb that mechanically enforces this
shape is a separate, later addition (#3806 part 2) — this section defines the format only. Legacy
PLAN.md content written before this convention existed is not migrated or flagged by it.

# Where each authored field comes from

`@gsd/core` holds exactly the kinds and fields gsd-core defines, and SpecForge adds only computed values, collected
facts, graph logic and guards (ADR 0370 D1). This file is the evidence for the fields a person or an agent writes: a
field whose `writer` is absent or `human` in the kinds golden. A computed, harness, context or verdict field is
SpecForge's by definition and has no row. The names SpecForge gives items that gsd-core leaves unnamed (`T1`, `TR1`,
`AR1` and the rest of [README.md](README.md#names-of-every-kind)) are not fields and have no row either (ADR 0370 D7).

A row names one field of one kind, and the `source` says where the field comes from:

- **an upstream line**: a path under `gsd/parity/upstream/v1.19.0/` and a line number (`gsd-core/templates/project.md:8`),
  the gsd-core 1.19.0 file and line the field mirrors. `the line holds` is a text that line holds, verbatim (a `|`
  in it is written `\|`), so the row is wrong the day the line stops saying it;
- **a capability**: when gsd-core writes no such field, the SpecForge capability that reads it, as the
  declaration names it: `pass:<name>`, `rule:gsd/<code>`, `route:<name>`, `command:<name>` or
  `computed:<kind>.<field>` (a computed field whose pass reads this one). `the line holds` is empty.

A field that both mirrors upstream and is read by a capability cites upstream. A block or list field has one row, and
its members are covered by the line that holds the block. A field with neither source has no row: it is removed from
its kind, or given the capability that reads it. The test `tests/provenance.rs`
(`every_authored_field_names_its_upstream_line_or_the_capability_that_reads_it`) holds the table to the declaration:
one row per authored field, no row for another field, every upstream line holding its text, every capability declared.

## project.md

| kind | field | source | the line holds |
|---|---|---|---|
| project | name | gsd-core/templates/project.md:8 | # [Project Name] |
| project | what | gsd-core/templates/project.md:10 | ## What This Is |
| project | core_value | gsd-core/templates/project.md:15 | ## Core Value |
| project | customer | gsd-core/templates/project.md:24 | **Customer**: [Who pays / who uses |
| project | revenue_model | gsd-core/templates/project.md:25 | **Revenue model**: [How it makes money |
| project | success_metric | gsd-core/templates/project.md:26 | **Success metric**: [The number that matters |
| project | context | gsd-core/templates/project.md:52 | ## Context |
| project | constraints | gsd-core/templates/project.md:60 | ## Constraints |
| project | out_of_scope | gsd-core/templates/project.md:45 | ### Out of Scope |
| project | key_decisions | gsd-core/templates/project.md:67 | ## Key Decisions |
| project | project_code | gsd-core/references/planning-config.md.txt:266 | Prefix for phase dirs |
| requirement | text | gsd-core/templates/requirements.md:26 | **[CAT]-01**: [Requirement description] |
| requirement | category | gsd-core/templates/requirements.md:39 | ### [Category] |
| requirement | tier | gsd-core/templates/requirements.md:13 | ## v1 Requirements |
| requirement | reason | gsd-core/templates/requirements.md:50 | [Why excluded] |
| requirement | defined | gsd-core/templates/requirements.md:10 | **Defined:** [date] |
| requirement | shapes | gsd-core/references/specless-probe-fallback.md:88 | REQUIREMENTS.md: {"id","text","shapes"?} |
| requirement | text_en | src/edge-probe.cts:63 | text_en?: string; |
| milestone | name | gsd-core/templates/milestone.md:6 | ## v[X.Y] [Name] (Shipped: YYYY-MM-DD) |
| milestone | delivered | gsd-core/templates/milestone.md:8 | **Delivered:** [One sentence describing what shipped] |
| milestone | accomplishments | gsd-core/templates/milestone.md:12 | **Key accomplishments:** |
| milestone | whats_next | gsd-core/templates/milestone.md:26 | **What's next:** |
| milestone | goal | gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md:8 | **Goal:** [One sentence describing milestone focus] |
| milestone | target_features | gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md:10 | **Target features:** |
| milestone | abandoned | computed:milestone.state |  |
| milestone | workstream | gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md:1 | .planning/workstreams/<name>/STATE.md |
| phase | milestone | gsd-core/templates/roadmap.md:167 | ### 🚧 v1.1 [Name] (In Progress) |
| phase | name | gsd-core/templates/roadmap.md:29 | ### Phase 1: [Name] |
| phase | slug | gsd-core/templates/spec.md:3 | .planning/phases/XX-name/ |
| phase | goal | gsd-core/templates/roadmap.md:30 | **Goal**: [What this phase delivers] |
| phase | depends_on | gsd-core/templates/roadmap.md:31 | **Depends on**: Nothing (first phase) |
| phase | requirements | gsd-core/templates/roadmap.md:32 | **Requirements**: [REQ-01, REQ-02, REQ-03] |
| phase | success_criteria | gsd-core/templates/roadmap.md:33 | **Success Criteria** (what must be TRUE): |
| phase | mode | gsd-core/workflows/new-project.md:969 | `**Mode:** mvp` |
| workstream | purpose | command:workstreams |  |
| retrospective | milestone | gsd-core/templates/retrospective.md:5 | ## Milestone: v{version} — {name} |
| retrospective | built | gsd-core/templates/retrospective.md:10 | ### What Was Built |
| retrospective | worked | gsd-core/templates/retrospective.md:15 | ### What Worked |
| retrospective | inefficient | gsd-core/templates/retrospective.md:19 | ### What Was Inefficient |
| retrospective | patterns | gsd-core/templates/retrospective.md:23 | ### Patterns Established |
| retrospective | cost | gsd-core/templates/retrospective.md:30 | ### Cost Observations |
| lesson | retrospective | gsd-core/templates/retrospective.md:26 | ### Key Lessons |
| lesson | text | gsd-core/templates/retrospective.md:27 | {Specific, actionable lesson} |
| lesson | top | gsd-core/templates/retrospective.md:51 | ### Top Lessons (Verified Across Milestones) |
| lesson | kind | gsd-core/workflows/extract-learnings.md:12 | 4 categories: decisions, lessons, patterns, and surprises |
| methodology | name | gsd-core/workflows/discuss-phase-assumptions.md:224 | Parse each named lens |
| methodology | diagnoses | gsd-core/workflows/discuss-phase-assumptions.md:224 | its diagnoses |
| methodology | recommends | gsd-core/workflows/discuss-phase-assumptions.md:224 | recommendations |
| methodology | apply_when | gsd-core/workflows/discuss-phase-assumptions.md:224 | triggering conditions |

## discuss.md

| kind | field | source | the line holds |
|---|---|---|---|
| context | phase | gsd-core/templates/context.md:3 | .planning/phases/XX-name/{phase_num}-CONTEXT.md |
| context | plan | gsd-core/workflows/quick.md:457 | ${QUICK_DIR}/${quick_id}-CONTEXT.md |
| context | gathered | gsd-core/templates/context.md:20 | **Gathered:** [date] |
| context | boundary | gsd-core/templates/context.md:24 | ## Phase Boundary |
| context | discretion | gsd-core/templates/context.md:43 | ### Claude's Discretion |
| context | specifics | gsd-core/templates/context.md:49 | ## Specific Ideas |
| context | canonical_refs | gsd-core/templates/context.md:58 | ## Canonical References |
| context | reusable_assets | gsd-core/templates/context.md:78 | ### Reusable Assets |
| context | patterns | gsd-core/templates/context.md:81 | ### Established Patterns |
| context | integration_points | gsd-core/templates/context.md:84 | ### Integration Points |
| decision | phase | gsd-core/templates/context.md:3 | .planning/phases/XX-name/{phase_num}-CONTEXT.md |
| decision | plan | gsd-core/workflows/quick.md:457 | ${QUICK_DIR}/${quick_id}-CONTEXT.md |
| decision | area | gsd-core/templates/context.md:33 | ### [Area 1 that was discussed] |
| decision | text | gsd-core/templates/context.md:34 | **D-01:** [Specific decision made] |
| decision | rationale | gsd-core/templates/project.md:71 | Decision \| Rationale \| Outcome |
| decision | outcome | gsd-core/templates/project.md:73 | [✓ Good / ⚠️ Revisit / — Pending] |
| decision | reversibility | gsd-core/references/planner-reversibility.md:57 | discuss-phase already recorded a rating and rationale |
| decision | settled | rule:gsd/W311 |  |
| decision | origin | rule:gsd/E310 |  |
| deferred_idea | text | gsd-core/templates/context.md:90 | ## Deferred Ideas |
| deferred_idea | phase | gsd-core/templates/context.md:3 | .planning/phases/XX-name/{phase_num}-CONTEXT.md |
| deferred_idea | why | gsd-core/workflows/discuss-phase/templates/context.md:141 | reason it was deferred |
| deferred_idea | origin | route:phase-context |  |
| deferred_idea | status | gsd-core/workflows/progress/steps/forensic-audit.md:95 | status: resolved |
| phase_spec | phase | gsd-core/templates/spec.md:3 | .planning/phases/XX-name/{phase_num}-SPEC.md |
| phase_spec | created | gsd-core/templates/spec.md:21 | **Created:** [date] |
| phase_spec | goal | gsd-core/templates/spec.md:25 | ## Goal |
| phase_spec | background | gsd-core/templates/spec.md:29 | ## Background |
| phase_spec | in_scope | gsd-core/templates/spec.md:49 | **In scope:** |
| phase_spec | out_of_scope | gsd-core/templates/spec.md:53 | **Out of scope:** |
| phase_spec | constraints | gsd-core/templates/spec.md:57 | ## Constraints |
| phase_spec | acceptance | gsd-core/templates/spec.md:62 | ## Acceptance Criteria |
| locked_req | spec | gsd-core/templates/spec.md:3 | {phase_num}-SPEC.md |
| locked_req | title | gsd-core/templates/spec.md:35 | **[Short label]**: [Specific, testable statement.] |
| locked_req | current | gsd-core/templates/spec.md:36 | - Current: [what exists or does NOT exist today] |
| locked_req | target | gsd-core/templates/spec.md:37 | - Target: [what it should become after this phase] |
| locked_req | acceptance | gsd-core/templates/spec.md:38 | - Acceptance: [concrete pass/fail check |
| locked_req | traces | rule:gsd/W203 |  |
| locked_req | shapes | gsd-core/references/edge-probe.md:44 | shapes? |
| locked_req | text_en | gsd-core/references/edge-probe.md:44 | text_en? |
| interview | spec | gsd-core/templates/spec.md:3 | {phase_num}-SPEC.md |
| interview | round | gsd-core/templates/spec.md:111 | Format: round → question → answer → decision locked. |
| interview | perspective | gsd-core/templates/spec.md:113 | Perspective |
| interview | question | gsd-core/templates/spec.md:111 | Format: round → question → answer → decision locked. |
| interview | answer | gsd-core/templates/spec.md:111 | Format: round → question → answer → decision locked. |
| interview | locked | gsd-core/templates/spec.md:113 | Decision locked |
| edge | spec | gsd-core/templates/spec.md:3 | {phase_num}-SPEC.md |
| edge | requirement | gsd-core/references/edge-probe.md:128 | { requirement_id, category |
| edge | category | gsd-core/references/edge-probe.md:128 | requirement_id, category, status |
| edge | status | gsd-core/references/edge-probe.md:128 | category, status, verification |
| edge | verification | gsd-core/references/edge-probe.md:128 | status, verification, resolution |
| edge | resolution | gsd-core/references/edge-probe.md:128 | verification, resolution, reason |
| edge | reason | gsd-core/references/edge-probe.md:128 | resolution, reason, probe |
| spec_prohibition | spec | gsd-core/templates/spec.md:3 | {phase_num}-SPEC.md |
| spec_prohibition | requirement | gsd-core/templates/spec.md:87 | Requirement \| Status \| Verification / Reason |
| spec_prohibition | statement | gsd-core/templates/spec.md:87 | Prohibition (must-NOT statement) |
| spec_prohibition | status | gsd-core/references/prohibition-probe.md:99 | **status** |
| spec_prohibition | verification | gsd-core/references/prohibition-probe.md:105 | **verification** (only when |
| spec_prohibition | reason | gsd-core/references/prohibition-probe.md:101 | accompanied by a required |
| spec_prohibition | check_kind | gsd-core/references/prohibition-probe.md:163 | `check_kind` — `node-test` |
| spec_prohibition | check_target | gsd-core/references/prohibition-probe.md:164 | `check_target` — the test file |
| spec_prohibition | check_rule | gsd-core/references/prohibition-probe.md:165 | `check_rule` — the `ruleId` |
| spec_prohibition | check_violation_fixture | gsd-core/references/prohibition-probe.md:166 | `check_violation_fixture` — path to a KNOWN-BAD subject |
| spec_prohibition | check_clean_fixture | gsd-core/references/prohibition-probe.md:168 | `check_clean_fixture` — **optional** path |
| discussion | phase | gsd-core/workflows/discuss-phase.md:455 | ${phase_dir}/${padded_phase}-DISCUSSION-LOG.md |
| discussion | entries | gsd-core/workflows/discuss-phase.md:380 | accumulate area name, options presented |
| discussion | checkpoint | gsd-core/workflows/discuss-phase.md:379 | Incremental checkpoint |
| research | phase | agents/gsd-phase-researcher.md:270 | {phase_num}-RESEARCH.md |
| research | plan | gsd-core/workflows/quick.md:465 | ${QUICK_DIR}/${quick_id}-RESEARCH.md |
| research | domain | agents/gsd-phase-researcher.md:276 | **Domain:** [primary technology/problem domain] |
| research | researched | agents/gsd-phase-researcher.md:275 | **Researched:** [date] |
| research | confidence | agents/gsd-phase-researcher.md:277 | **Confidence:** [HIGH/MEDIUM/LOW] |
| research | summary | agents/gsd-phase-researcher.md:279 | ## Summary |
| research | user_constraints | agents/gsd-phase-researcher.md:771 | <user_constraints> |
| research | responsibility_map | agents/gsd-phase-researcher.md:285 | ## Architectural Responsibility Map |
| finding | research | agents/gsd-phase-researcher.md:279 | ## Summary |
| finding | topic | agents/gsd-research-synthesizer.md:27 | STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md |
| finding | text | gsd-core/templates/research-project/SUMMARY.md:23 | ## Key Findings |
| finding | confidence | gsd-core/templates/research-project/STACK.md:12 | **Confidence:** [HIGH/MEDIUM/LOW] |
| finding | sources | agents/gsd-phase-researcher.md:28 | must be tagged with its source |
| stack_choice | research | agents/gsd-phase-researcher.md:291 | ## Standard Stack |
| stack_choice | name | agents/gsd-phase-researcher.md:294 | Library \| Version \| Purpose \| Why Standard |
| stack_choice | version | agents/gsd-phase-researcher.md:294 | Version \| Purpose |
| stack_choice | purpose | agents/gsd-phase-researcher.md:294 | Purpose \| Why Standard |
| stack_choice | tier | agents/gsd-phase-researcher.md:293 | ### Core |
| stack_choice | rejected_because | agents/gsd-project-researcher.md:239 | Alternative \| Why Not |
| pattern | research | agents/gsd-phase-researcher.md:336 | ## Architecture Patterns |
| pattern | name | agents/gsd-phase-researcher.md:360 | ### Pattern 1: [Pattern Name] |
| pattern | what | agents/gsd-phase-researcher.md:361 | **What:** [description] |
| pattern | when | agents/gsd-phase-researcher.md:362 | **When to use:** [conditions] |
| pattern | example | agents/gsd-phase-researcher.md:363 | **Example:** |
| pattern | anti | agents/gsd-phase-researcher.md:369 | ### Anti-Patterns to Avoid |
| pitfall | research | agents/gsd-phase-researcher.md:394 | ## Common Pitfalls |
| pitfall | name | agents/gsd-phase-researcher.md:396 | ### Pitfall 1: [Name] |
| pitfall | goes_wrong | agents/gsd-phase-researcher.md:397 | **What goes wrong:** [description] |
| pitfall | why | agents/gsd-phase-researcher.md:398 | **Why it happens:** [root cause] |
| pitfall | avoid | agents/gsd-phase-researcher.md:399 | **How to avoid:** [prevention strategy] |
| pitfall | warning_signs | agents/gsd-phase-researcher.md:400 | **Warning signs:** [how to detect early] |
| dont_hand_roll | research | agents/gsd-phase-researcher.md:372 | ## Don't Hand-Roll |
| dont_hand_roll | problem | agents/gsd-phase-researcher.md:374 | Problem \| Don't Build |
| dont_hand_roll | instead | agents/gsd-phase-researcher.md:374 | Don't Build \| Use Instead |
| dont_hand_roll | why | agents/gsd-phase-researcher.md:374 | Use Instead \| Why |
| open_question | research | agents/gsd-phase-researcher.md:432 | ## Open Questions |
| open_question | question | agents/gsd-phase-researcher.md:434 | **[Question]** |
| open_question | known | agents/gsd-phase-researcher.md:435 | What we know: [partial info] |
| open_question | unclear | agents/gsd-phase-researcher.md:436 | What's unclear: [the gap] |
| open_question | recommendation | agents/gsd-phase-researcher.md:437 | Recommendation: [how to handle] |
| open_question | resolved_by | agents/gsd-plan-checker.md:633 | RESOLVED: Use |
| source | research | agents/gsd-phase-researcher.md:503 | ## Sources |
| source | url | agents/gsd-phase-researcher.md:507 | [Official docs URL] |
| source | tier | agents/gsd-phase-researcher.md:505 | ### Primary (HIGH confidence) |
| source | checked | agents/gsd-phase-researcher.md:507 | [what was checked] |
| package_audit | research | agents/gsd-phase-researcher.md:321 | ## Package Legitimacy Audit |
| package_audit | package | agents/gsd-phase-researcher.md:325 | Package \| Registry |
| package_audit | registry | agents/gsd-phase-researcher.md:325 | Package \| Registry |

## plan.md

Two rows of this section name the capability that reads a link field gsd-core does not write: `truth.requirement`
(`gsd/E519`) and `truth.proof` (a judged input of `gsd/E501`). Each row goes with its field when the field leaves its
kind.

| kind | field | source | the line holds |
|---|---|---|---|
| plan | phase | agents/gsd-planner.md:311 | phase: XX-name |
| plan | type | agents/gsd-planner.md:313 | type: execute |
| plan | depends_on | agents/gsd-planner.md:315 | depends_on: [] |
| plan | files_modified | agents/gsd-planner.md:316 | files_modified: [] |
| plan | files_deleted | gsd-core/references/planner-quick-batch.md:54 | ### `files_deleted` — only if your plan removes a file |
| plan | coupling_justified | gsd-core/references/planner-coupling.md:30 | coupling_justified: ["03-02: |
| plan | gap_closure | gsd-core/workflows/verify-work.md:788 | MUST carry `gap_closure: true` and `gap_ids: [...]` |
| plan | requirements | agents/gsd-planner.md:318 | requirements: []            # REQUIRED |
| plan | wave0 | gsd-core/references/nyquist-compliance.md:55 | Wave 0 plan must execute before dependent task |
| plan | agent_hint | gsd-core/workflows/execute-phase/steps/per-plan-executor-routing.md:11 | A plan opts into a specialist by declaring `agent_hint: <name>` |
| plan | estimate | agents/gsd-planner.md:321 | estimate:                   # Projected execution cost |
| plan | objective | agents/gsd-planner.md:333 | <objective> |
| plan | purpose | agents/gsd-planner.md:336 | Purpose: [Why this matters] |
| plan | output | agents/gsd-planner.md:337 | Output: [Artifacts created] |
| plan | context_refs | agents/gsd-planner.md:345 | <context> |
| plan | verification | agents/gsd-planner.md:384 | <verification> |
| plan | success_criteria | agents/gsd-planner.md:388 | <success_criteria> |
| plan | slug | gsd-core/workflows/quick.md:138 | `quick_id`, `slug` |
| task | plan | agents/gsd-planner.md:354 | <tasks> |
| task | type | agents/gsd-planner.md:356 | <task type="auto"> |
| task | files | agents/gsd-planner.md:169 | **<files>:** Exact file paths created or modified. |
| task | read_first | agents/gsd-planner.md:177 | Code excerpts belong in `<read_first>` source files |
| task | precondition | agents/gsd-planner.md:209 | **<precondition>** (optional, one prose line) |
| task | reversibility | agents/gsd-planner.md:211 | **<reversibility>** (optional) |
| task | action | agents/gsd-planner.md:173 | **<action>:** Specific implementation instructions |
| task | verify | agents/gsd-planner.md:191 | **Nyquist Rule:** Every `<verify>` includes `<automated>`. |
| task | verify_text | agents/gsd-planner.md:360 | <verify>[Command or check]</verify> |
| task | verify_human | agents/gsd-planner.md:251 | use `<verify><human-check>`. |
| task | acceptance | gsd-core/references/planner-preconditions.md:16 | `<verify>` + `<done>` + `<acceptance_criteria>` |
| task | done | agents/gsd-planner.md:205 | **<done>:** Acceptance criteria |
| checkpoint | plan | gsd-core/references/checkpoints.md:49 | <task type="checkpoint:human-verify" gate="blocking"> |
| checkpoint | after | gsd-core/references/checkpoints.md:591 | **After automation completes** - not before Claude does the work |
| checkpoint | kind | gsd-core/references/checkpoints.md:150 | type="checkpoint:decision" |
| checkpoint | gate | gsd-core/references/checkpoints.md:18 | `gate="blocking"` |
| checkpoint | what_built | gsd-core/references/checkpoints.md:50 | <what-built>[What Claude automated and deployed/built]</what-built> |
| checkpoint | how_to_verify | gsd-core/references/checkpoints.md:51 | <how-to-verify> |
| checkpoint | decision | gsd-core/references/checkpoints.md:151 | <decision>[What's being decided]</decision> |
| checkpoint | options | gsd-core/references/checkpoints.md:153 | <options> |
| checkpoint | auto_select | gsd-core/references/checkpoints.md:169 | `auto_select` is optional and names the `id` of the `<option>` |
| checkpoint | instructions | gsd-core/references/checkpoints.md:252 | <instructions> |
| checkpoint | action | gsd-core/references/checkpoints.md:251 | <action>[What human must do |
| checkpoint | verification | gsd-core/references/checkpoints.md:256 | <verification>[What Claude can check afterward]</verification> |
| checkpoint | resume_signal | gsd-core/references/checkpoints.md:54 | <resume-signal>[How to continue - "approved", "yes", or describe issues]</resume-signal> |
| truth | plan | agents/gsd-planner.md:327 | must_haves: |
| truth | text | agents/gsd-planner.md:328 | truths: []                # Observable behaviors |
| truth | requirement | rule:gsd/E519 |  |
| truth | proof | rule:gsd/E501 |  |
| truth | verification | gsd-core/references/honest-verifier.md:66 | `{ statement, verification: backstop }` |
| artifact | plan | gsd-core/references/planner-guidance.md:227 | artifacts: |
| artifact | path | gsd-core/references/planner-guidance.md:228 | - path: "src/components/Chat.tsx" |
| artifact | provides | gsd-core/references/planner-guidance.md:229 | provides: "Message list rendering" |
| artifact | min_lines | gsd-core/references/planner-guidance.md:230 | min_lines: 30 |
| artifact | exports | gsd-core/references/planner-guidance.md:233 | exports: ["GET", "POST"] |
| artifact | contains | gsd-core/references/planner-guidance.md:236 | contains: "model Message" |
| key_link | plan | gsd-core/references/planner-guidance.md:237 | key_links: |
| key_link | from | gsd-core/references/planner-guidance.md:238 | - from: "src/components/Chat.tsx" |
| key_link | to | gsd-core/references/planner-guidance.md:239 | to: "src/app/api/chat/route.ts" |
| key_link | via | gsd-core/references/planner-guidance.md:240 | via: "fetch in useEffect — calls /api/chat endpoint" |
| key_link | pattern | gsd-core/references/planner-guidance.md:241 | pattern: "fetch.*api/chat" |
| prohibition | plan | gsd-core/references/prohibition-probe.md:160 | **five flat scalar keys** on the `must_haves.prohibitions` item |
| prohibition | statement | gsd-core/references/prohibition-probe.md:203 | where `statement` is the must-NOT sentence |
| prohibition | verification | gsd-core/references/prohibition-probe.md:105 | - **verification** (only when `status` is `resolved` |
| prohibition | test | gsd-core/references/prohibition-probe.md:107 | - **test** — the prohibition can be mechanically checked |
| disposition | plan | gsd-core/references/planner-reviews.md:54 | `## Review Dispositions Ledger` section in the affected PLAN.md |
| disposition | round | gsd-core/references/planner-reviews.md:55 | `### Round {N} — {REVIEWS_sha}` subsection |
| disposition | feedback | gsd-core/references/planner-reviews.md:68 | {concern} \| HIGH \| Plan {N}, Task {M}: {how} |
| disposition | outcome | gsd-core/references/planner-reviews.md:70 | ### Review Feedback Deferred |
| disposition | reason | gsd-core/references/planner-reviews.md:71 | Concern \| Reason |
| disposition | concern | gsd-core/references/planner-reviews.md:66 | Concern \| Severity \| How Addressed |
| review | phase | gsd-core/workflows/review.md:739 | phase: {N} |
| review | plans | gsd-core/workflows/review.md:742 | plans_reviewed: [{list of PLAN.md files}] |
| review | reviewers | gsd-core/workflows/review.md:740 | reviewers: [claude, codex |
| review | lanes | gsd-core/workflows/review.md:778 | ## <reviewsSection> Review |
| review | coverage | gsd-core/workflows/review.md:759 | plan_coverage:            # only present |
| review | consensus | gsd-core/workflows/review.md:793 | ## Consensus Summary |
| review | strengths | gsd-core/workflows/review.md:797 | ### Agreed Strengths |
| review | divergent | gsd-core/workflows/review.md:803 | ### Divergent Views |
| review | unchecked | gsd-core/workflows/plan-review-convergence.md:310 | **UNCHECKABLE** (adapter cannot analyze this language/kind |
| review_concern | review | gsd-core/workflows/review.md:800 | ### Agreed Concerns |
| review_concern | plan | gsd-core/workflows/review.md:227 | Analyze each plan and provide: |
| review_concern | reviewers | gsd-core/workflows/review.md:801 | {concerns raised by 2+ reviewers |
| review_concern | severity | gsd-core/workflows/review.md:231 | with severity: HIGH/MEDIUM/LOW |
| review_concern | text | gsd-core/workflows/review.md:231 | **Concerns** — Potential issues, gaps, risks |
| review_concern | actionable | gsd-core/workflows/plan-review-convergence.md:285 | ACTIONABLE — a non-HIGH review finding |
| review_concern | drift | gsd-core/workflows/plan-review-convergence.md:368 | **Severity: advisory, never a blocker.** |
| review_concern | evidence | gsd-core/workflows/review.md:214 | cite concrete `path/to/file:line` evidence |
| revision_conflict | phase | gsd-core/references/revision-loop.md:121 | and the phase `*-REVIEWS.md` already exists |
| revision_conflict | plan | gsd-core/references/revision-loop.md:125 | REVISION_CONFLICT {dimension}/{plan} |
| revision_conflict | dimension | gsd-core/references/revision-loop.md:125 | REVISION_CONFLICT {dimension} |
| revision_conflict | property | gsd-core/references/revision-loop.md:125 | required_property: {property} |
| revision_conflict | conflicts_with | gsd-core/references/revision-loop.md:125 | conflicts with: {locked decision D-nn |
| revision_conflict | alternatives | gsd-core/references/revision-loop.md:125 | alternatives: {the agent's alternatives} |
| user_setup | plan | gsd-core/templates/user-setup.md:76 | In PLAN.md, `user_setup` declares human-required configuration |
| user_setup | service | gsd-core/templates/user-setup.md:80 | - service: stripe |
| user_setup | why | gsd-core/templates/user-setup.md:81 | why: "Payment processing requires API keys" |
| user_setup | env_vars | gsd-core/templates/user-setup.md:82 | env_vars: |
| user_setup | dashboard | gsd-core/templates/user-setup.md:87 | dashboard_config: |
| user_setup | local_dev | gsd-core/templates/user-setup.md:91 | local_dev: |

## execute.md

| kind | field | source | the line holds |
|---|---|---|---|
| plan_start | plan | agents/gsd-executor.md:106 | PLAN_START_TIME=$(date -u |
| summary | plan | gsd-core/templates/summary.md:12 | plan: YY |
| summary | status | gsd-core/templates/summary.md:71 | status: complete |
| summary | one_liner | gsd-core/templates/summary.md:76 | **[Substantive one-liner describing outcome |
| summary | subsystem | gsd-core/templates/summary.md:13 | subsystem: [primary category |
| summary | tags | gsd-core/templates/summary.md:14 | tags: [searchable tech |
| summary | requires | gsd-core/templates/summary.md:17 | requires: |
| summary | provides | gsd-core/templates/summary.md:20 | provides: |
| summary | affects | gsd-core/templates/summary.md:22 | affects: [list of phase names |
| summary | tech_added | gsd-core/templates/summary.md:33 | added: [libraries/tools added in this phase] |
| summary | tech_patterns | gsd-core/templates/summary.md:34 | patterns: [architectural/code patterns established] |
| summary | key_decisions | gsd-core/templates/summary.md:40 | key-decisions: |
| summary | patterns_established | gsd-core/templates/summary.md:44 | patterns-established: |
| summary | requirements_completed | gsd-core/templates/summary.md:48 | requirements-completed: [] |
| summary | threat_flags | agents/gsd-executor.md:740 | ## Threat Flags |
| summary | next_readiness | gsd-core/templates/summary.md:149 | ## Next Phase Readiness |
| summary | halted_because | rule:gsd/E402 |  |
| deliverable | summary | gsd-core/templates/summary.md:53 | coverage: |
| deliverable | description | gsd-core/templates/summary.md:55 | description: "[deliverable in human-readable form |
| deliverable | requirement | gsd-core/templates/summary.md:56 | requirement: "[REQ-ID from this plan's `requirements` |
| deliverable | human_judgment | gsd-core/templates/summary.md:61 | human_judgment: false     # REQUIRED boolean |
| deliverable | rationale | gsd-core/templates/summary.md:66 | rationale: "[REQUIRED when human_judgment: true |
| deliverable | checks | gsd-core/templates/summary.md:57 | verification: |
| deviation | summary | gsd-core/templates/summary.md:110 | ## Deviations from Plan |
| deviation | kind | gsd-core/templates/summary.md:133 | ## Issues Encountered |
| deviation | rule | gsd-core/templates/summary.md:118 | **1. [Rule X - Category] Brief description** |
| deviation | text | gsd-core/templates/summary.md:120 | - **Issue:** [What was wrong] |
| deviation | files | gsd-core/templates/summary.md:122 | - **Files modified:** [file paths] |
| pause | phase | gsd-core/workflows/pause-work.md:83 | "phase": "{phase_number}", |
| pause | plan | gsd-core/workflows/pause-work.md:86 | "plan": {current_plan_number}, |
| pause | reason | gsd-core/templates/state.md:98 | Stopped at: |
| pause | handoff | gsd-core/workflows/pause-work.md:131 | **Write handoff to the path determined in the detect step** |
| handoff | phase | gsd-core/workflows/pause-work.md:136 | phase: XX-name |
| handoff | task | gsd-core/workflows/pause-work.md:137 | task: 3 |
| handoff | state | gsd-core/workflows/pause-work.md:139 | status: in_progress |
| handoff | current_state | gsd-core/workflows/pause-work.md:165 | <current_state> |
| handoff | completed_work | gsd-core/workflows/pause-work.md:169 | <completed_work> |
| handoff | remaining_work | gsd-core/workflows/pause-work.md:177 | <remaining_work> |
| handoff | decisions_made | gsd-core/workflows/pause-work.md:184 | <decisions_made> |
| handoff | blockers | gsd-core/workflows/pause-work.md:190 | <blockers> |
| handoff | context | gsd-core/workflows/pause-work.md:213 | <context> |
| handoff | next_action | gsd-core/workflows/pause-work.md:217 | <next_action> |
| blocker | text | gsd-core/templates/state.md:81 | ### Blockers/Concerns |
| blocker | phase | gsd-core/templates/state.md:184 | - Prefix with originating phase |
| blocker | plan | agents/gsd-executor.md:426 | last_gate_trip: {plan_id}/{task_id} |
| blocker | resolved | gsd-core/templates/state.md:185 | - Cleared when addressed |
| quick_batch | items | gsd-core/workflows/quick-batch/steps/batch-init.md:46 | j.items.length |

## verify.md

| kind | field | source | the line holds |
|---|---|---|---|
| verification | phase | agents/gsd-verifier.md:687 | phase: XX-name |
| verification | deferred | agents/gsd-verifier.md:716 | deferred: # Only if deferred items exist |
| verify_gap | verification | agents/gsd-verifier.md:707 | gaps: # Only if status: gaps_found |
| verify_gap | truth | agents/gsd-verifier.md:708 | - truth: "Observable truth that failed" |
| verify_gap | reason | agents/gsd-verifier.md:710 | reason: "Why it failed" |
| verify_gap | artifacts | agents/gsd-verifier.md:711 | artifacts: |
| verify_gap | missing | agents/gsd-verifier.md:714 | missing: |
| override | verification | agents/gsd-verifier.md:695 | overrides: # Only if overrides exist |
| override | truth | agents/gsd-verifier.md:221 | normalize both the override `must_have` and the current truth |
| override | accepts | agents/gsd-verifier.md:696 | - must_have: "Must-have text that was overridden" |
| override | reason | agents/gsd-verifier.md:697 | reason: "Why deviation is acceptable" |
| override | by | agents/gsd-verifier.md:698 | accepted_by: "username" |
| override | at | agents/gsd-verifier.md:699 | accepted_at: "ISO timestamp" |
| human_item | verification | agents/gsd-verifier.md:734 | human_verification: # Only if status: human_needed |
| human_item | test | agents/gsd-verifier.md:735 | - test: "What to do" |
| human_item | expected | agents/gsd-verifier.md:736 | expected: "What should happen" |
| human_item | why_human | agents/gsd-verifier.md:737 | why_human: "Why can't verify programmatically" |
| human_item | uat | agents/gsd-verifier.md:537 | `human_needed` → `{phase_num}-UAT.md` path |
| uat_session | phase | gsd-core/templates/UAT.md:12 | phase: XX-name |
| uat_session | source | gsd-core/templates/UAT.md:13 | source: [list of SUMMARY.md files tested] |
| uat_session | started | gsd-core/templates/UAT.md:14 | started: [ISO timestamp] |
| uat_test | session | gsd-core/templates/UAT.md:27 | ## Tests |
| uat_test | expected | gsd-core/templates/UAT.md:30 | expected: [observable behavior - what user should see] |
| uat_test | reported | gsd-core/templates/UAT.md:40 | reported: "[verbatim user response]" |
| uat_test | severity | gsd-core/templates/UAT.md:41 | severity: major |
| uat_test | reason | gsd-core/templates/UAT.md:46 | reason: [why skipped] |
| uat_test | blocked_by | gsd-core/templates/UAT.md:51 | blocked_by: server \| physical-device |
| uat_gap | test | gsd-core/templates/UAT.md:72 | test: [N] |
| uat_gap | truth | gsd-core/templates/UAT.md:68 | - truth: "[expected behavior from test]" |
| uat_gap | reason | gsd-core/templates/UAT.md:70 | reason: "User reported: [verbatim response]" |
| uat_gap | severity | gsd-core/templates/UAT.md:71 | severity: blocker \| major \| minor \| cosmetic |
| uat_gap | root_cause | gsd-core/templates/UAT.md:73 | root_cause: ""     # Filled by diagnosis |
| uat_gap | artifacts | gsd-core/templates/UAT.md:74 | artifacts: []      # Filled by diagnosis |
| uat_gap | missing | gsd-core/templates/UAT.md:75 | missing: []        # Filled by diagnosis |
| validation | phase | gsd-core/templates/VALIDATION.md:2 | phase: "{N}" |
| validation | framework | gsd-core/templates/VALIDATION.md:22 | **Framework** |
| validation | config_file | gsd-core/templates/VALIDATION.md:23 | **Config file** |
| validation | quick_cmd | gsd-core/templates/VALIDATION.md:24 | **Quick run command** |
| validation | full_cmd | gsd-core/templates/VALIDATION.md:25 | **Full suite command** |
| validation | max_latency_s | gsd-core/templates/VALIDATION.md:35 | **Max feedback latency:** {N} seconds |
| validation_row | validation | gsd-core/templates/VALIDATION.md:39 | ## Per-Task Verification Map |
| validation_row | task | gsd-core/templates/VALIDATION.md:41 | Task ID \| Plan |
| validation_row | requirement | gsd-core/templates/VALIDATION.md:41 | Requirement \| Threat Ref |
| validation_row | test_type | gsd-core/templates/VALIDATION.md:41 | Test Type \| Automated Command |
| validation_row | command | gsd-core/templates/VALIDATION.md:41 | Automated Command \| File Exists |
| validation_row | file | gsd-core/templates/VALIDATION.md:41 | File Exists \| Status |
| wave0_item | validation | gsd-core/templates/VALIDATION.md:49 | ## Wave 0 Requirements |
| wave0_item | path | gsd-core/templates/VALIDATION.md:51 | `{tests/test_file.py}` |
| wave0_item | for | gsd-core/templates/VALIDATION.md:51 | stubs for REQ-{XX} |
| wave0_item | kind | gsd-core/templates/VALIDATION.md:53 | `{framework install}` — if no framework detected |
| manual_check | validation | gsd-core/templates/VALIDATION.md:59 | ## Manual-Only Verifications |
| manual_check | behavior | gsd-core/templates/VALIDATION.md:61 | Behavior \| Requirement |
| manual_check | requirement | gsd-core/templates/VALIDATION.md:61 | Requirement \| Why Manual |
| manual_check | why_manual | gsd-core/templates/VALIDATION.md:61 | Why Manual \| Test Instructions |
| manual_check | instructions | gsd-core/templates/VALIDATION.md:61 | Test Instructions |
| milestone_audit | milestone | gsd-core/workflows/audit-milestone.md:197 | milestone: {version} |
| milestone_audit | tech_debt | gsd-core/workflows/audit-milestone.md:216 | tech_debt:  # Non-critical, deferred |
| milestone_audit | debt_accepted | gsd-core/workflows/audit-milestone.md:356 | **A. Complete milestone** — accept debt |
| broken_window | kind | agents/gsd-executor.md:726 | --kind stub |
| broken_window | phase | agents/gsd-executor.md:727 | --phase "${PHASE_NUMBER}" |
| broken_window | file | agents/gsd-executor.md:728 | --file "<path-relative-to-repo-root>" |
| broken_window | line | agents/gsd-executor.md:729 | --line "<line-number-or-omit>" |
| broken_window | description | agents/gsd-executor.md:730 | --description "<one-line description |
| broken_window | refactor | computed:broken_window.status |  |
| refactor_proposal | phase | src/complexity-trigger.cts:908 | `phase: ${p.phase}`, |
| refactor_proposal | target_file | src/complexity-trigger.cts:909 | `target_file: ${p.target_file}`, |
| refactor_proposal | target_function | src/complexity-trigger.cts:910 | `target_function: ${p.target_function}`, |

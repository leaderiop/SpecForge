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
its kind, or given the capability that reads it. The test `tests/provenance.rs` holds the table to the declaration.

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
| stack_choice | rejected_because | agents/gsd-phase-researcher.md:304 | Instead of \| Could Use \| Tradeoff |
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

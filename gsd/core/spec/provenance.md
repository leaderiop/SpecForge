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

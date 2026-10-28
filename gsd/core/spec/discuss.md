# Discuss, specify, research

```
 phase ─1──0..1─ context ─1──*─ decision           phase ─1──0..1─ phase_spec ─1──*─ locked_req
                    ├──*─ deferred_idea                              └──*─ interview
                    └──*─ canonical_refs (a block of the context)
 phase ─1──0..1─ discussion (audit log, append-only)
 phase | project ─1──0..1─ research ─1──*─ finding | stack_choice | pattern | pitfall | dont_hand_roll
                                      └──*─ open_question            each item ─*─ source
```

Files: `context.spec`, `research.spec` in the phase folder; project research in `spec/gsd/knowledge/research/`.

## context

GSD: `NN-CONTEXT.md`, written by `/gsd-discuss-phase`. One per phase; scoped by it. Name `03-CONTEXT`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | R I | file location |
| gathered | date | I | `**Gathered:**` |
| boundary | text | R | `<domain>` `## Phase Boundary` |
| discretion | strs | | `### Claude's Discretion` |
| specifics | text | | `<specifics>` |
| canonical_refs | {topic str, paths files} | | `<canonical_refs>` (mandatory: not `R`, so gsd/E201 is the one diagnostic of its absence) |
| reusable_assets, patterns, integration_points | strs | | `<code_context>` |
| status | enum{gathering, ready_for_planning} | C | `**Status:**`: ready once canonical_refs and boundary are present and no interview is open |

## decision

GSD: `- **D-NN:**` in CONTEXT, PROJECT `Key Decisions` rows, SUMMARY `key-decisions`. Name `D-01`, scoped by its phase
(or by the project for project decisions).

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | I | owning CONTEXT; absent for a project decision |
| area | str | | `### [Area]` |
| text | str | R | the decision |
| rationale | str | | Key Decisions `Rationale` |
| outcome | enum{good, revisit, pending} | | `✓ Good` / `⚠️ Revisit` / `— Pending` |
| reversibility | enum{reversible, costly, one_way} | | planner reversibility rating |
| settled | bool | | locked: a plan contradicting it is refused (judged, [rules](rules.md)) |
| origin | enum{context, project, summary, checkpoint} | R I | where it was recorded |
| from_summary | → summary | | when `origin summary`; declared with `summary` (r38): a reference to a kind no extension declares is W021 |
| superseded_by | → decision | | a later decision that reverses it |

## deferred_idea

GSD: `<deferred>`. `text str R`, `phase → phase R`, `why str`. Excluded from every coverage rule.

## phase_spec, locked_req, interview

GSD: `NN-SPEC.md`, written by `/gsd-spec-phase`. Names `03-SPEC`, `LR1`, `IV1`.

**phase_spec**: `phase → phase R I`, `context_digest str H` (the digest of the phase's context, written by the command that records the spec; gsd/W406), `created date I`, `goal str R` (X changes from A to B), `background text`,
`in_scope strs`, `out_of_scope strs`, `constraints strs`, `acceptance strs`;
`scores {goal int{0..100}, boundary int{0..100}, constraint int{0..100}, acceptance int{0..100}, ambiguity int{0..100}}`
is `V`: the judged verdict of the `spec_phase` prompt (a `prompt:spec_phase` judge, K15), in percent (GSD's 0–1 score × 100), gated at
goal ≥ 75, boundary ≥ 70, constraint ≥ 65, acceptance ≥ 70, ambiguity ≤ 20.

**locked_req**: a numbered SPEC requirement. `spec → phase_spec R`, `title str R`, `current str R`, `target str R`,
`acceptance str R` (falsifiable), `traces →* requirement`.

**interview**: one `## Interview Log` row, append-only. `spec → phase_spec R`, `round int R`,
`perspective enum{researcher, simplifier, boundary_keeper, failure_analyst} R`, `question str R`, `answer str`,
`locked str`.

## discussion

GSD: `NN-DISCUSSION-LOG.md`. Name `03-DISCUSSION-LOG`. `phase → phase R I`;
`entries {area str, question str, options strs, chosen str, mode enum{interactive, assumptions, advisor}}` `+`.
An audit record: no agent context is built from it.

## research and its items

GSD: `NN-RESEARCH.md` (phase) and `research/{STACK, FEATURES, ARCHITECTURE, PITFALLS, SUMMARY}.md` (project). The
Markdown sections become typed items, so a pitfall or an open question can be referenced, checked and queried.

**research**: name `03-RESEARCH` (project research: `RESEARCH`); each item kind below is scoped by its `research` and named `F1`, `SC1`, `P1`, `PF1`, `DH1`, `OQ1`, `SRC1`, `PA1`.
`phase → phase` (absent for project research), `context_digest str H` (as on the spec; gsd/W406), `domain str`, `researched date I`,
`confidence enum{high, medium, low} R`, `summary text R`, `user_constraints →* decision`,
`responsibility_map {capability str, tier str}`.

| Item kind | Fields | GSD section |
|---|---|---|
| finding | `research R`, `topic enum{stack, features, architecture, pitfalls, other}`, `text str R`, `confidence enum{high, medium, low} R`, `sources →* source` | `## Summary`, project research files |
| stack_choice | `research R`, `name str R`, `version str`, `purpose str R`, `tier enum{core, supporting, alternative}`, `rejected_because str` | `## Standard Stack` |
| pattern | `research R`, `name str R`, `what str R`, `when str`, `example text`, `anti bool` | `## Architecture Patterns`, `### Anti-Patterns` |
| pitfall | `research R`, `name str R`, `goes_wrong str R`, `why str R`, `avoid str R`, `warning_signs str` | `## Common Pitfalls` |
| dont_hand_roll | `research R`, `problem str R`, `instead str R`, `why str R` | `## Don't Hand-Roll` |
| open_question | `research R`, `question str R`, `known str`, `unclear str R`, `recommendation str`, `resolved_by → decision` | `## Open Questions` |
| source | `url str R`, `tier enum{primary, secondary, tertiary} R`, `checked str` | `## Sources` |
| package_audit | `research R`, `package str R`, `registry str R`, `legitimacy enum{legitimate, suspicious, unknown}` `V` | the package-legitimacy table |

An `open_question` with no `resolved_by` is what the plan checker's research-resolution dimension flags; here it is a
rule over the graph, `gsd/W204` ([rules](rules.md)).

# Discuss, specify, research

```
 phase | plan(quick) ─1──0..1─ context ─1──*─ decision     phase ─1──0..1─ phase_spec ─1──*─ locked_req
                                  ├──*─ deferred_idea (a phase's only)            ├──*─ interview
                                  │                                               ├──*─ edge
                                  │                                               └──*─ spec_prohibition
                                  └──*─ canonical_refs (a block of the context)
 phase ─1──0..1─ discussion (audit log, append-only)
 phase | project | plan(quick) ─1──0..1─ research ─1──*─ finding | stack_choice | pattern | pitfall
                                                    ├──*─ dont_hand_roll | open_question
                                                    └── each item ─*─ source
```

Files: `context.spec`, `research.spec` in the phase folder; project research in `spec/gsd/knowledge/research/`; a quick
plan's context, decisions and research beside it (`/gsd-quick --discuss --research`).

The `0..1` of the diagram is enforced by `gsd/E206`: the kernel's scopes would allow a second context, spec, discussion log or research in a phase, a quick plan a second context or research, and the project a second research, so the second and later, by id, are reported. A quick plan has no phase, so its context, decisions and research name it in `plan` and are in the project scope, as it is; they are counted by the plan they name. E206 also holds the owner: a context names exactly one of `phase` and `plan`, a decision and a research at most one, and a `plan` must be a quick plan (one with no phase). A research with neither is the project's research and must be named `RESEARCH`; any other is reported too.

## context

GSD: `NN-CONTEXT.md`, written by `/gsd-discuss-phase`. One per phase; scoped by it. Name `03-CONTEXT`. A quick task's
`<quick_id>-CONTEXT.md` (`/gsd-quick --discuss`) is one per quick plan, names it in `plan`, and is in the project scope:
name `251101-abc-CONTEXT`.

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | I | file location; absent for a quick plan's context (not `R`: gsd/E206 requires exactly one of `phase` and `plan`) |
| plan | → plan | I | (quick) `quick/<id>/<id>-CONTEXT.md`: the quick plan, which has no phase |
| gathered | date | I | `**Gathered:**` |
| boundary | text | R | `<domain>` `## Phase Boundary` |
| discretion | strs | | `### Claude's Discretion` |
| specifics | text | | `<specifics>` |
| canonical_refs | {topic str, paths paths<present>} | | `<canonical_refs>` (mandatory: not `R`, so gsd/E201 is the one diagnostic of its absence, whether the field is missing or `[]`; a context with no external spec writes one ref whose `topic` is GSD's sentence, "No external specs — requirements are fully captured in decisions above", and whose `paths` is empty) |
| reusable_assets, patterns, integration_points | strs | | `<code_context>` |
| status | enum{gathering, ready_for_planning} | C | `**Status:**`: ready once `boundary` is written and `canonical_refs` has an item. GSD's "no interview is open" is not computable: an interview belongs to a `phase_spec` and has no state |

## decision

GSD: `- **D-NN:**` in CONTEXT, PROJECT `Key Decisions` rows, SUMMARY `key-decisions`. A decision of `origin summary` has no `from_summary` field: the summary names it by `summary.key_decisions`, and a relation is written from one side (W162); the graph derives the way back. Name `D-01`, scoped by its phase
(or by the project for project decisions; a quick context's decision is in the project scope too, names its quick plan
in `plan`, and is named `<quick_id>-D-01` so it cannot collide with a project decision).

| Field | Type | Flags | GSD |
|---|---|---|---|
| phase | → phase | I | owning CONTEXT; absent for a project decision |
| plan | → plan | I | owning quick CONTEXT: the quick plan whose context recorded it (a quick context's digest covers its `origin context` decisions; at most one of `phase` and `plan`, gsd/E206) |
| area | str | | `### [Area]` |
| text | str | R | the decision |
| rationale | str | | Key Decisions `Rationale` |
| outcome | enum{good, revisit, pending} | | `✓ Good` / `⚠️ Revisit` / `— Pending` |
| reversibility | enum{reversible, costly, one_way} | | planner reversibility rating |
| settled | bool | | locked: a plan contradicting it is refused (judged, [rules](rules.md)); read by gsd/W311, not by gsd/E310, which counts every trackable decision as GSD does and reads the plans' `decisions` lists |
| origin | enum{context, project, summary, checkpoint} | R I | where it was recorded |
| superseded_by | → decision | | a later decision that reverses it |

## deferred_idea

GSD: `<deferred>`. `text str R`, `phase → phase R`, `why str`. Excluded from every coverage rule.

## phase_spec, locked_req, interview, edge, spec_prohibition

GSD: `NN-SPEC.md`, written by `/gsd-spec-phase`. Names `03-SPEC`, `LR1`, `IV1`, `EC1`, `SP1`.

**phase_spec**: `phase → phase R I`, `context_pack {route str, digest str, max_tokens int} P` (the `gsd/phase-context` pack the spec was written against, recorded by the host for `record-spec`; gsd/W406), `created date I`, `goal str R` (X changes from A to B), `background text`,
`in_scope strs`, `out_of_scope strs`, `constraints strs`, `acceptance strs`;
`scores {goal int{0..100}, boundary int{0..100}, constraint int{0..100}, acceptance int{0..100}, ambiguity int{0..100}}`
is `V`: the judged verdict of the `spec_phase` prompt (a `prompt:spec_phase` judge, K15), in percent (GSD's 0–1 score × 100), gated at
goal ≥ 75, boundary ≥ 70, constraint ≥ 65, acceptance ≥ 70, ambiguity ≤ 20. It is `V`: the `values` of the `spec_phase` prompt's verdict on `gsd/E202` (bounds 0..100 enforced), so a hand-written `scores` is E082. `record_verdict` refuses a `passed` on `gsd/E202` whose `scores` are missing, outside 0..100 or outside the gates above.

**locked_req**: a numbered SPEC requirement. `spec → phase_spec R`, `title str R`, `current str R`, `target str R`,
`acceptance str R` (falsifiable), `traces →* requirement`, `shapes enum{numeric_range, collection, text, stateful, io}*`
(upstream's authored shapes, `-` written `_`: they override the edge probe's classifier, and an empty list proposes no
edge), `text_en str` (an English translation the edge probe classifies instead, when `response_language` is set).

**interview**: one `## Interview Log` row, append-only. `spec → phase_spec R`, `round int R`,
`perspective enum{researcher, simplifier, boundary_keeper, failure_analyst} R`, `question str R`, `answer str`,
`locked str`.

**edge**: one `## Edge Coverage` row: the edge probe's category of a locked requirement and its resolution (ADR 0333).
Written whole by `specforge gsd resolve-edge`, which keys a row by its requirement and category.

| Field | Type | Flags | GSD |
|---|---|---|---|
| spec | → phase_spec | R I | the SPEC the table is in |
| requirement | → locked_req | R | `Requirement` (a locked requirement of the same spec, gsd/E523) |
| category | enum{boundary, adjacency, empty, encoding, ordering, precision, idempotency, concurrency, unclassified} | R | `Category` |
| status | enum{resolved, dismissed, unresolved} | R | `Status` |
| verification | enum{explicit, backstop} | | `Verification`: a resolved row's tier, absent otherwise (gsd/E523) |
| resolution | str | | `Resolution`: the acceptance criterion or backstop note a resolved row requires |
| reason | str | | the reason a dismissed row requires |

**spec_prohibition**: one `## Prohibitions` row: a must-NOT statement of a locked requirement and its resolution. The
plan's `prohibition` is lifted from it. The `check_*` scalars are a resolved `test` row's check descriptor, read
fail-closed where it is used, not judged here.

| Field | Type | Flags | GSD |
|---|---|---|---|
| spec | → phase_spec | R I | the SPEC the table is in |
| requirement | → locked_req | R | `Requirement` |
| statement | str | R | `Statement` (the must-NOT sentence) |
| status | enum{resolved, dismissed, unresolved} | R | `Status` |
| verification | enum{test, judgment} | | `Verification`: a resolved row's tier, absent otherwise (gsd/E523) |
| reason | str | | the reason a dismissed row requires |
| check_kind | enum{node_test, lint_rule} | | `check_kind` (`node-test`, `lint-rule`) |
| check_target | str | | `check_target` |
| check_rule | str | | `check_rule` (a lint rule's) |
| check_violation_fixture | str | | `check_violation_fixture` |
| check_clean_fixture | str | | `check_clean_fixture` |

## discussion

GSD: `NN-DISCUSSION-LOG.md`. Name `03-DISCUSSION-LOG`. `phase → phase R I`;
`entries {area str, question str, options strs, chosen str, mode enum{interactive, assumptions, advisor}, notes str,
confidence enum{confident, likely, unclear}, evidence strs}` `+`: `notes` is the user's follow-up notes (upstream's
"**Notes:**") or a correction's reason; `confidence` and `evidence` are assumptions mode's Confidence and Evidence columns.
An audit record: no agent context is built from it. `checkpoint {timestamp ts, areas_completed strs, areas_remaining
strs, decisions {area str R, question str R, answer str, options_presented strs}*, deferred_ideas strs, canonical_refs
strs}`: GSD's `NN-DISCUSS-CHECKPOINT.json` (`templates/checkpoint.json`'s members), where an interrupted discussion
stands. The agent sets it with `specforge set` after each area and clears it with `specforge unset` once the context is
written; it is not append-only.

## research and its items

GSD: `NN-RESEARCH.md` (phase) and `research/{STACK, FEATURES, ARCHITECTURE, PITFALLS, SUMMARY}.md` (project). The
Markdown sections become typed items, so a pitfall or an open question can be referenced, checked and queried.

**research**: name `03-RESEARCH` (project research: `RESEARCH`; a quick plan's: `251101-abc-RESEARCH`, though any name
is allowed); each item kind below is scoped by its `research` and named `F1`, `SC1`, `P1`, `PF1`, `DH1`, `OQ1`, `SRC1`, `PA1`.
`phase → phase` (absent for project research and a quick plan's), `plan → plan I` (a quick plan's research,
`/gsd-quick --research`; at most one of `phase` and `plan`, gsd/E206), `context_pack P` (as on the spec; a quick
research records `gsd/quick-context`; gsd/W406), `domain str`, `researched date I`,
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
| source | `research R`, `url str R`, `tier enum{primary, secondary, tertiary} R`, `checked str` | `## Sources` |
| package_audit | `research R`, `package str R`, `registry str R`, `legitimacy enum{legitimate, suspicious, unknown}` `V` | the package-legitimacy table |

`package_audit.legitimacy` is `V`: the outcome of the phase researcher's verdict on `gsd/W205`, so a hand-written value is E082 (`specforge query <audit> --inputs gsd/W205`, then `specforge gsd record-verdict`). `context.status` is declared `C` and answered by the derive pass (r38): `ready_for_planning` when `boundary` is written and `canonical_refs` has an item, else `gathering`.

An `open_question` with no `resolved_by` is what the plan checker's research-resolution dimension flags; here it is a
rule over the graph, `gsd/W204` ([rules](rules.md)), for a phase's research and a quick plan's (upstream's quick planner reads its `RESEARCH.md` too): the plan checker reads the phase's `RESEARCH.md`, so the project's research (the one named `RESEARCH`) is exempt; a research with neither `phase` nor `plan` and another name is E206 and is judged as a phase's.

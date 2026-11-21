# A refusal names the entities owing the verdicts that block its runs

**Status:** accepted (2026-11-20). **Amends**
[ADR 0379](0379-gsd-strict-gates-fresh-verdicts-traced-requirements-and-task-trailers-and-is-measured.md): its
Consequences sentence that the refusal names every verdict refusing each run it names "by its holder and its judge".
Builds on [ADR 0377](0377-a-verdict-is-fresh-over-what-it-cites-and-owed-by-its-owner-state.md) D7 (one `gsd/E516` per
entity and rule, naming the gate's subject; the excusal of the verdicts a workflow judges itself) and ADR 0379's
Consequences (the remedy prompts each workflow declares, and the gate-free `specforge gsd record-verdict`, which a
verdict no prompt's run records again names alone). No protocol change, no kernel change, no new code and no new
setting.

## Context

- **A blocked remedy names where, not what.** A refusal whose remedy run is itself refused names what blocks that run
  by its holder and its judge only ("owed on v1.0/01 (gsd-verifier)"). A user facing a phase of seven must-haves
  cannot tell which entity to judge again.
- **A verdict no phase or milestone scopes gets a phantom holder.** The holder of a verdict on an entity no phase or
  milestone scopes (a quick plan's must-have, a package audit of a quick plan's research or of the project's
  research) falls back to the entity's outermost id, which the tail then presents as a holder ("owed on RESEARCH"),
  though no prompt runs there.
- **The message is read as it is.** The message ("the `<rule>` verdict on '`<entity>`' ...") already names its own
  entity, and many tests read its exact text.

## D1. The tail names the entities

Each blocking verdict is grouped by its holder and its judge (another extension's rule when it has no core judge), as
before, and the group names the entities that owe its verdicts, each by its full id between single quotes, as the
message writes it, in the order a person reads plan and task ids. An entity owing several rules of one judge is named
once. A group names only the entities whose verdict refuses every run the help names, read with the same rule the gate
reads, and is left out when none does.

The reason: the user records a verdict on an entity, by its id; the holder says where a prompt runs, the entity says
what to judge.

## D2. Five names per holder and judge

A group names at most five entities, then says how many more (", and 2 more").

The reason: a phase can hold every must-have of its plans; five keep the help readable and the count keeps it honest,
and a bound per group never hides a whole holder.

## D3. A verdict no phase or milestone holds is named by its own entity

A holder is an entity of a kind some remedy prompt runs on (a phase or a milestone, read from the prompt registry). A
group whose entities no holder scopes is named by those entities' ids, "by '`<id>`' itself", after the groups a phase
or milestone holds, and never by the outermost id it falls back to. Its own refusal keeps naming
`specforge gsd record-verdict` with its id, the route no gate refuses, as ADR 0379 says.

The reason: a holder in the tail tells the user where a prompt would run; naming an id no prompt runs on sends them
nowhere.

## D4. What does not change

The message, the static helps (an unreadable verdict, another extension's rule, a verdict no prompt's run records
again), when `gsd/E516` refuses and the count of diagnostics (one per entity and rule) stay as ADR 0377 D7 and ADR 0379
left them.

The reason: this decision is about what the help says, not about what the gate judges.

## Rejected

- **Naming the entities in the message itself.** The message already names its own entity, many tests read its exact
  text, and the tail is where the other verdicts are listed.
- **Naming (entity, rule) pairs.** A plan's four plan checks would be named four times; the judge already says which
  rules.
- **Ids relative to the holder.** Shorter, but not what `record-verdict` and `query` take, and not how the message
  spells them.
- **No bound.** A phase's every must-have in one line.
- **One bound for the whole tail.** It would hide whole holders.
- **Escaping ids in the tail alone.** The message echoes them raw, and the grammar's ids hold no quote, space or
  control character, so both spell one id one way.
- **Naming the outermost id as before.** It names a holder no prompt runs on (D3).

## Limits

- Past five, a group's entities are counted, not named; for the verifier's verdicts
  `specforge gsd re-verify <plan> --stale-only` lists them all.
- The tail lists only verdicts that refuse every named run, so a verdict refusing some of them is named by none.
- Within one help every named run is on one subject today, where the per-entity reading names what the per-group one
  did, with the entities added.

## What each decision becomes

"Amended" means the decision holds with the change named. "Kept" means it holds as written.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0379 | Consequences (the tail "by its holder and its judge") | amended | the entities, bounded (D1, D2, D3) |
| 0379 | Consequences (no prompt's run: `record-verdict` alone) | kept | its own refusal is unchanged (D3) |
| 0377 | D7 (one `gsd/E516` per entity and rule, naming the subject) | kept | the message and the count stay (D4) |

ADR 0379: 1 amended, 1 kept. ADR 0377: 1 kept.

## Consequences

- A refusal's help grows by at most five ids per holder and judge.
- A user can record the named verdicts one by one with `specforge gsd record-verdict`.
- The docs that describe the help say so.
- The `@gsd/core` blob is re-vendored: its source changed, its declaration did not.

The tests that hold each decision, by their path in `gsd/core`:

D1 (the entities): `passes::gates::e516::tests::tail::the_tail_names_each_entity_owing_a_verdict_that_blocks_the_runs`,
`passes::gates::e516::tests::tail::an_entity_owing_several_rules_of_one_judge_is_named_once`,
`passes::gates::e516::tests::tail::the_entities_are_named_in_id_order`,
`passes::gates::e516::tests::tail::only_the_entities_whose_verdict_refuses_every_named_run_are_named`,
`passes::gates::e516::tests::a_verdict_that_refuses_only_some_named_runs_is_not_listed` and, on the taskboard twin,
`e516_remedy::verdicts_of_two_judges_in_two_phases_name_each_other_and_the_gate_free_route`.

D2 (five names): `passes::gates::e516::tests::tail::five_entities_owing_one_holder_are_all_named`,
`passes::gates::e516::tests::tail::a_sixth_entity_is_counted_not_named`,
`passes::gates::e516::tests::tail::many_entities_name_the_first_five_in_id_order_and_count_the_rest`,
`passes::gates::e516::tests::tail::the_bound_is_per_holder_and_judge`,
`e516_remedy::the_seven_must_haves_of_a_stale_phase_are_named_five_and_two_more` and
`passes::gates::e516::tests::tail::the_docs_state_the_bound_the_tail_applies`, which fails when the bound in the code
and the number the docs and D2 state differ.

D3 (no holder): `passes::gates::e516::tests::tail::a_verdict_no_phase_holds_is_named_by_its_own_entity`,
`passes::gates::e516::tests::tail::held_groups_come_first_then_a_verdict_no_phase_holds`,
`passes::gates::e516::tests::tail::the_entities_of_one_outer_id_are_named_each_itself`,
`passes::gates::e516::tests::tail::the_groups_no_holder_scopes_follow_the_held_ones_in_id_order`,
`passes::gates::e516::tests::tail::the_bound_holds_for_the_entities_of_an_outer_id`,
`passes::gates::e516::tests::tail::a_holder_is_a_kind_a_remedy_prompt_runs_on` and, on the discuss project,
`e516_remedy::a_verdict_no_phase_holds_is_named_by_its_own_entity_and_recorded_without_a_gate`; with the behaviours
it keeps, `passes::gates::e516::tests::a_holder_the_gate_excuses_is_still_named_for_another_holder`,
`passes::gates::e516::tests::a_missing_verdict_names_a_stale_holder_elsewhere`,
`passes::gates::e516::tests::an_entity_no_phase_scopes_names_the_gate_free_route_and_refuses_the_named_runs` and
`passes::gates::e516::tests::an_unreadable_verdict_keeps_its_text_and_refuses_the_named_runs`.

D4 (what does not change): `passes::gates::tests::*`, unchanged; the message-only pins of `flow_strict::*`,
`prompts_gates::*`, `cmd_execute::*` and `verdict_freshness::*`; and the determinism and encoding tests
`passes::gates::e516::tests::tail::the_tail_is_the_same_whatever_the_input_order_and_on_repeat` and
`passes::gates::e516::tests::tail::an_entity_id_is_named_in_a_tail_as_its_message_names_it`.

## Would reopen it

- A request to name the entities in the message itself.
- A remedy prompt that runs on a quick plan: its kind would become a holder, read from the registry, and its verdicts
  would be named on it.
- A bound that should differ per judge.

# Personas and journeys

```
 persona ─1──*─ journey ─*──*─ milestone        (milestones: where it ships; the milestone reads `journeys`)
                   ├─*──*─ requirement          (requirements: what it exercises as a whole)
                   └─ steps[] ─*──0..1─ requirement
```

Files: `spec/gsd/product/personas.spec`, `spec/gsd/product/journeys.spec`.

GSD keeps no persona or journey file: these kinds are beyond parity
([ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)), and no
`gsd-compat` rule needs them. Each is unscoped, so its id is unique across the project and a reference to it never
needs a path.

## persona

Who the product serves. Name: a word, `developer`.

| Field | Type | Flags | Meaning |
|---|---|---|---|
| goals | strs | | what the persona wants to achieve |
| context | str | R | who they are and the situation they work in |
| pain_points | strs | | what gets in their way today |
| technical_level | enum{expert, advanced, intermediate, beginner, non_technical} | | technical proficiency, most technical first |
| status | enum{active, deprecated} | | `deprecated` when the product no longer serves them; absent means active |
| tags | strs | | free words that group personas |

The persona reads back the journeys that name it as `journeys` (journey.persona): a relation is written from one side,
and writing `journeys` on a persona is E079.

## journey

A persona's path through the product, step by step. Name: a word, `sign_in`.

| Field | Type | Flags | Meaning |
|---|---|---|---|
| persona | → persona | R | who takes the journey; the persona reads it back as `journeys` |
| channels | strs | | the surfaces it goes through, as words (`cli`, `web`): a channel is no kind |
| steps | {action str R, requirement → requirement} | R | the ordered steps; a step names the requirement it exercises once known |
| requirements | →* requirement | | the requirements it exercises as a whole; a step's `requirement` is the finer link |
| milestones | →* milestone | | every milestone it ships in; each milestone reads it back as `journeys` |
| priority | enum{must, should, could} | | MoSCoW; a journey nobody wants is not written |
| tags | strs | | free words |
| last_uat | → uat_session | C | the session of a UAT test that names it (`uat_test.journey`): the latest `started`, an undated session older than every dated one, then the greatest id; absent when no test names it |
| covered | bool | C | it names a requirement, each one it names is proven by the requirement thread (complete, and its thread whole: the predicate gsd/E515 reads), and each step names one or the journey lists `requirements`; a reference that does not resolve is unproven |
| tests | →* test | C | the `@code/core` test facts tagged with it (`[gsd:<journey>]`), run or not, by id, sorted; `[]` when none. Evidence beside `covered`, which reads the requirement thread only |

A step's requirement is the journey's edge `steps.requirement`, resolved from the journey outward (ADR 0285 D4). A
journey lists every milestone it ships in rather than being scoped by one: a scope is one reference (ADR 0284 D1).

The journey reads back as `uat_tests` the UAT tests that walk it (`uat_test.journey`); `last_uat` is the latest of
their sessions (ADR 0357 D22).

## What the old product kinds became

The old `@specforge/product` declared `persona`, `journey`, `term`, `channel`, `deliverable` and `milestone`, with
product commands, rules that reported an absence, and presentation keywords.
[ADR 0357](../../../docs/adr/0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md) maps each of
their fields to its home here or on the milestone ([project.md](project.md#milestone)): a channel is a word of
`journey.channels`, a deliverable is a milestone whose kind of artifact is a word of `artifact_types`, and a field no
tracked project writes and that has no home is refused rather than dropped. The commands and the absence rules are not
carried: `specforge query`, `list` and `trace` answer what the commands answered.

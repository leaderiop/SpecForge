# Personas, journeys and the glossary

```
 persona ─1──*─ journey ─*──*─ milestone        (milestones: where it ships; the milestone reads `journeys`)
                   └─ steps[] ─*──0..1─ requirement
```

Files: `spec/gsd/product/personas.spec`, `spec/gsd/product/journeys.spec`.

GSD keeps no persona, journey or glossary file: these kinds are beyond parity, and no `gsd-compat` rule needs them.
Each is unscoped, so its id is unique across the project and a reference to it never needs a path.

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
| milestones | →* milestone | | every milestone it ships in; each milestone reads it back as `journeys` |
| priority | enum{must, should, could} | | MoSCoW; a journey nobody wants is not written |
| tags | strs | | free words |

A step's requirement is the journey's edge `steps.requirement`, resolved from the journey outward (ADR 0285 D4). A
journey lists every milestone it ships in rather than being scoped by one: a scope is one reference (ADR 0284 D1).

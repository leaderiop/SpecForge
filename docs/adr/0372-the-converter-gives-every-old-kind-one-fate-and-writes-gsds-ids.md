# The converter gives every old kind one fate and writes GSD's ids

**Status:** accepted (2026-11-17). **Amends**
[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) D1 (ids are GSD's, not kept: D3), D2
(the homes are To, Computed or Dropped: D2), D3 (the ledger's classes and its format 2: D8), D4 (a reference is
written again, through the id map, and one to an entity the run does not convert is dropped, never stubbed: D4) and
D8 (nothing waits: D2); [ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md)
D3 and D4 (carried out: D4, D8); and
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D8's "how each is written is the conversion
change's design" (decided here: D1 to D8). Builds on
[ADR 0365](0365-a-module-is-dropped-and-no-kind-is-added-to-hold-it.md) (a module is dropped) and
[ADR 0368](0368-a-bound-is-part-of-a-requirements-text-and-the-prove-pass-and-the-protocol-do-not-change.md) D2 (a
bound is part of a requirement's text). No protocol, SDK or kernel change comes with this decision, `@gsd/core` does
not change, and no builtin is re-vendored: the converter is `xtask/src/absorb/`. D8's root half retired by
[ADR 0374](0374-the-root-spec-runs-on-gsd-core-and-its-tests-name-requirements.md) D7 after it last held.

## Context

**What was left.** ADR 0364 decided that the software kinds are dropped and that a census gives every old kind its
fate; ADR 0370 decided which kinds and fields `@gsd/core` holds and which old values are dropped. Both left the
conversion itself to a later change: how each old kind is written, under which ids, and how the ledger proves that
nothing was lost silently. This ADR is that change's record. The owner decided on 2026-11-16 that the converted ids
are GSD's with a map from old to new, and that a release becomes a version milestone named by its version; the
recommendations of 2026-11-17 settled the rest, each recorded below.

**What the converter reads.** The root `spec/` holds 2,291 entities: 17 old kinds and the `spec` block, no id shared
by two kinds. Of its 18 milestones, 13 are listed by its 3 releases; 5 are listed by none. Of its 117 decisions, 100
are in force (`accepted` 100, `superseded` 11, `proposed` 3, `deprecated` 3; one carries `superseded_by`). Its 63
constraints name seven category words (reliability 34, performance 17, usability 5, portability 3, compatibility 2,
interoperability 1, security 1). Four old kinds, `behavior`, `event`, `type` and `port`, were never read by absorb at
all: its reader kept only the 13 kinds the table named, and skipped the others silently.

**What shapes D7.** Once the converted target holds one `phase`, `gsd/E101` (every `v1` requirement is in exactly one
phase's `requirements`) reports every `v1` requirement no phase lists: 275 on the whole spec (63 features, 63
constraints, 149 invariants). A typed edit is judged by the errors it adds (E080), so the one edit plan that writes the
conversion would be refused whole. The links cannot place them: of 149 invariants, 73 reach no phase through the
behaviors that cite them and 36 reach several, so "exactly one" is no rule the links can give.

## D1. Every old kind has one fate

The 17 old kinds each have one entry in the converter's table. Six convert: a feature, a constraint and an invariant
to a `requirement`; a decision in force to a `decision`; a release to a `milestone`; a milestone a converted release
lists to a `phase`. Eleven are dropped whole, each with its reason:

- a deliverable: GSD's deliverable is a summary's coverage item, a summary needs a plan, and the old spec holds no
  plan;
- a failure mode: it becomes no requirement and no threat (a threat needs a plan);
- a journey, a term, a persona, a channel, a module, a behavior, an event, a type and a port: gsd-core has no kind for
  them.

absorb reads every authored entity of the source; an entity of a kind with no fate stops the run.

A `ref` declaration, the core grammar's outside resource (`ref gh.issue:42 "…"`), is no old kind, but absorb reads it
like one and gives it a fate: it is dropped whole, its title and each metadata field ledgered `dropped`. It is not
carried to the target: no `@gsd/core` kind holds it, a typed edit cannot create one, and every list that named it is
dropped (its scheme references among them), so a carried `ref` would be one nothing points at. The root spec holds
none; the fixture holds one, in block form, and the census counts it.

| Old kind | Entities | Fate | Converted | Dropped whole |
|---|---|---|---|---|
| `feature` | 141 | requirement | 141 | 0 |
| `constraint` | 63 | requirement | 63 | 0 |
| `invariant` | 149 | requirement | 149 | 0 |
| `decision` | 117 | decision, when in force | 100 | 17 |
| `release` | 3 | milestone | 3 | 0 |
| `milestone` | 18 | phase, when a converted release lists it | 13 | 5 |
| `deliverable` | 12 | dropped | 0 | 12 |
| `failure_mode` | 35 | dropped | 0 | 35 |
| `journey` | 54 | dropped | 0 | 54 |
| `term` | 100 | dropped | 0 | 100 |
| `persona` | 6 | dropped | 0 | 6 |
| `channel` | 5 | dropped | 0 | 5 |
| `module` | 54 | dropped | 0 | 54 |
| `behavior` | 706 | dropped | 0 | 706 |
| `event` | 123 | dropped | 0 | 123 |
| `type` | 678 | dropped | 0 | 678 |
| `port` | 26 | dropped | 0 | 26 |
| **Total** | **2,290** | | **469** | **1,821** |

With the `spec` block, which the target scaffolds fresh, that is the 2,291. No entity is created that the old spec did
not hold.

## D2. Homes are To, Computed or Dropped

The table holds rows for the six converting kinds only, one per field the old builtins register, and each row's homes
are To (an authored field of a `@gsd/core` kind, through a named way of writing), Computed (the new kind computes it;
the old value is compared after the write) or Dropped (with its reason). No row waits and none refuses.

What stops a run before any write is the refused class: an entity of a kind with no fate, a field of a converting
kind with no row, two entities given one new id, a category word with a letter outside ASCII (D3), an id the target
refuses, a required value no rule fills. An empty required value (a blank text, an empty list) is a missing one: a
phase whose name holds no ASCII letter or digit gets no slug, and a constraint whose only text is `""` gets no `text`,
so the stop names each.

A source whose old compile reports W060 is refused too, before any write, as one with errors is, each W060
declaration named. W060 is a second declaration of an id another kind declared first: the old graph keeps the first
and drops the second, so absorb could neither convert nor ledger it, and the run would lose it silently. The source
renames one first. The root spec and the fixture hold none.

An entity dropped whole ledgers its title and each of its fields, `verify` lines, `method` members and field
annotations `dropped`, with its fate's reason.

## D3. Ids are GSD's, and the ledger maps old to new

- **A requirement** is `<CAT>-<NN>`. Its category is a constraint's own `category` word capitalised, else the old
  kind's name (`Feature`, `Invariant`, `Constraint`). The prefix is the category's ASCII letters and digits
  upper-cased, or the old kind's name when that is empty, starts with a digit, or is the decisions' `D`. The heading
  and the prefix therefore agree letter for letter for a category led by an ASCII letter (`Data-integrity`,
  `DATAINTEGRITY-01`), and not otherwise (`9lives` is `CONSTRAINT-NN`). A category word with a letter or digit
  outside ASCII stops the run before any write, naming the constraint: its prefix would lose those letters with no
  signal (`Sécurité` would be `SCURIT`), so it is spelt in ASCII in the source first. Numbers run from 1 per prefix in
  (old kind, old id) order, at least two digits and as many as the group's size needs. On the root spec:
  `FEATURE-001` to `FEATURE-141`, `INVARIANT-001` to `INVARIANT-149`, `RELIABILITY-01` to `-34`, `PERFORMANCE-01` to
  `-17`, `USABILITY-01` to `-05`, `PORTABILITY-01` to `-03`, `COMPATIBILITY-01` to `-02`, `INTEROPERABILITY-01`,
  `SECURITY-01`.
- **A decision** is `D-<NN>` at project scope, numbered in old-id order: `D-001` to `D-100`.
- **A milestone** is `v<version>`, with no second `v` when the version has one: `v0.1.0`, `v0.5.0`, `v1.0.0`.
- **A phase** is `<NN>` under its version milestone, numbered in source order (file, line) within its release and
  keyed by its path (`v0.1.0/01`): 5, 5 and 3 phases. Its create's id is the number alone, placed by its `milestone`
  field, since a name holds no `/`.

Every id is assigned before any draft, so a reference names an entity whatever order the entities are drafted in. The
map is the ledger's `ids` section: total and one to one over the converted entities, and a re-run gives the same ids.

## D4. A release becomes a version milestone, and its milestones phases under it

- **The release.** Its title is the milestone's `name`, its description its `goal`, its changelog its `delivered`;
  its status is compared with the milestone's computed `state` (planned, in_progress, released as shipped).
- **Each milestone it lists.** Its title, less a leading `Phase N:`, is the phase's `name`; its slug is filled from
  that name; its description is the `goal`; its exit criteria are its `success_criteria` (its description the one
  criterion when it has none); its features are its `requirements` and its dependencies its `depends_on`, both
  through the id map; its status is compared with the phase's computed `status` (planned as not_started,
  in_progress, completed as complete).
- **A dependency** is kept only when it names an earlier phase of the same version milestone. On the root spec 7 are
  kept and 5 dropped.
- **A milestone no converted release lists** is dropped whole: no version milestone is made for it.
- **A reference to an entity the run does not convert** (outside the selection, dropped whole, or becoming another
  kind) is ledgered `dropped` with its reason, never stubbed.
- **A reference names the old kind its field references**: a milestone's features a feature, its dependencies and a
  release's milestones a milestone. Every listing is keyed by the new path it resolves to, never by the bare old id.
- Every other release and milestone value is dropped with its reason.

The phase's `milestone`, `requirements` and `depends_on` are the only homes that write a reference.

**What the typed edit showed.** The converter's first run of this design on its fixture wrote two version milestones
and three phases in one edit plan, judged clean (0 errors, 0 warnings): `Edit::Create` takes a phase's `milestone` as
one value (`v0.1.0`) and its `requirements` and `depends_on` as lists, a dependency given as the path `v0.1.0/01` is
printed `[01]` inside its own milestone, and a phase is created before the milestone it names when its file sorts
first, since one plan resolves its forward references. The target's computed values are read by the new path:
`first_cut`'s phase, `v0.1.0/01`, reads its computed `status` as `not_started`. Neither needed a change outside
`xtask/`.

## D5. Only a decision in force converts

A decision is in force when its status is `accepted` and it carries no `superseded_by`. It writes `text`, `rationale`,
`settled true` and `origin project`. Every other decision is dropped whole, `docs/adr/` named as its record.

## D6. A requirement's text holds its source's words

- **A constraint's text** is its description, then its metric with its threshold appended to the metric's last line
  (ADR 0368 D2: the bound is part of the text).
- **An invariant's text** is its guarantee, then its description.
- **A feature's text** is its problem, its description and its solution. No entity is created for a solution.

An invariant's `verify` lines, risk, references and formal expression, a constraint's `verify` lines, `constrains` and
`enforced_by`, and a feature's acceptance and tests are dropped with their reasons.

## D7. A requirement no written phase lists is `v2`

When the run writes a phase, a requirement whose tier would be `v1` and that no written phase lists is written `v2`
and ledgered `filled`, once, so `gsd/E101` holds. A run that writes no phase keeps the tiers status gives. On the root
spec that is 69 requirements `v1` and 284 `v2` (9 by status, the rest by this rule).

`gsd/E101` also reports a `v1` requirement several phases list. A requirement several written phases list is in one
phase: the first in source order (file, line) that lists it keeps it, keeping its tier, and each later listing is
ledgered `dropped`, naming that phase. This is `gsd/E101`'s own suggestion ("keep it in one phase and remove it from
the others"), and the rule holds whatever the tier, so a phase's requirements never depend on a tier. Demoting the
requirement to `v2` is rejected: it would unlist a requirement the old spec planned twice. The root spec has none.

## D8. Every ledger entry has one outcome, and a census holds it

The classes are `computed`, `filled`, `unconverted` and `dropped`. `unconverted` holds only the target's own scaffold
entries, its `spec` block and its `specforge.json`, each with the outcome "written fresh by absorb". A scheme reference
in a list is dropped.

The ledger is format 2: `format`, `from`, `entries`, then `ids`, an array of `{"kind", "old", "new"}` sorted by the old
kind and id, `new` the new id or, for a phase, its path.

A census test, pure, holds per old kind over the repository's own spec and over absorb's fixture: entities = converted
+ dropped whole, 0 unaccounted, and every kind in the spec has a fate. A `ref` declaration is counted like a kind.

## What each amended decision becomes

"Amended" means the decision holds with the change named. "Carried out" means a decision that left its design to a
later change now has it.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0360 | D1 | amended | ids, titles and file paths were kept; ids are now GSD's, with the map in the ledger (D3); titles, file paths, the project's name, version and spec root are still kept |
| 0360 | D2 | amended | the homes are To, Computed or Dropped; Later and Refused rows go, and the refused class is the run's stops before any write (D2) |
| 0360 | D3 | amended | the ledger's classes are `computed`, `filled`, `unconverted` and `dropped`, with one outcome per entry, and it is format 2 with the `ids` section (D8) |
| 0360 | D4 | amended | a phase's references are written again, through the id map; one to an entity the run does not convert is ledgered `dropped`, never stubbed (D4) |
| 0360 | D8 | amended | nothing waits for a kind: every value has a home or is dropped with its reason (D1, D2) |
| 0364 | D3 | carried out | a release becomes the version milestone its version names and each milestone it lists a phase under it; its other values are dropped (D4) |
| 0364 | D4 | carried out | the census holds per old kind over the root spec and the fixture (D8) |
| 0370 | D8 | carried out | how each fate is written is decided: the fates, the homes, the ids, the release and its milestones, the decisions in force, the texts, the tiers and the ledger (D1 to D8) |

## Rejected

- **A catch-all phase for the requirements no phase lists.** It is an entity added to port the old spec.
- **Placing them in the last phase.** It fabricates a delivery the old spec never planned.
- **Changing `gsd/E101`.** It is a `@gsd/core` change after the boundary that left `@gsd/core` as gsd-core defines it.
- **Converting no phase.** The releases' milestones would have no home, against the decision that a release becomes
  a version milestone with its milestones as phases.
- **A version milestone made for a milestone no release lists.** It is an entity added to port the old spec.
- **A failure mode as a requirement or a threat.** A requirement made from one is an entity added; a threat needs a
  plan.
- **A deliverable converted.** It needs an invented plan and summary.
- **A decision made from a feature's solution.** It is an entity added, and its id is not GSD's `D-NN`.
- **A four-letter category prefix.** `Compatibility` and `Compliance` collide at `COMP`, and a collision needs a second
  rule.
- **Phase numbers parsed from titles.** A title without one (`Groundwork`) must still convert; source order needs no
  parsing.
- **Keeping a cross-release `depends_on`.** GSD resolves a phase's `depends_on` within its own milestone, though
  `gsd/E102`, which reports only a forward reference inside one milestone, would pass it.

## Consequences

- 95 `done` features read `v2`: their old status survives in the ledger's `computed` entries.
- The ledger grows to 20,130 entries on the whole spec (`dropped` 19,306, `filled` 678, `computed` 138,
  `unconverted` 8), of which 16,190 are the software kinds' values (behavior 11,075, type 4,320, event 544, port 251),
  every one ledgered. Its `ids` section holds 469 entries, one per converted entity.
- Phase numbers restart in each version milestone, and a selection numbers its own subset: an `--only` run numbers
  from 1, so its ids are not those of a whole-spec run.
- No requirement takes the decisions' prefix `D`: the target holds one id per entity whatever its kind, so a
  requirement `D-01` beside the decision `D-01` would be one entity, and the edit plan refused. A constraint whose
  category word is `d` is `CONSTRAINT-NN`, its heading still `D`. Two entities of one old kind and id are given one
  id, so the map's one-to-one stop still names them.
- **The failure modes.** None converts, and the ledger keeps all 35 with their values. For a person to author by hand
  at the swap, as a security requirement, if wanted, the eight candidates, each with its severity and the invariant it
  threatens:

  | Failure mode | Severity | Invariant it threatens |
  |---|---|---|
  | `host_function_type_violation` | critical | `host_function_type_safety` |
  | `wasm_extension_crash` | high | `wasm_sandbox_integrity` |
  | `wasm_memory_exhaustion` | high | `wasm_sandbox_integrity` |
  | `wasm_host_function_timeout` | medium | `wasm_sandbox_integrity` |
  | `extension_initialization_failure` | high | `extension_isolation` |
  | `wasm_compile_cache_corruption` | medium | `wasm_compile_cache_integrity` |
  | `collector_output_malformation` | medium | `collector_output_conformance` |
  | `registry_unavailability` | medium | `registry_integrity` |

### As built, measured on the repository's own spec

**The census.** A pure test over the root spec and over absorb's fixture pins D1's table per old kind: entities =
converted + dropped whole, 0 unaccounted, and a kind with no fate fails it. On the root spec: 2,290 authored entities
(2,291 with the `spec` block), 469 converted (353 requirements, 69 `v1` and 284 `v2`; 100 decisions; 3 milestones;
13 phases) and 1,821 dropped whole, every count as D1 to D7 planned. On the fixture: 40 entities (its one `ref`
declaration among them), 17 converted and 23 dropped whole. The census's root half reads the old spec and goes with the self-spec sample when the root spec is
converted; its fixture half stays.

**The whole-spec trial.** absorb ran on a copy of the repository with the facts of its collectors recorded and the
three `@rust` extensions loaded (`--extension @rust/cargo --extension @rust/source --extension @rust/test`):

| Measure | This decision as built | Before it, same collectors | First baseline |
|---|---|---|---|
| Wall time | 9.01 s | 8.40 s | 66.47 s |
| Peak memory | 1,053,472 kB | 886,228 kB | 931,960 kB |

The first baseline joined proofs over the tests the old spec cited; that join left with the kinds that held a proof,
and the run fell to 8.40 s before this decision. The rise since then is this decision's: absorb now reads the four
software kinds it used to skip and ledgers their 16,190 values. Neither figure is a gate.

The run exited 0 with 469 creates in one edit plan (3 compiles), the ledger above and every unconverted entry the
scaffold's, each "written fresh by absorb". Per old kind its converted and dropped-whole counts equal the census.
`specforge check` of the target exits 0 under `gsd-compat` (the target names no profile, and `gsd-compat` is the
default): 0 errors, 7 warnings, 0 infos. All 7 are `gsd/W103`, the phases whose old milestone had more than five exit
criteria (`v0.1.0/04` 8, `v0.5.0/02` 8, `v0.5.0/03` 7, `v0.5.0/04` 11, `v0.5.0/05` 8, `v1.0.0/01` 10, `v1.0.0/02`
9); a phase has 2 to 5, and a warning is GSD's advice to merge or split, not a refusal.

## Would reopen it

- gsd-core gaining a release or a risk kind.
- `gsd/E101` changing upstream.

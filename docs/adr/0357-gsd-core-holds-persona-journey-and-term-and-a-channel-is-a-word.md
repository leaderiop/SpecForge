# `@gsd/core` holds persona, journey and term, and a channel is a word

**Status:** accepted (2026-11-14). Extends [ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md)
D4 (a reference inside a block), [ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md)
(a field that names its reverse) and [ADR 0279](0279-a-builtin-closes-a-fields-words-only-when-they-fit-any-domain.md)
D1 (open words). No kernel, SDK or protocol change. Extends
[ADR 0287](0287-who-writes-a-field-is-declared.md) (a computed reference list, held beside the graph, D17) and
[ADR 0331](0331-the-gsd-requirement-thread.md) (a journey's coverage reads the requirement thread, D20).

## Context

- `@gsd/core` modelled no persona, journey or glossary entry: GSD keeps no such file, and its milestone carries only
  GSD's fields (a name, what it delivered, its goal and target features, and the computed state, dates and stats).
- The old `@specforge/product` declares `persona`, `journey`, `term`, `channel`, `deliverable` and `milestone`. Beside
  their fields it carries 14 product commands (`personas`, `journeys`, `terms`, `channels`, `persona_features`,
  `persona_channels`, `channel_features`, `channel_coverage_matrix`, `coverage_matrix`, `journey_coverage`,
  `term_graph`, `term_clusters`, `term_density`, `deliverable_personas`), 8 rules that report an absence (W042, I010,
  I046, I047, I050, I068, I069, I070), one cycle check (E015 on a milestone's `depends_on`) and presentation keywords
  (`semantic_token`, `lsp_icon`, `dot_*`, `inference_guide`).
- Its milestone also carries planning facts GSD never writes: exit criteria, a start and a target date, the milestones
  it depends on, an owner, contributors and tags. A deliverable says what kind of thing ships (`extension`, `library`,
  `service`, `cli`) and which journeys it serves.
- SpecForge's own spec writes them all: 6 personas, 54 journeys, 100 terms, 5 channels, 12 deliverables and
  18 milestones. Its 6 kinds have 48 fields, plus the `tags` every product kind shares: 54 (kind, field) rows.
- A project that loads `@specforge/product` beside `@gsd/core` registers a kind twice, which is E026; the first
  registration wins.

## Decisions

- **D1. `@gsd/core` declares `persona`, `journey` and `term`** ([spec/product.md](../../gsd/core/spec/product.md)).
  GSD keeps no such file, so they are beyond parity: no GSD file renders them and no `gsd-compat` rule needs them. Each
  is unscoped, so its id is project-wide.
- **D2. A channel is a word.** `journey.channels` is a list of open words: no channel vocabulary fits every domain
  (ADR 0279 D1). There is no channel kind. An old channel entity's own text becomes a term of the same id: its title
  becomes the term's name, its description the definition, and its interaction model a word of its tags.
- **D3. A journey's steps are an ordered block list `{action, requirement}`.**
  - A step's requirement is optional, because a converted flow names none.
  - The step's requirement is the journey's edge `steps.requirement`, resolved from the journey outward
    (ADR 0285 D4).
  - `requirements` names what the journey exercises as a whole: the old journey's features, whose requirements no
    step pairs with.
- **D4. A journey lists every milestone it ships in.**
  - The field is `milestones →* milestone`, read back by the milestone as `journeys` (ADR 0354).
  - It is a list, not a scope: a scope is one reference
    ([ADR 0284](0284-a-name-is-unique-within-its-scope.md) D1), and SpecForge's own spec ships 8 journeys in 2 to 4
    deliverables each.
  - A journey's id is project-wide, and a reference to it never needs a path.
- **D5. Priority is MoSCoW:** `must`, `should`, `could` (`kinds/product.rs`'s `PRIORITY`). "Won't" is no journey at
  all. The requirement kind gains a `priority` of the same three words; it declares none yet.
- **D6. A term carries its word in `name`,** because a pass sees no entity's title.
  - `aliases` are accepted names and `avoid` the words not to use; one is never the other. A term whose `avoid` lists
    a word the glossary accepts, its own `name` or one of its `aliases` or another term's, is `gsd/W538`, a warning
    under every profile. The words compare with each run of whitespace as one space and Unicode lowercase. The
    warning sits on the avoiding term and names every term and field that accepts the word.
  - `related` is a relation of terms with themselves (ADR 0354's kind-pair rule pairs two different kinds only).
- **D7. A persona has a status,** `active` or `deprecated`, and absent means active. A deprecated persona says the
  product no longer serves them; nothing else reads it.
- **D8. A milestone carries the old milestone's planning fields, beyond GSD.** These are `exit_criteria`, `starts`,
  `targets`, `depends_on`, `owner`, `contributors`, `artifact_types` and `tags`
  ([spec/project.md](../../gsd/core/spec/project.md#milestone)).
  - Upstream GSD's milestone has none of them (`templates/milestone.md`, `templates/roadmap.md`), so all are optional,
    no GSD file renders them and no ported prompt asks for them.
  - `depends_on` is acyclic: a cycle is `gsd/E102`, the code GSD's phase and plan cycles already use. The old E015
    was `@specforge/product`'s.
  - `artifact_types` is the home of the old deliverable's `artifact_type`. A deliverable is a milestone (D12), and
    what kind of thing a milestone ships is a fact of its own, which a query filters on.
    - Open words, because ADR 0279 D1 applies: the old list was a suggestion on an identifier, and a pharma project
      ships batch records.
    - A list, because one milestone can ship a library and its CLI.
    - Not `tags`, which would bury the meaning, and not a package, which is the code rather than its kind.
- **D9. No command zoo.** The 14 product commands are not carried. `specforge query`, `list` and `trace` answer them,
  and a journey's coverage becomes a computed value of its own.
- **D10. No orphan rules.** W042, I010, I046 and I047 (no edges), I050 (an empty flow: `steps` is required), I068 (tag
  spelling), and I069 and I070 (a deprecated status without its reason) are not carried. A rule fires on a
  contradiction, not on an absence.
- **D11. No kernel namespacing.** A project that loads `@specforge/product` beside `@gsd/core` is E026 on five kinds
  (`milestone`, `deliverable`, `persona`, `journey`, `term`), and the first registration wins. A converted project
  loads `@gsd/core`, `@specforge/checks` and `@code/core`, never an old stack.
- **D12. Each old product deliverable becomes its own milestone.**
  - Its title becomes the milestone's name and its description the milestone's goal.
  - Its artifact type becomes one word of `artifact_types`.
  - Every journey it lists names that milestone in `milestones`, and all memberships are kept.
- **D13. The carry-over table** below. Zero loss: every authored value has a home, a mapping, a computed value or a
  later home. A value equal to the field's absent default (`Default`) is implied rather than written, and any other
  value of such a field stops the conversion. Only presentation leaves.
  - A field that no tracked project writes and that has no home is *refused*: the conversion stops on a value of it,
    naming the entity and the field, rather than drop it.
  - "Later" is for module references only: a module becomes a `@code/core` package when modules move there.
- **D14. A milestone's dates are in order.** A milestone that writes both `starts` and `targets`, with `targets`
  before `starts`, is `gsd/E537` ("milestone '<id>' targets <day> before it starts <day>").
  - It is an error under every profile, because it is a contradiction.
  - The same day is clean, and so is a milestone missing either date. The dates compare as calendar days.
  - A text that is no `YYYY-MM-DD` day is the kernel's E061 alone. The kernel reports it but still passes its text to
    the pass, so `gsd/E537` judges only two calendar days and adds no second error, false or misleading, to the first.
  - The old milestone had no such rule. A converted milestone whose dates are out of order is reported, not altered.

### The carry-over table

"Self-spec values" counts the entities of SpecForge's own spec that write the field.

| Old field | Self-spec values | Home |
|---|---|---|
| persona (id, title) | 6 | persona, id and title kept |
| persona.description | 6 | persona.context |
| persona.technical_level | 6 | persona.technical_level (same five words) |
| persona.goals | 6 | persona.goals |
| persona.pain_points | 6 | persona.pain_points |
| persona.status | 6, all `active` | persona.status |
| persona.reason | 0 | refused |
| persona.key_features →* feature | 0 | `personas`, a field the requirement kind gains, of each feature's requirement |
| persona.tags | 6 | persona.tags |
| journey (id, title) | 54 | journey, id and title kept |
| journey.persona | 54 | journey.persona |
| journey.description | 0 | refused |
| journey.channels →* channel | 20 | journey.channels, each channel's id as a word |
| journey.features →* feature | 54 (81 links) | journey.requirements, each feature's requirement |
| journey.flow | 54 | journey.steps, one `{action}` per line, in order |
| journey.priority | 54 | journey.priority: critical → must, high → should, medium and low → could |
| journey.tags | 54 | journey.tags |
| term (id, title) | 100 | term, id and title kept; term.name ← the title |
| term.definition | 100 | term.definition |
| term.context | 50 | term.context |
| term.aliases | 79 | term.aliases |
| term.see_also →* term | 68 | term.related |
| term.module → module | 0 | later: a `@code/core` package |
| term.tags | 39 | term.tags |
| channel (id, title) | 5 | a term of the same id, name ← the title |
| channel.description | 5 | that term's definition |
| channel.interaction_model | 5 | a word of that term's tags |
| channel.url | 0 | refused |
| channel.status | 5, all `active` | Default(`active`): `active` is implied and nothing is written; any other word stops the conversion, never a quiet drop |
| channel.reason | 0 | refused |
| channel.tags | 5 | that term's tags |
| deliverable (id, title) | 12 | its own milestone, id kept, name ← the title |
| deliverable.description | 12 | milestone.goal |
| deliverable.artifact_type | 12 | milestone.artifact_types, one word |
| deliverable.status | 0 | milestone.state, computed (the conversion compares the words) |
| deliverable.journeys →* journey | 12 lists, 65 listings | journey.milestones of each listed journey, every listing kept |
| deliverable.modules →* module | 12 | later: `@code/core` packages |
| deliverable.version | 0 | refused |
| deliverable.milestones | 0 | refused |
| deliverable.depends_on | 0 | milestone.depends_on |
| deliverable.reason | 0 | refused |
| deliverable.owner | 0 | milestone.owner |
| deliverable.contributors | 0 | milestone.contributors |
| deliverable.tags | 0 | milestone.tags |
| milestone (id, title) | 18 | milestone, id kept, name ← the title |
| milestone.description | 18 | milestone.goal |
| milestone.status | 18 | milestone.state, computed (the conversion compares the words) |
| milestone.features →* feature | 18 (81 links) | milestone.target_features, as words |
| milestone.exit_criteria | 18 (108 values) | milestone.exit_criteria |
| milestone.target_date | 15 | milestone.targets |
| milestone.start_date | 15 | milestone.starts |
| milestone.modules →* module | 16 | later: `@code/core` packages |
| milestone.depends_on →* milestone | 17 (18 links) | milestone.depends_on |
| milestone.blockers | 0 | refused |
| milestone.priority | 0 | refused |
| milestone.reason | 0 | refused |
| milestone.owner | 18 | milestone.owner |
| milestone.contributors | 18 | milestone.contributors |
| milestone.refs | 0 | refused |
| milestone.tags | 18 (35 values) | milestone.tags |

Presentation (`semantic_token`, `lsp_icon`, `dot_*`, `inference_guide`) leaves.

Of the 54 field rows: 37 have a home, 10 of them through a mapping (the channel words, the steps, the priority, the
features' requirements, a channel as a term, the artifact word, the journey memberships, the target features);
2 become computed values; 3 wait for packages; 11 are refused; 1 is `Default(active)`.

## Words

- **D15. One word matcher, two boundaries.** `passes/words.rs` holds `has_word(haystack, needle, Boundary)`, the one
  test of a word or phrase in prose, and `folded`, the one reading of a word the glossary compares.
  - `Boundary::Js` is JavaScript's ASCII `\b`: bytes, a word character is an ASCII letter, digit or `_`, and both
    sides are ASCII-lowercased. `gsd/E312` reads its phrases so, as upstream's planner rule does.
  - `Boundary::Unicode` is the vocabulary's reading. Both sides are folded as `gsd/W538` folds a word: each run of
    whitespace is one space, then `str::to_lowercase`. A word character is a letter or a number
    (`char::is_alphanumeric`), `_`, or a combining mark (U+0300–036F, U+1AB0–1AFF, U+1DC0–1DFF, U+20D0–20FF,
    U+FE20–FE2F), so a decomposed "é" never splits its word. A boundary is needed only on an edge where the needle's
    own character is a word character, so `C++` is found before a space or a full stop. Every occurrence counts,
    overlapping ones too.
  - Under both, an empty needle matches nothing. There is no stemming, no accent stripping and no normalisation: a
    word in NFC does not match the same word in NFD. Lowercase mapping is not full case folding: "STRASSE" does not
    match "straße", and Turkish dotted and dotless i stay distinct letters.
- **D16. The prose fields.** One table, `passes/vocabulary.rs`'s `PROSE`, names what the vocabulary reads: a
  requirement's `text`; a decision's `text`, `rationale` and `context`; a truth's `text`; each journey step's
  `action`; a risk's `cause` and `effect`; an invariant's `statement`. They are read in the author's language:
  `text_en` is not read, nor a term's `definition`, a persona's goals or a milestone's goal.
- **D17. `term.used_in` is computed.** It lists the entities whose prose (D16) holds the term's `name` or one of its
  `aliases` as a whole word or phrase (D15, `Unicode`), by id, sorted, and `[]` when the term is unused, so "none" is
  never "unknown". An `avoid` word is never a use.
  - It is a reference list to the six prose kinds. A computed value is held beside the graph and makes no edge
    (ADR 0287), so `specforge query <term>` shows it and `specforge refs` or `trace` do not.
- **D18. `gsd/W534`: prose writes a word a term avoids.** A prose field (D16) that holds a word or phrase a term lists
  in `avoid` (D15, `Unicode`) is one warning per entity and avoided word, the word compared as `gsd/W538` compares it.
  - It sits on the entity, and names the first field that holds the word, every term that avoids it and the word each
    asks for (its `name`).
  - It is a warning under every profile: vocabulary is advice.
  - The term side is `gsd/W538` (D6). An avoided word the glossary also accepts is W538's alone and never W534: the
    glossary contradicts itself, so it cannot say which word the prose should write. A term is no prose.

## Journey coverage

- **D19. The requirements a journey names** are each step's `requirement` (the edge `steps.requirement`) and the
  journey's own `requirements`, each once.
- **D20. `journey.covered` is computed.** It is true when the journey names a requirement, each one it names is proven
  by the requirement thread ([ADR 0331](0331-the-gsd-requirement-thread.md): complete and its thread whole,
  `thread::requirement_proven`, the predicate `gsd/E515` reads), and each step names a requirement or the journey
  lists `requirements`.
  - A journey that names none is not covered: no vacuous truth.
  - A step with no requirement of its own is carried by the journey-level list only; without one, it leaves the
    journey uncovered.
  - A reference that does not resolve (the host reports it) counts as unproven.
- **D21. `journey.tests` is computed.** It lists the `@code/core` test facts tagged with the journey
  (`[gsd:<journey>]`, an edge `names`), by id, sorted, and `[]` when none. They are evidence beside `covered`, never
  read by it: one reading of coverage, `gsd/E515`'s. A fact's status is read on the fact.
- **D22. A UAT test names the journey it walks.** `uat_test.journey` is authored and optional, read back as
  `journey.uat_tests` ([ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md)).
  - `journey.last_uat` is computed: the session of a test that names the journey, the latest `started` first, a
    session with no `started` older than every dated one, then the greatest id. Any result counts; the session's own
    `status` says how it went.
  - Both writers of a `uat_test` write it: the UAT template's fence, which `/gsd-verify-work` runs, and
    execute-phase's human-verification fence. Each sets a `JOURNEY` variable and passes
    `${JOURNEY:+--journey "$JOURNEY"}`, so a test with no journey writes the same command as before. The test-list step
    of `/gsd-verify-work` says how to find the journeys of a phase.
- **D23. `milestone.journeys_covered` is computed:** a block `{journeys, covered}` holding the journeys whose
  `milestones` list the milestone and how many of them are covered (D20). It is absent when no journey lists it.
- **D24. `gsd/W530`: a journey's requirement is outside a milestone it lists.** For each milestone a journey lists in
  `milestones`, each requirement it names (D19) that no phase of that milestone lists in `requirements` is one warning
  on the journey. It names the requirement, the steps that need it (or "the journey's requirements") and the milestone.
  - An unphased requirement is outside every milestone.
  - Each listed milestone is a promise of its own, so a requirement outside two of them is two warnings, and two
    steps on one requirement are one.
  - A journey that lists no milestone is silent, and so is a step with no requirement (`covered` says it).
  - It is a warning under `gsd-compat` and an error under `gsd-strict` and `gsd-plus`: the milestone cannot deliver
    the journey.

## Rejected

- A channel kind: a channel has no behaviour of its own in any project tracked, and its text is a term's.
- A `persona.name` field: the title holds it, and no pass reads it.
- A journey scoped by its milestone (D4).
- Aliases mapped into `avoid`: an alias is an accepted name, the opposite of an avoided one.
- Kernel namespacing of kinds (D11).
- The product commands as `gsd` commands (D9).
- Artifact types as a closed enum, a tag or a package (D8).
- A new code for milestone cycles: `gsd/E102` already says "dependency cycle".
- `targets` before `starts` as a warning: it is a contradiction, so it is an error (`gsd/E537`, D14).
- UAX #29 word segmentation (D15): it keeps "l'Élève" one word, so French elision misses, and it adds a dependency.
- Stemming or lemmatising (D15): each is language-specific. An inflection goes in `aliases` or `avoid`.
- Reading `text_en` (D16): the vocabulary is the author's language.
- One `used_in_<kind>` field per kind (D17): the host admits a computed union list.
- `covered` as requirement `status complete` (D20): that says only that a plan finished.
- A journey-tagged test making a journey covered (D21): two readings of coverage.
- A required `milestones` list (D24): an imported journey has none.

## Breaking

- A project that loads `@specforge/product` beside `@gsd/core` now also fails E026 on `persona`, `journey` and `term`.

## Would reopen it

- A domain whose channels are entities with behaviour of their own.
- A GSD release whose milestone gains fields of the same names.

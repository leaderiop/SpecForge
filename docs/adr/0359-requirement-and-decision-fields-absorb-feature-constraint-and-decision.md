# Requirement and decision fields absorb feature, constraint and decision

**Status:** accepted (2026-11-15). Amends [ADR 0331](0331-the-gsd-requirement-thread.md) D5 (a requirement's thread
state is its computed `proof`, which `gsd/E515` reads), [ADR 0353](0353-a-pass-reads-the-facts-it-declares.md)
D6 (`gate_e515` declares no facts any more: it reads that `proof`),
[ADR 0150](0150-a-relation-is-written-on-one-side.md) D2 (a reverse an `inverse_of` names is the other side by name
on the field's own kind, a self-relation, and on each kind of a union) and
[ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) D7 (`risk.mitigated_by` reads back as `mitigates` on
task, truth and invariant). Builds on
[ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md) (an enum, a block),
[ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md) (a field that names its reverse),
[ADR 0093](0093-a-fields-declared-type-decides-what-its-value-references.md) (a scheme ref in a string list),
[ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) (a pass reads the facts it declares) and
[ADR 0361](0361-a-kind-field-may-be-a-number-an-exact-decimal-written-as-one-word.md) (a decimal number field). No
SDK or protocol change: every field is a `@gsd/core` declaration, and every typed edit (`specforge new`, `set`, `link`,
an edit plan) takes it with no wiring of its own. One host change, in the field registry (`crates/pipeline/registry`;
no kernel crate, no re-vendor): the ADR 0150 D2 amendment above.

## Context

- A `@gsd/core` requirement is GSD's `REQUIREMENTS.md` bullet: its text, its tier and its heading, upstream's
  `### [Category]`, which it held in a required string named `category`. `decision.area` names the same notion for a
  decision (`### [Area]`).
- The old `@specforge/product` `feature` and `@specforge/governance` `constraint` are what a requirement is in GSD's
  model, and they carry what a requirement could not hold. Across SpecForge's own spec, a feature writes `problem`
  and `solution` 141 times, `priority` (critical … low) and `effort` (xs … xl) 9 times each, `owner`, `contributors`
  and `acceptance` 8 times each, `tags` 6 times; a constraint writes a quality `category`, a `priority` and a prose
  `metric` 63 times each, `constrains` 63 times and `protects` 44 times. Their categories are reliability (34),
  performance (17), usability (5), portability (3), compatibility (2), interoperability (1) and security (1).
- The heading collides with the quality class: both want the word `category`.
- A `@gsd/core` decision is GSD's `D-NN` and a Key Decisions row. The old `@specforge/governance` decision is an
  ADR record: SpecForge's own spec holds 117, each with a `context`, a `decision` and its `consequences`, 105 with a
  `date`, 42 naming the invariants they shape, 12 with `tags`, 10 with `alternatives`, and 11 `superseded`. A
  `@gsd/core` decision held none of these but its text and its rationale, and it said what reversed it on the old
  record (`superseded_by`), where an ADR writes it on the new one.
- The field registry recognised the other side of a relation by name only through a field of another kind with one
  target kind ([ADR 0150](0150-a-relation-is-written-on-one-side.md) D2), so a reverse named on a self-relation
  (`requirement.constrains`, a decision that supersedes another) or on a union (`risk.mitigated_by`) was an unknown
  field when written: W020, the relation dropped silently.
- Whether a requirement's thread is whole (a task of its plans, a commit of one, a passing test) was read in one place,
  `gsd/E515`'s private loop at a gate, from the test facts the guard was sent. No surface could show it, and a second
  reader (a journey's coverage) would have copied the loop.

## Decisions

### The requirement

- **D1. `area` is the heading.** Upstream's `### [Category]` heading is the required string `area`, the word
  `decision.area` uses for the same notion. Every site that writes a requirement writes `area "…"` and
  `specforge new requirement --area`, and every asset line that names the heading calls it an area, upstream's kept
  text included (`[AREA]-[NUMBER]`, "scope each area", the roadmapper's "group by area"): an agent told `[Category]`
  writes the heading into `category`, the quality class, where it is E061. The new-milestone hand-off to the
  roadmapper labels each value with the field it fills (`area:`, `category:`, `priority:`).
- **D2. `category` is the quality class**, an optional enum of ten words: ISO/IEC 25010:2011's eight product quality
  characteristics by their short names (`functional`, `performance`, `compatibility`, `usability`, `reliability`,
  `security`, `maintainability`, `portability`), `safety`, which ISO/IEC 25010:2023 added, and `regulatory`. The
  2023 revision also renamed usability to *interaction capability* and portability to *flexibility*; the kind keeps
  the 2011 words, the ones SpecForge's own constraints already use (`usability` 5 times, `portability` 3), so a
  reader mapping against the 2023 standard reads `usability` as interaction capability and `portability` as
  flexibility. Interoperability is part of compatibility in both editions, so an old `interoperability` constraint
  is `compatibility`.
- **D3. `priority` is MoSCoW**: `must`, `should`, `could`, optional. "Won't" is `tier out_of_scope`, which a
  requirement already has. Upstream's FEATURES.md P1/P2/P3 is the precedent. The list is the journey's
  (`kinds::product::PRIORITY`): one MoSCoW list for both kinds.
- **D4. `metric` is a record**: `name`, `threshold` and `direction` (`at_most`, `at_least`, `exactly`) required, `unit`
  optional. A threshold without a direction measures nothing; a plain count has no unit, and a made-up one would lose
  meaning. The threshold is a `number` (a decimal, ADR 0361) with no bounds: a latency is positive, a drift may be
  negative, and the direction carries the sense. A metric written as prose is E061, not parsed: the field is a
  record.
- **D5. `source` is a string list** of where the requirement came from: a person, quoted (`"Alice Martin"`), or a
  declared `ref` written bare (`gh.issue:42`), which names that entity (ADR 0093) and is E003 when undeclared. People
  (the old `owner` and `contributors`) are quoted names: no person scheme.
- **D6. Three relations name their reverse** (ADR 0354), so W162 stays silent: `personas` (→* persona, read back as
  `requirements`), `constrains` (→* requirement, its reverse named `constrained_by`) and `protects` (→* invariant,
  read back as `protected_by`). Read back means the name written on the other side is E079, naming the owning field,
  on a persona, an invariant and a requirement alike. `constrains` is a self-relation: the field registry recognises
  its named reverse since a self-relation's and a union's `inverse_of` are read by name (amending
  [ADR 0150](0150-a-relation-is-written-on-one-side.md) D2), so a `constrained_by` written on a requirement is E079,
  not an unknown field. None is named `risks`, the reverse `risk.affects` declares on a requirement.
- **D7. `tags`**: free words to group it by, as on the other kinds that had them.
- **D8. `satisfied_by` is computed from the written `task.items`** of every task of the plans that list the
  requirement, any task type (a `manual` task's item too): sorted, each once, absent when none. It reads the items
  as written, as `gsd/W522` does, so an item a pending task plans to write counts before a collect holds it, and the
  compute pass needs no item facts (ADR 0353). They are the items `specforge trace` reaches through the
  requirement's plans.
- **D9. `proof` is the thread's state**, computed: `no_task`, `no_commit`, `not_run`, `no_test`, `failed`, `proven`,
  the first missing link of plan → task (not `manual`) → commit → a collected test run, then what the tests linked to
  a committed task, to the requirement, or named in the `proof` of a truth that names it say. Six states, not four:
  a requirement no plan lists is `no_task`, not `not_run`. `passes::thread::requirement_proof` is the one reading;
  the derive pass writes it once per requirement, and `requirement_proven(status, proof)` (status `complete` and
  proof `proven`) is the one predicate. `gsd/E515` reads the computed `status` and `proof` from the gate's graph and
  refuses a complete requirement for which the predicate is false, naming the first missing link as before: it
  declares no test facts any more (amends ADR 0331 D5's caller table: the requirement's `proof` is the caller E515
  was).
- **D10. `effort` is a T-shirt size**: `xs`, `s`, `m`, `l` or `xl`, optional, the old feature's estimate.

### The decision

- **D11. The later decision names the ones it reverses.** `decision.supersedes` (→* decision) is written on the
  new record, as an ADR's "Supersedes" line is, and names `superseded_by` its reverse: each superseded decision reads
  it back as the incoming edge `query` shows. No decision declares a `superseded_by` field: written by hand it is E079
  naming `decision.supersedes`, with the edit, and `specforge set` or `link` of it is refused `other_side`.
- **D12. A self-relation's and a union's named reverse is the other side by name** (amends ADR 0150 D2).
  `FieldRegistry::owning_fields`, which answers both E079 and the typed edit's `other_side`, finds every reference
  field, of any kind (the entity's own kind included), whose target kinds (one, or a union's) hold the entity's kind
  and whose `inverse_of` is the name written. Recognition by values is unchanged: a self-relation or a union is never
  guessed from ids alone, so an unknown field that merely lists ids of its own kind stays W020. W162's rule (a) reads
  each kind of a union, so a union cannot name a reverse one of its kinds declares. The amendment also makes D6's
  `constrained_by` real.
- **D13. `alternatives` is the checkpoint's option record plus `why_not`.** `kinds::OPTION` (`name` required,
  `pros`, `cons`) is the checkpoint's `options` record; `kinds::ALTERNATIVE` is its three fields and an optional
  `why_not`, so the two cannot drift. A checkpoint's options keep exactly three members: they are offered before the
  choice, and none has lost yet. `why_not` is optional, since an old governance alternative is a bare name.
- **D14. `affects` names what a decision shapes**: `requirement`, `invariant` and `@code/core`'s `item` (an optional
  peer, as in `task.items`), each reading it back as `affected_by`. **The union rule:** a kind joins `affects` only in
  the change that declares the kind, because a union member no loaded extension declares turns every unresolved id of
  the union into I004 rather than E003. A test pins the list; the change that declares a new member moves that pin.
  `risk`, `phase` and `milestone` are not members: no old value maps there.
- **D15. The ADR record's fields**: `context` (str, what prompted it), `consequences` (strs, what follows, good and
  bad), `decided` (date, immutable, like `context.gathered`: a change is a new decision that supersedes it),
  `deciders` (strs) and `tags` (strs). All are optional and authored. `deciders` records who decided, not an attested
  sign-off, so it is not human-only: a human-only field would refuse a one-time conversion's write.
- **D16. A superseded context decision is not trackable, under every profile.** `passes::plans::context_decisions`,
  the one set `gsd/E310`, `gsd/W317` and `gsd/W506` read, leaves out every decision a `supersedes` edge with force
  targets (`passes::plans::superseded`). GSD has no supersedes, so no GSD-written project holds one and compat's
  output on every GSD project is unchanged; upstream's own escape (`[informational]`, "if it should not be tracked")
  is the author's act, and here superseding a decision is that act. A plan that still lists a superseded decision
  stays silent: that needs a code of its own. Two edges have no force:
  - **One that would retire a settled decision from an agent.** A `settled` decision is locked with the user, so it
    leaves force only through a decision taken with the user too: `origin` `context` or `project`. A `summary` or
    `checkpoint` decision, which an agent records, does not retire it; it still retires an unsettled one. The
    executor's summary-decision line offers `--supersedes` and says so: reversing a settled decision is a deviation
    for the user.
  - **One inside a supersedes cycle.** A decision that supersedes itself, directly or round a cycle, is reversed by
    nothing outside it, so an edge between two decisions of one cycle retires neither, and every decision of the
    cycle stays tracked; the graph's W061 reports the cycle. An edge from a cycle to a decision outside it still
    retires that decision. The cycles are read after the settled rule, over the edges with force.

  The same function answers `decision.in_force` (computed, bool: false for a superseded decision), so every surface
  reads one answer. A prompt section that serves decisions (the planner's `decisions`, the plan checks'
  `phase_decisions` and `all_decisions`) cannot leave a superseded one out, since a selector filters on no computed
  field and on no incoming edge (ADR 0356 D13); it serves each decision with its `in_force`, and the planner's and
  plan checker's prompts hold locked only the decisions in force.
- **D17. The asset lines offer the relations.** Every asset line that writes a decision (the context, project and
  walking-skeleton templates, the PRD express path, the quick discussion step, the executor's summary decisions, the
  transition and the graduation) gives `--supersedes` and `--affects` (comma-separated ids) and `--alternatives` (one
  JSON list, read from a quoted heredoc so an apostrophe is text), each only when it holds something
  (`${VAR:+--flag "$VAR"}`). The ADR-shaped fields are not on asset lines: no GSD step holds their values; the
  conversion and `specforge set` write them.
- **D18. A risk's controls read it back as `mitigates`** (amends ADR 0358 D7). `risk.mitigated_by` (→* task | truth |
  invariant) names `mitigates`, now that a union's named reverse is read on each of its kinds: a task, a truth or an
  invariant reads back the risks it controls, and `mitigates` written on one is E079 naming `risk.mitigated_by`. The
  word is not `risks`, which `affects` and `milestone` already name and which on a task would not say whether the task
  creates the risk or controls it.

### Converting the old governance decision

The one-time conversion of an `@specforge/governance` decision reads this table; its measure counts it.

| Old | New | Note |
|---|---|---|
| (kind) decision | decision, `origin project` | no phase: a project decision, named by its old id |
| decision | text | |
| context | context | |
| consequences | consequences | |
| date | decided | |
| deciders | deciders | |
| tags | tags | |
| reason | rationale | |
| alternatives (strings) | alternatives `[{name: <string>}]` | |
| invariants | affects | the converted invariant; a reference that leaves the converted set is the conversion's ledger |
| affects_features | affects | the requirement the feature becomes |
| constraints | affects | the requirement the constraint becomes |
| superseded_by X on D | `supersedes [D]` on X | the direction flips |
| status accepted | settled true | |
| status proposed | outcome pending | |
| status deprecated | outcome revisit | |
| status superseded | computed | read back as `superseded_by` from the successor's `supersedes`, and `in_force` false; one that names no successor has no field to land in, so the conversion ledgers it |
| refs, description, authors | no field | unused in every tracked project |

## Rejected

- `category` kept as the heading, with the quality class named `quality`: the heading is `decision.area`'s notion,
  and the old constraint's `category` is the quality class.
- An integer threshold in the unit's smallest step: `0.25` is written as such, and a decimal is a kernel type.
- `source` as `rfs_any(["ref"])`: `ref` is the grammar's structural declaration, not a kind an extension declares,
  and a person has no entity to name.
- E515 recomputing the thread at the gate: two readings of one fact, and the gate's would be invisible elsewhere.
- `satisfied_by` read from resolved item edges: the derive pass would declare every item fact for no gain, and a
  pending task's planned item would not count.
- A computed `superseded_by` field beside `supersedes`: a field and an `inverse_of` of one name on one kind is W162,
  and two stores of one fact is what ADR 0150 removed.
- Keeping the authored `superseded_by`: it writes on the old record, and ADR practice writes the new one.
- Leaving a self-relation's or a union's written reverse at W020: ADR 0150 rejected E079 as a warning because the
  relation written there is lost silently, and this is that case. A gsd-only check instead: a host rule in a guest,
  and it would miss the typed edit's `other_side`.
- Widening recognition by values to a self-relation or a union: unrelated unknown fields would become errors.
- `why_not` on the checkpoint's option record: a checkpoint's options are offered before the choice, and none has
  lost yet.
- `deciders` as a human-only field: it records who decided, not an attested sign-off, and would refuse the one-time
  conversion's write.
- Counting a superseded decision under gsd-compat and exempting it under gsd-strict and gsd-plus: a profile raises a
  severity or turns a rule off, never narrows what a rule judges, and compat would demand that a plan list a decision
  its author reversed, whose only way out is deleting the record `supersedes` keeps.
- Dropping `--supersedes` from the executor's summary-decision line: an executor's decision may still reverse an
  unsettled one, and the rule belongs in the one function every pass reads, not in one prompt.
- Untracking a supersedes cycle: a self-reference would turn E310, an Error, into W061, a Warning `check` passes.
- Leaving superseded decisions out of the prompt sections by selector: the field filter reads authored scalar fields
  only (a judged digest is taken before the compute passes), and `superseded_by` is an incoming edge, no field; a new
  selector form for it would restate the settled and cycle rules in the kernel, which knows neither.
- `risks` as `mitigated_by`'s reverse: `risk.affects` and `risk.milestone` already name it, and on a task it would not
  say whether the task creates the risk or controls it.

## Consequences

- Every new field is a `specforge new requirement` flag, a `set`/`link` target and an edit-plan field, with no wiring.
- `gsd/E515` sheds its test facts; it and the requirement's `proof` agree by construction, on every surface.
- A later reader of the thread (a journey's coverage, say) reads the computed `proof` and the one predicate, not
  the tests.
- An old governance decision converts with no value lost but `refs`, `description` and `authors`, which no tracked
  project writes; the 10 superseded decisions of SpecForge's spec that name no successor are the conversion's ledger.
- A kind added to `decision.affects` waits for its declaration (the union rule), and moves the pin that holds the
  list.
- A supersedes cycle keeps every decision of it tracked, and W061 reports it; a plan that lists a superseded decision
  is silent, which needs a code of its own.
- An agent cannot take a user-locked decision out of the coverage gates by superseding it.

## Would reopen it

- A requirement satisfied by code no task names: a coverage reading over the requirement's items would have to say
  which tests exercise them.
- A decision that reverses only part of another: `supersedes` untracks the whole of the earlier one, so a partial
  reversal would need its own relation.

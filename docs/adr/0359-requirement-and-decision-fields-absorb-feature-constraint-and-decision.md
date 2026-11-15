# Requirement and decision fields absorb feature, constraint and decision

**Status:** accepted (2026-11-15). Amends [ADR 0331](0331-the-gsd-requirement-thread.md) D5 (a requirement's thread
state is its computed `proof`, which `gsd/E515` reads). Builds on
[ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md) (an enum, a block),
[ADR 0354](0354-a-reference-field-that-names-its-reverse-is-its-own-relation.md) (a field that names its reverse),
[ADR 0093](0093-a-fields-declared-type-decides-what-its-value-references.md) (a scheme ref in a string list),
[ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) (a pass reads the facts it declares) and
[ADR 0361](0361-a-kind-field-may-be-a-number-an-exact-decimal-written-as-one-word.md) (a decimal number field). No
SDK, protocol or host change: every field is a `@gsd/core` declaration, and every typed edit (`specforge new`, `set`,
`link`, an edit plan) takes it with no wiring of its own.

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
- **D2. `category` is the quality class**, an optional enum of ten words: ISO/IEC 25010:2023's nine product quality
  characteristics by their short names (`functional`, `performance`, `compatibility`, `usability`, `reliability`,
  `security`, `maintainability`, `portability`, `safety`) and `regulatory`. Interoperability is part of
  compatibility in that standard, so an old `interoperability` constraint is `compatibility`.
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
- **D6. Three relations name their reverse** (ADR 0354), so the other side reads them back and W162 stays silent:
  `personas` (→* persona, read back as `requirements`), `constrains` (→* requirement, read back as `constrained_by`)
  and `protects` (→* invariant, read back as `protected_by`). None is named `risks`, the reverse `risk.affects`
  declares on a requirement.
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

## Rejected

- `category` kept as the heading, with the quality class named `quality`: the heading is `decision.area`'s notion,
  and the old constraint's `category` is the quality class.
- An integer threshold in the unit's smallest step: `0.25` is written as such, and a decimal is a kernel type.
- `source` as `rfs_any(["ref"])`: `ref` is the grammar's structural declaration, not a kind an extension declares,
  and a person has no entity to name.
- E515 recomputing the thread at the gate: two readings of one fact, and the gate's would be invisible elsewhere.
- `satisfied_by` read from resolved item edges: the derive pass would declare every item fact for no gain, and a
  pending task's planned item would not count.

## Consequences

- Every new field is a `specforge new requirement` flag, a `set`/`link` target and an edit-plan field, with no wiring.
- `gsd/E515` sheds its test facts; it and the requirement's `proof` agree by construction, on every surface.
- A later reader of the thread (a journey's coverage, say) reads the computed `proof` and the one predicate, not
  the tests.

## Would reopen it

- A requirement satisfied by code no task names: a coverage reading over the requirement's items would have to say
  which tests exercise them.

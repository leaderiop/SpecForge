# The self-spec moves to @gsd/core by a one-time converter

**Status:** accepted (2026-11-15). Builds on [ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)
(the product kinds and their carry-over table), [ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) (risk,
invariant and the word scale), [ADR 0359](0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md)
(the requirement and decision fields and the decision's conversion table),
[ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md) and
[ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md) (a typed edit, judged by the change it makes:
E080), [ADR 0064](0064-a-declared-field-type-decides-one-wire-form.md) (one wire form of a value) and
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) (recorded facts). No SDK, protocol or
kernel change: the converter is a repository task in `xtask`, and every value it writes goes through the typed edit.

## Context

- SpecForge's own spec is written in the kinds of the old builtins, `@specforge/governance` (decision, constraint,
  failure_mode, invariant) and `@specforge/product` (feature, journey, deliverable, milestone, module, term, persona,
  channel, release). `@gsd/core` now declares the kinds that take their place: decision, requirement, risk,
  invariant, journey, persona, term and milestone.
- One environment cannot hold both sets: a kind two loaded extensions declare is E026 and the first registration
  wins (milestone, decision and deliverable before this work; persona, journey, term and invariant since). So the old
  project and the new one are two compiles in two environments, never one.
- "Every old governance and product kind has a home" was a prose figure that mapped kinds, not fields; nothing
  converted a value, and nothing counted what was left behind.

## Decisions

- **D1. One repository task, the compiler in and typed edits out.** `cargo run -p xtask --bin absorb -- --from <old>
  --to <new>` compiles the old project in-process with its own builtins, maps every authored value through one table,
  scaffolds a fresh target with `ops init` (`@specforge/checks`, `@code/core` and the `@gsd/core` component
  `GSD_CORE_WASM` names, plus each `--extension`), and writes the whole conversion as one typed-edit plan of creates
  (`edit::run`): forward references resolve inside it, and it writes all or nothing, judged by E080. Ids, titles and
  file paths are kept. It is a one-time migration, deleted with the old builtins once the root project has moved.
- **D2. The table is the measure.** `xtask/src/absorb/table.rs` holds one row per old (kind, field) as the old
  builtins' registry holds it, shared fields and other builtins' enhancements included, plus one obligation row per
  old kind that takes `verify`. Each row has one or more homes:
  - **To** a declared, authored field of a `@gsd/core` kind, through a named transform;
  - **Computed**: the new kind computes the value (a risk's `rpn`, a milestone's `state`); the converter compares
    the computed value with the old one after the write;
  - **Later**: the value waits for a kind not absorbed yet, named by what is missing, never by a date: a release, a
    package, a truth or a behavior;
  - **Refused**: ADR 0357's class, a field no tracked project writes and no new kind holds; a value of it stops the
    run, naming the entity and the field;
  - **Default**: the old field's implied default, which the new kind implies too (`channel.status active`); any other
    word stops the run.

  No row drops a value. A test counts the rows against the registry the old builtins load (141 field rows: To 89,
  Computed 4, Later 31, Refused 16, Default 1; 3 obligation rows), and a second checks every To and Computed home
  against the declared `@gsd/core` kinds (`gsd/core/tests/golden/kinds.json`), so a renamed field fails a test, not
  a conversion.
- **D3. The absorb ledger** is the converter's record of every old value not written as itself, with why: a Later
  value, a reference that leaves the run or reaches a kind not absorbed, a computed value that differs from the old
  one, a filled required field, a collapsed word, an unconverted metric, an unproven obligation, an entity the
  converter created. Format 1 is a JSON object (`format`, `from`, `entries`) whose entries are sorted by (kind, id,
  field, the value's canonical JSON, why), each value in the one wire form every output writes (ADR 0064); it holds
  no time and no absolute path, so two runs on one input write the same bytes.
- **D4. A reference that leaves the run is ledgered, never stubbed.** A reference to an entity this run does not
  convert is `out_of_selection`; to a kind with no home yet it is `target_later`, naming what it waits for. An inverse
  write (a deliverable's journeys, a persona's key features, a decision's successor) onto an entity not converted is
  ledgered the same way. No placeholder entity is ever created.
- **D5. Computed values are recomputed, then compared.** A risk's `severity`, `occurrence` and `detection` words take
  ADR 0358 D8's numbers, and its `rpn` and `post_rpn` are the new kind's; a feature's status becomes the
  requirement's tier and is compared with its computed `status`; a milestone's or deliverable's status is compared
  with the milestone's computed `state`; a superseded decision is compared with its computed `in_force`. A difference
  is a ledger entry with the new value.
- **D6. Proofs are the tests that state them.** An old invariant's `verify` obligation becomes a `proof` reference to
  the test id of each recorded `@rust/source` test item whose spec-link attribute states it: the attribute is read
  with `syn` (one reader, shared with check-links), and joined to the item that names the invariant at the same file
  and line. The run lists every cited test id, so a gate can run just those. An obligation no test item states is an
  `unproven` ledger entry naming the attributes that state it. A written proof is held once the target collects that
  test's facts; until then it is `code/W102`, as any proof of an uncollected test is. A source that loads
  `@rust/source` and holds no recorded item is refused, so proofs are never silently empty.
- **D7. A metric is the bound line the prove pass reads.** The old constraint's `metric` lines are read with the
  prove pass's expression reader (`parse_expression`), the one reader of that format. Exactly one line of the form
  `<name> <op> <number><unit>` becomes the requirement's `metric` record; its threshold keeps the number as written
  (`95.0` stays `95.0`), and its other lines join the requirement's text. A strict bound is written as `at_most` or
  `at_least` and ledgered as collapsed; no such line, several, or `!=` writes no metric and ledgers it unconverted.
- **D8. What Later waits for.** Release values wait for a release kind of `@gsd/core`; package values (module
  references, a module's own fields) for `@code/core` packages once modules move there; truth values (a feature's
  acceptance, a constraint's and failure mode's obligations, `enforced_by`, a formal expression) and behavior values
  (a constraint's `constrains`, a milestone's behaviors) for the truth and behavior kinds. The table names each, and a
  later change that lands one gives its rows homes.
- **D9. The root project keeps its old builtins until its spec is swapped**, and the swap chooses how it loads
  `@gsd/core`, among:
  - (a) vendor it as a committed builtin: a `BUILTINS` row, re-vendored with the others. Every check loads it with
    nothing to build; it reverses [gsd ADR 0001](../../gsd/adr/0001-one-cargo-workspace-built-on-demand.md) D4
    (the blob is built on demand, not committed), and the blob budget grows by its size.
  - (b) `specforge.json` names a built `.wasm` by path, and every check builds it first, as `GSD_CORE_WASM` does for
    the gate today: nothing committed, but every fresh clone builds the gsd workspace before its first check.
  - (c) a registry package pinned by its digest in `specforge.lock`: a released `@gsd/core`, installed as any
    extension is; it needs the gsd release pipeline first.

## Rejected

- A kernel kind-map migration (`specforge migrate --kind-map`): the kernel is domain-free and knows no kind's
  meaning; the map is one domain's, and it is code here so the measure is a test.
- `@gsd/import`: it reads `.planning/` Markdown, cannot compile the old kinds, and a `gsd/` crate may not name the
  repository's `spec/`.
- Rewriting the `.spec` text: it would restate the formatter, the reference spelling and the field order the typed
  edit already owns, and E080 would not judge it.
- Loading both stacks in one project: E026, and the first registration silently wins.
- One plan per file: the plan's cost is the plan, not the file count, and a partial target would be left behind when
  a later file failed.

## Consequences

- The conversion is a command with a test per row, and its ledger is what a reviewer reads instead of the old spec.
- A converted project checks with no error by construction: a required value the old spec lacks is filled and
  ledgered (an area, a tier, an origin, a reason, an occurrence or detection of 10), and a value nothing can hold
  stops the run before any write.
- The root project does not move here: its swap is its own change, with D9's choice.
- The engine's plan is superlinear in the number of creates (about a second for a hundred, minutes for thousands),
  so a whole-spec run is a deliberate, measured step.

## Would reopen it

- A kind landing for a Later home: its rows move to To, and the measure's counts with them.
- A second old project to convert: the table would then be read by more than one run, and its Later rows would need
  a plan of their own.

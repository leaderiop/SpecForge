# A field's checks may wait for its owner

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). **Amends**
[ADR 0336](0336-a-field-declares-the-checks-its-values-pass.md) D4 (the values phase comes first for ungated values
only; gated values are checked after the compute passes) and D5 (a gated value is no judged input), and
[ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md) D4 (the values phase reads owner state too, on the
merged snapshot), and [ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) D7 (the baseline runs
the ungated checks at the commit, D9). Protocol 2.0.0, unreleased: its additions go into the 2.0.0 row, no bump.
Amended by [ADR 0356](0356-an-extension-declares-the-context-a-role-reads.md): a gate on a `context` field is kept with no declared check; its record's staleness is its check.

## Context

- A field's checks (ADR 0336) run on every value. A plan's paths name files the plan will create, so a file check on
  them is red until the plan has run: `@gsd/core` therefore leaves 17 of its 18 authored path fields plain strings, and
  nothing reports a missing file once the plan is complete.
- Some paths are the opposite: a file a plan deletes, the file a broken window or a refactor proposal names. They
  must exist while their owner is open; once it is closed they are history.
- Owner state is ADR 0329 D4's: a kind's `complete_when`, else its nearest scope ancestor's. It is decided on the
  merged snapshot, where a computed value (gsd's `plan.status`) decides. The values phase runs before the compute
  passes, on the authored snapshot, where every gsd plan is pending.

## D1. `checks_while` is a property of the field

`FieldDescriptor.checks_while` is `complete` or `pending` (kept a string, so an unknown word costs the gate, not the
describe payload). The SDK writes it with `FieldBuilder::planned()` (`complete`) and `FieldBuilder::while_open()`
(`pending`); the wire words are `OwnerState`'s.

- It governs every check on the field, the ones `specforge.json` attaches included.
- The registry validates it against the kind's scope chain (D6), and the schema shows it
  (`SchemaField.checks_while`).

## D2. Planned and while-open values, and their statuses

- A **planned** field (`checks_while: complete`) is checked once its holder's owner is complete. While the owner is
  pending its value's row says `planned`: no checker is called, nothing goes red, and nothing counts toward I206 or
  I207.
- A **while-open** field (`checks_while: pending`) is checked only while its owner is pending. Once the owner is
  complete the row says `closed`, with no call.
- `CheckStatus` gains `Planned` and `Closed` (`planned`, `closed`); `inspect`, MCP, the hover and a collector's
  `check_outcomes` input carry them. `@specforge/checks` writes no outcome fact for either: they are no outcomes.

## D3. Owner state on the merged snapshot: a second values stage

- The values phase runs in two stages over one checker cache and one value budget per compile:
  1. **ungated** values, first, on the authored snapshot: their outcome classes are what judged digests read
     (ADR 0336 D4 now holds for them);
  2. **gated** values, after `Environment::checked` returns the merged snapshot (judged rules, compute, merge,
     registry checks and check passes): owner state is read there, so a computed status decides.
- Compute reads judged values and judged rules read stage-1 outcomes, so the order cannot be reversed. The cost is one
  more `__check_values` call per extension per compile, only when a gated value is checked; the cache answers it in a
  repeated run.

## D4. A gated value is no judged input

Judged rules run before compute, so a gated value could feed a digest only as a constant. Gated values are left out of
`CheckedValues`: no digest moves when an owner completes, and no gated value is ever `unknown` or I207. A rule that
judges the file reads it with `file()`. Amends ADR 0336 D5 ("the stamps").

## D5. The written form is read while the owner is pending

A held planned value of a path-form check (`CheckRegistry::path_root`) is still read lexically through
`Contained::lexical`: an unknown root is E095, an absolute path or a climb out is E094, a backslash is W175, placed as a
probe's refusal is. A link, case differences and existence wait for completion: only the disk is not asked.

## D6. W199 and I213

- **W199** (registry, setup): the gate is dropped and the checks run on every value, when `checks_while` is neither
  `complete` nor `pending`, the field declares no check, or neither its kind nor a kind along its `scoped_by` chain
  keeps a `complete_when` (a static walk). A refused attribute is dropped and the rest is kept (ADR 0329, "Rejected"):
  a check is never silently lost. A field that is not authored already loses its checks to W189; its gate goes with
  them.
- **I213** (info, not `machine`: the project's state decides it): once per compile when a value is `planned`, counted
  by declared path. `closed` rows are not counted: nothing waits.

## D7. gsd's path fields carry their owner mode

The kit declares a path field with a mode, `Planned`, `Present`, `WhileOpen` or `History`, replacing `Ty::Files`:

- **Planned:** `plan.files_modified`, `task.files`, `task.read_first`, `artifact.path`, `key_link.from`,
  `key_link.to`, `validation.config_file`, `validation_row.file`, `wave0_item.path`, and `@gsd/security`'s
  `threat.files`;
- **Present:** `plan.context_refs`, `context.canonical_refs[].paths`, `verify_gap.artifacts[].path`,
  `uat_gap.artifacts[].path`;
- **WhileOpen:** `plan.files_deleted`, `broken_window.file`, `refactor_proposal.target_file`;
- **History:** `deviation.files`, unchecked by design.

Three kinds gain `complete_when`: `phase` (`executed`, `needs_review`, `complete`: its plans ran, so validation's files
exist), `broken_window` (`waived`, `fixed`) and `refactor_proposal` (`accepted`, `declined`). Every phase-scoped kind
without its own declaration is therefore pending until its phase is executed; no reader of `pending` acts on that
today, and a future one inherits it.

## D8. E320's excuse comes from the typed planned paths

`gsd/E320` excuses a missing verify directory only while the task is pending, and only when a typed Planned path of
its plan's subtree equals or covers it. The comma and white-space split of `task.files` and the hand-made list of
fields go: the set is derived from the declaration, so no second list can drift.

## D9. The baseline runs the ungated checks at the commit

A verdict's digest at the baseline (ADR 0290 D7) reads the commit's checked values as it reads them now, so an
unchanged tree judges alike at both ends:

- the baseline runs stage 1 at the commit, before its judged rules, over the commit's files: its tree says what exists
  (fetched whenever the project has a check), and the bytes a probe reads are fetched from it on demand, through the
  one reader the baseline's compute passes use (ADR 0300);
- only the compile's effect journal answers a probe there (`cached`): nothing is performed at the commit, and a value
  with no fresh record is `unknown`, unjudged as it is now; the compile's checker cache answers what it already knows;
- stage 2 does not run at the commit: a gated value is no judged input (D4);
- the baseline cache's environment digest covers the checks `specforge.json` attaches; a cached baseline also keeps
  each journal record the commit's checks read, with the expiry it answered to, and is used (in memory, on disk, or
  held by a session) only while each still answers so, so the cache's key is the commit, the environment and those
  answers; and its format is 5.

## D10. Gaining an owner gate is a minor schema change

The schema diff that bumps the schema version treats `checks_while` as an attribute of the field: a field that gains
one checks fewer values, so a document valid before stays valid, and the change is minor; a field that loses its gate,
or changes `complete` to `pending` or back, checks values that went unchecked, and the change is major.

## D11. A compile's checker cache may change under a hook

The values phase keeps its checkers' answers in the host's checker cache under `.specforge/effects/checks/`
(ADR 0336). A hook that compiles the project may therefore leave that cache changed; it is the host's state, never an
extension's, and no hook writes anything else in the project but its own extension state.

## Rejected

- **A gate on the entity** (a per-entity flag): the meaning belongs to the field, which the registry can validate and
  the schema can show.
- **A check argument**: arguments belong to a checker (ADR 0336 D7), and a gate governs every check on the field.
- **Two booleans** (`planned`, `while_open`): they could contradict each other; one value cannot.
- **A constant `planned` digest tag**: it changes every gated entity's digest once, for nothing, and is the same
  constant as leaving the value out afterwards.
- **Computing owner state before the phase**: compute reads judged values, which read stage-1 outcomes.
- **Judging the baseline with no checked values**: every verdict over a checked entity digests differently at the
  commit, so an unchanged tree reports every truth stale there and no regression is ever seen.
- **Giving the baseline today's checked values**: a file deleted since the commit would be judged missing at the commit.

## Consequences

- A pending gsd plan's new file is `planned`: no E016, and `inspect` says so. The same plan, complete without the file,
  reports E016 on the item.
- An extension may be called twice per compile by the values phase (stage 2).
- An owner field written by a failing compute pass (E028) stays pending, and its planned values wait silently; E028 is
  already reported.

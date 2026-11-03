# `@code/core` holds test facts and their ids

**Status:** accepted (2026-11-02, build-plan R1, plan 02). Amends the "Linkage" section of
[ADR 0002](0002-test-runner-extensions.md) together with [ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md)
(D10): it says which extension judges a cited test fact, and how. Ships in extension protocol 1.15.0 (D4, D3's
`result`).

## Context

ADR 0327 lets a collector answer facts and a spec file cite one by a scheme reference (`"js:…#name"`). The kernel
names no fact kind, and a reference no fact holds is *unheld*: no edge and no core diagnostic, because whether it is
wrong is the kind's extension's to judge. R1's thread (`TASK-01 → … → js:taskboard/test/tasks.test.js#… (passed)`)
needs that extension: a neutral home for test facts that every runner pack (`@javascript/node-test` now, vitest and
`@rust/test` later) writes the same way, and that a requirement kind (`@gsd/core`'s `truth.proof`) can cite.

Four facts shape it:

- `bridge.wit` has no guest-to-guest calls, so packs share code by linking a library, not by calling `@code/core`.
- `node --test --test-reporter=junit` nests a `describe`d test in `<testsuite>` elements, and two tests named
  `adds` under different `describe`s in one file are legal.
- A requirement cited before its test is written (TDD) is not wrong yet. The kernel has no notion of "pending":
  `lifecycle_field` is read and ignored since ADR 0287, and `scoped_by` is the only link from an entity to its owner.
- A fresh clone has no fact store (ADR 0327 D5): a citation there names no fact and no store.

## Decisions

- **D1. `@code/core` is the neutral base of test facts.** A builtin (ADR 0328) at `extensions/code/core`, version
  1.0.0. It declares the kinds `test` and `test_run` (D3), the codes it reports (catalogued, so they print
  `code/E101`, ADR 0328 D2) and two check passes, `refs` (D5) and `proof` (D6). It declares no collector: a runner
  pack collects, and links the library of D7.
- **D2. A test's id names its package, its file and its suite path.** `js:<package>/<file>#<s1 > … > name>`:
  - `<package>` is the name of the package the pack assigns the test to (node-test: the nearest `package.json` with a
    `name`), a scoped `@scope/name` kept whole. An unscoped npm name cannot start with `@`, so the form is injective;
  - `<file>` is the test file's path under that package, with `/` separators;
  - the enclosing suites and the test's name are joined by `" > "` (vitest's separator), outermost first; a top-level
    test is `#<name>`.

  A `test_run`'s id is `run:<test id>`. A store is per key (ADR 0327 D3), so the run of another commit is in another
  store: R1 needs no `commit` field.
- **D3. The kinds.** Both are collector-written (`KindBuilder::collected`) and neither is scoped: a scheme ref names
  the fact whole.

  | Kind | Schemes | Fields | Result |
  |---|---|---|---|
  | `test` | `js` | `package`, `file` (the case's file, project-relative), `suite` (string list), `name`, `status` (enum `passed\|failed\|skipped`, the `TestStatus` names), `names` (string list) | `status` |
  | `test_run` | `run` | `test` (reference to `test`, edge `run_of`), `status` (the same enum), `duration_ms` (integer) | none |

  - **A kind's result.** A collector-written kind may name the field that holds its result (`result`, protocol
    1.15.0): the field of the fact a reference cites, so a reader of `truth.proof` reads the cited test's status with
    no inverse hop. The registry keeps it only on a collector-written kind, on a declared enum field whose values are
    `TestStatus` names; anything else is W021 and the attribute is dropped. `test` declares `result("status")`.
  - `test.status` is written by the collector, not computed. One call of the library's `facts()` writes it and the
    run's `status` from the same case, so the two cannot disagree. Two cases with one id merge: the worst status wins
    (failed, then skipped, then passed) and the durations add up.
  - `duration_ms` is an integer: a kind's `number` field is W021 (ADR 0305), and a fact's number is read as an
    integer (ADR 0327 D5).
  - `names` lists the entities a test says it proves. A later pack fills it from its runner's idiom (vitest's
    `meta.specforge`, `@rust/test`'s attribute); no R1 collector writes it. It is a string list, not a reference
    list: a fact's reference to an authored entity would need its own kernel rule, which comes with the first writer.
- **D4. Owner state (protocol 1.15.0).** A kind may declare `complete_when { field, values }`: its entities are
  complete while `field` holds one of `values`, and pending otherwise (unwritten included). The registry keeps it
  when `field` is an `enum` field the kind declares and every value is one of its values; anything else is W021 and
  the declaration is dropped, as a refused `scoped_by` is.
  - A pass entity carries `pending` (sent only when true). The entity's own kind's `complete_when` decides; if that
    kind declares none, the nearest `scoped_by` ancestor whose kind declares one decides; with no declaration in the
    chain, the entity is not pending.
  - It is decided on the snapshot the check passes read, computed values included (`plan.status` is computed).
  - The kernel stays domain-free: `@gsd/core` declares that a `plan` is complete on `status ∈ [complete]`, and
    `@code/core` reads only `pending`, never a GSD field.
- **D5. A cited fact reference is linked, E101, W102 or unchecked: exactly one.** The pass `refs` reads each item of
  `PassEntity.unheld` in `@code/core`'s schemes (`js`, `run`). An unheld reference carries the span it is written at
  (`UnheldReference.span`, protocol 1.15.0), so a finding points inside its quotes.
  1. Its holder is `pending`: nothing (U4: a red or missing test before the owner is complete is TDD).
  2. A served store (`PassInput.fact_stores`) holds facts of its scheme: **`code/E101`**, the store does not hold the
     test. It suggests the closest held id that shares its `<scheme>:<package>/<file>#` prefix.
  3. Otherwise: **`code/W102`**, no facts of its scheme were collected (`specforge collect`).

  A store's schemes are the schemes of the facts it holds, not the ones its kinds declare. "No `ts` facts were
  collected" is the true and actionable answer for a `ts:` reference beside a store of `js` facts, where E101 would
  claim a test is missing that no collector looked for. A fact nothing cites is never a finding, and there is no
  project-level "no facts yet" note: a project that cites nothing hears nothing.
- **D6. A failing proof and a dangling name.** The pass `proof` reports:
  - **`code/W401`** on each entity outside `@code/core`'s kinds that is not pending and has an edge to a `test` whose
    `status` is `failed`;
  - **`code/I402`** on each `test` whose `names` holds an item that is no entity id of the snapshot.
- **D7. The JUnit reader is a linked library.** `extensions/code/junit` (package `specforge-code-junit`, an extension
  library, ADR 0328 D3) holds the JUnit reader (nested suites; `<failure>` and `<error>` fail, `<skipped>` skips, a
  `todo` included), `relative()` (it strips a `\\?\` prefix, folds `\` to `/`, compares a drive letter
  case-insensitively and tries the other spelling of macOS's `/private` alias), `TestId`, and `facts()`, the one
  function that mints `test` and `test_run` facts. A runner pack links it. gsd-junit stays a copy, since gsd may not
  depend outside `gsd/` (R-9).

## Rejected

- **Reading the owner's fields by name in `@code/core`.** The base would know GSD's vocabulary. `complete_when` is
  one domain-free declaration, and `pending` one derived flag.
- **E030 for a malformed `complete_when` or `result`.** E030 is a declaration the host cannot use at all (its
  identity, its codes); a refused kind attribute is W021 and is dropped, as a refused `scoped_by` is, and the kind
  still loads.
- **W102 per project, or per store key.** A project-level note fires on every project that loads `@code/core`, and a
  per-key reading turns a `rust:` reference into E101 because node-test ran.
- **A computed `last_status` on `test`.** A computed field on a collected kind is a second writer of a fact; the
  collector already knows the status.
- **The result on `test_run` alone.** Every reader of a proof would then look for the run whose `test` names the
  cited fact, an inverse lookup.
- **An id without the suite path.** Two nested tests named `adds` in one file would share an id.
- **`@code/core` calling into the packs, or the packs calling `@code/core`.** The wire has no guest-to-guest call.

## Consequences

- A spec that cites `js:` facts with no store hears `code/W102` once per cited reference, and nothing while the
  citing entity's owner is pending.
- Every JUnit pack mints identical ids from one function; a pack chooses only the package and the file base.
- `@gsd/core` (plan 04) declares `complete_when` on `plan` and `task` and cites tests through `truth.proof`.
- The other kinds the beyond-parity design names for `@code/core` (coverage, findings, dependencies), `test.kind`,
  `flaky`, run history and `names` as an edge come with later rounds.

## What would reopen it

- A runner whose tests have no stable name (generated, parameterised beyond a name): the id grammar would need a
  parameter part.
- A second notion of "done" an owner cannot state as one enum field.

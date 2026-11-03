# A collector's facts are read-only entities, recorded by digest

**Status:** accepted (2026-11-02, R1 of the Beyond Parity build plan, K23). Amends the "Linkage" section of
[ADR 0002](0002-test-runner-extensions.md) (D10). Ships in extension protocol 1.15.0.

## Context

A collector maps a runner's report to results on authored entities (`entity_results`, `unlinked`) and may answer an
edit plan (ADR 0289, 0303). It cannot say what it *observed*: the tests a runner found, with their ids and files, are
not entities, so a spec cannot cite one and no pass can judge a citation.

The kernel explore (2026-11-02) found:

- **K1.** `collect` dispatches each collector, merges its results into `specforge-report.json` and keeps its plan.
  `CollectOutput` has no list of observed things.
- **K2.** The graph is built at `Environment::build_sources` from parsed `SpecFile`s. Every command, query, export
  and pass reads the graph; nothing builds a `SpecFile` outside the parser. A kind's `writer` is read only as
  `human`, and `FieldWriter` has no value for "a collector gives the whole entity".
- **K3.** A quoted reference holding `:` (`"js:pkg/file.test.js#name"`) already parses as one reference in a
  `reference_list` and is looked up whole: no grammar change is needed.
- **K4.** Such a reference answers E003 when nothing holds its id. The store a collect writes is git-ignored, so on
  a fresh clone or in CI every citation would be an error.
- **K5.** An LSP rename plans an edit of every declaration; a fact has no text, so the plan is unplaced and the
  rename is answered `ContentModified` and retried forever.
- **K6.** `.specforge/` is git-ignored; `specforge-harness` is committed with `merge=union`, and a line whose key
  holds a namespace (`ns:rest`) survives every rewrite. A host write takes the write lock and rewrites the ledger in
  the same change.
- **K7.** A new wire field is a protocol minor (1.15.0).
- **K8.** E014 caps ids at 60 characters; a test's name is longer.

## Decision

- **D1. A fact is an entity a collector writes.** Its kind declares `writer: "collector"` and `schemes`, the
  prefixes of its ids (`js`, `rust`). A fact is never written in a `.spec` file (E115) and never edited (D7). The
  kernel names no kind: `@code/core` declares `test` and `test_run`.
- **D2. The store.** `.specforge/facts/<key>/<collector>.json` (git-ignored), one file per collector and key:

  ```json
  {"store": 1, "collector": "node-test", "extension": "@javascript/node-test", "key": "<key>",
   "facts": [{"kind": "test", "id": "js:taskboard/test/tasks.test.js#a task is added with the next id",
              "title": "a task is added with the next id", "fields": {"file": "taskboard/test/tasks.test.js"}}]}
  ```

  The file is written canonically: facts sorted by `(kind, id)`, fields in key order, two-space JSON and a final
  newline. Equal facts give equal bytes, and so an equal digest.
- **D3. The key** is asked through the History port, with no new git call:
  - HEAD's full sha when `status` shows nothing;
  - `<sha>-dirty` when `status` lists a tracked change, a conflict or an untracked file;
  - `worktree` when the root is in no repository, HEAD is unborn, or git can't be asked.

  A key other than a clean sha is I209 in the collect outcome, naming why. A dirty collect overwrites the previous
  dirty one of that sha. The key is provenance: a reader compares it with HEAD to call a fact fresh.
- **D4. The ledger.**
  - The line is `<collector> facts:<key> <digest>`, with `digest = harness::digest(collector, "facts:<key>", file
    bytes)`.
  - A collect holds **one** line per collector. It replaces any `facts:` line of that collector, so the ledger grows
    with collectors, not with commits. A union merge can leave two; the next collect collapses them.
  - A rewrite drops a `facts:` line whose collector no loaded extension declares.
  - The store file, the removal of the collector's files under other keys, and the line are one planned change
    under the write lock (ADR 0287, 0289).
  - Git keeps the history of the line, which is the audit trail.
- **D5. Serving.**
  - `Environment::build_sources` adds one synthetic `SpecFile` per store file that a ledger line records (its
    collector and key, its digest equal).
  - The `SpecFile`'s path is the store file's path from the root. Each fact is an `Entity` with its kind, its id,
    its title and its fields. A field value is read from JSON by the field's form: a string, a number, a bool, an
    array of strings as a list, a quoted reference.
  - A store the ledger doesn't record is W191, and one that doesn't read is W192. Neither is served.
  - When no loaded kind is collector-written, `.specforge/facts/` and the ledger are not read for facts: the build is
    byte-identical.
  - A fresh clone has no store, so it serves nothing, and its fact references are unheld (D6).
- **D6. Resolution.**
  - `GraphInputs.fact_schemes` maps each scheme to the collector-written kinds that declare it.
  - A reference finds no entity, its id starts with `<scheme>:`, the scheme is in `fact_schemes`, and the field's
    target is none or admits one of the scheme's kinds: that is `Resolution::Unheld`, with no edge and no
    diagnostic.
  - `Mistyped` (E022) and `Unavailable` (I004, the target kind not loaded) are decided first.
  - The node keeps its unheld references, and a pass is sent them (`PassEntity.unheld`), with the stores the session
    served (`PassInput.fact_stores`). Whether an unheld reference is wrong is the kind's extension's to judge.
- **D7. Refusals.**
  - A collected entity is created, set, unset, deleted and renamed only by `collect`. Every caller is refused, a
    person in the editor too, with the code `collected`.
  - A rename of an authored entity that a fact names is refused too: the fact has no text to rewrite.
  - The LSP answers its rename with that reason (`invalid_params`), not a `ContentModified` retried forever.
- **D8.** A collected entity's id is exempt from E014. Its length is its tool's (a test's name).
- **D9.** The session records the ledger, the store listing and each store file as inputs. A change re-reads the
  facts and applies them as source changes, and the LSP's watchers follow the recorded inputs.
- **D10. Amendment to ADR 0002 "Linkage".** A spec file may cite a test fact by a scheme reference
  (`truth.proof ["js:…#name"]`). It does not rot silently: `@code/core` reports a reference no fact holds (E101, or
  W102 with no store) and I402. "Spec files carry no test paths" now reads "carry no paths a tool can't check".

## Consequences

- A fresh clone serves no facts. Its references are unheld, and `@code/core` reports them, not the kernel.
- Switching branches hides a store the branch's ledger doesn't record (W191) until `collect` runs.
- The ledger grows by one line per collector.
- The digest makes every collect a ledger change, unless the facts are equal.
- Facts are graph nodes: every command, query, export, pass and `trace` reads them with no new path.

## Rejected

- **A side table of facts with its own query API.** Every consumer would have to learn it; as entities, facts are
  read by what already reads the graph.
- **Serving any store and only warning.** An uncommitted, unrecorded store (another branch's, or one edited by hand)
  would then decide `@code/core`'s E101. The committed ledger decides what is served.
- **A ledger line per key.** It grows with every commit and needs its own gc; git already keeps the line's history.
- **A digest of the tree as the key, or refusing a dirty collect.** The first needs a walk of the tree; the second
  blocks the edit-and-test loop.
- **A per-collector declaration of the kinds it writes.** The kind's `schemes` and E116 guard the ids; it can come
  with a second pack if a clash shows.

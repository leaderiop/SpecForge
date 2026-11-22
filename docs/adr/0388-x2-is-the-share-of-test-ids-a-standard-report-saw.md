# X2 is the share of items with a test id that a standard report saw

**Status:** accepted (2026-11-22, round r6-breadth, Phase 10). **Amends**
[ADR 0374](0374-the-root-spec-runs-on-gsd-core-and-its-tests-name-requirements.md) D5 (the dogfood check also prints
X2).

## Context

- Build-plan R6 exits on "X2 = 100% on taskboard and on SpecForge", and owner decision D3 of the round says what X2
  counts: the items that have a test id, and of those the ones a standard test report saw. Nothing defined the number
  or computed it, so it could not be driven anywhere.
- Guests' tests run under `cargo test` and write no JUnit until Phase 14 moves them under nextest, so on SpecForge the
  value is far from 100% today, and it need not be more: this round defines and prints it, Phase 14 drives it up.
- JavaScript items carry no `test_id`: `@javascript/source` mints every item with `test_id: None`
  (`extensions/javascript/source/src/items.rs`), and only a Rust test function's item names its test's id. Taskboard
  is a JavaScript project, so its X2 is `0/0` by construction whatever its JUnit holds (research finding F3).
- `@code/core` holds the code facts and their ids ([ADR 0329](0329-code-core-holds-test-facts-and-their-ids.md),
  [ADR 0347](0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md)); a library with no wasm is an rlib a
  builtin links ([ADR 0328](0328-a-builtin-is-embedded-and-a-non-specforge-builtin-catalogues-its-codes.md) D3).

### The readers this round adds

Phase 10 also gives `@code/core` five report readers. Their decisions are recorded here, not in an ADR of their own,
so plans 10-02 to 10-07 implement a written design:

- **Five kinds**, collector-written like the four before them: `coverage`, `static_finding`, `dependency`,
  `vulnerability` and `api_item`. The SARIF kind is `static_finding`, not `finding` (F1): `@gsd/core` declares a
  research `finding`, and a project loading both would get E026, the first registration winning
  ([ADR 0369](0369-the-builtin-vocabulary-maps-a-kind-keyword-to-every-builtin-that-declares-it.md); the root
  project loads both). `@code/core` goes to 1.2.0.
- **One rlib per format** under `extensions/code/`, named `specforge-code-<format>` (`lcov`, `sarif`, `cyclonedx`,
  `osv`, `api`). Each depends on `specforge-code-facts`, never the reverse, so a reader edit re-vendors only the packs
  that link it (F2). The neutral models (`CoverageFile`, `SarifResult`, `Component`, `OsvRecord`, `ApiEntry`) and the
  minters live in `specforge-code-facts`, beside `test_facts` and `item_facts`.
- **Ids and schemes.** `cov:<item id>`, and `cov:@<file>` for the lines and functions no item holds;
  `sarif:<tool>/<rule>#<file>:<line>`; `dep:<package id>/<purl>`; `vuln:<package id>/<osv id>`; `api:rust:...` and
  `api:js:...`. Report text that enters an id is percent-encoded, so a rule named `x#y` cannot forge a delimiter.
  Every minter merges or suffixes a repeated id (`~2`, `~3`, in document order): the host refuses a collect that
  answers one id twice (E116), so one repeat in a real report would drop all of it (F4).
- **SARIF level as the file states it**: the result's `level`, else its rule's `defaultConfiguration.level`, else
  `unknown`. SARIF's own default `warning` is not applied, so `unknown` means "stated nowhere" (F5).
- **Severity of a vulnerability**: the record's `database_specific.severity` (GHSA's `MODERATE` read as medium), else
  the highest CVSS v3 base score, computed by the v3.1 formula with the integer Roundup of its Appendix A (a score of
  0.0 reads low, the vocabulary having no `none`), else `unknown`. A record with only a v4 or a v2 vector is
  `unknown`; nothing guesses.
- **What is the collector's error**, so nothing is minted: a report path outside the project, a malformed report, an
  unsupported version (named in the error), a BOM whose root component matches no collected package, and an
  osv-scanner lockfile no collected package holds. A vulnerability affects the packages at the lockfile's directory,
  else those under it.
- **API snapshots** are `cargo public-api` text for Rust and api-extractor `.api.json` for JavaScript. An entry maps by
  its qualified path against item ids; a re-exported path that differs from the definition's path maps to no item.
- **Unmappable entries are kept.** An entry that cannot be mapped is still minted, with `unknown` or no reference, and
  the new check pass `reports`, sent only the five kinds
  ([ADR 0353](0353-a-pass-reads-the-facts-it-declares.md)), raises one warning per class: `code/W104` an entry names
  no item (a finding: no item and no package), `code/W105` a finding's rule or level is unknown, `code/W106` a
  dependency has no purl, `code/W107` a vulnerability's severity is unknown.

## D1. The definition

Over one project's collected facts:

- `m` is the number of `item` facts that carry a `test_id`;
- `n` is the number of those whose `test_id` is the id of a `test` fact;
- X2 is `n/m`, with the percent rounded down to one decimal, so `100.0%` is printed only when no item is missing:
  `X2 <project>: n/m (p%)`. When `m = 0` it prints
  `X2 <project>: 0/0 (no item has a test_id)` and no percent.
- A test of any status counts, a skipped one too: whether it passed is W401's to say, not X2's.
- The items it misses are the item ids with a `test_id` and no `test` fact, sorted, each on its own line under the
  ratio, indented two spaces.

The reason: X2 measures whether a standard report sees the tests the code declares, not whether they pass. Reading it
from the collected facts, not from a test list of its own, keeps it the same number every consumer would compute.

## D2. A pure function the gate prints

`gate/src/x2.rs` holds the computation, a pure function over the items' test ids and the test ids, tested on literal
input like `judge::requirement_verdict`. The `dogfood` check calls it and writes its lines to
`<target>/gate/dogfood.log`. It is print only: no floor fails the check until Phase 14 asserts 100% with a negative
control. A failed collect or export of an X2 project is an error of the check, since then no number was computed; the
number itself never is. So is an export of another shape (no `nodes` array, a node with no `id` or `kind`), which would
otherwise read as `0/0`; and when the requirement verdict already fails the check, an X2 error is still logged. `doctor` may call the same function later (Phase 11 owns `doctor`).

The reason: guests are not under nextest yet, so any floor now would be either meaningless or red; the gate already
collects the root for `dogfood`, so X2 costs no second collect of it.

## D3. The projects

Three projects print, in this order:

- **SpecForge**: the collected dogfood tree `dogfood` already builds.
- **taskboard**: a scratch copy of `gsd/parity/fixtures/taskboard` collected by `javascript-source` and by `node-test`
  over its recorded node-test JUnit (`gsd/parity/goldens/taskboard/junit.xml`, `--no-run`). It prints `0/0` (F3).
- **ledger**: a scratch copy of `gsd/parity/fixtures/ledger` collected by `rust-cargo`, `rust-source` and `rust-test`
  over its recorded nextest report (`gsd/parity/goldens/ledger/rust-test.xml`): a non-zero denominator, so the ratio
  is shown on a project where it means something.

The scratch copies live under `<target>/gate/x2/<project>`, emptied first; the tracked fixtures are never written.

The reason: D3 of the round names taskboard and SpecForge. Taskboard's denominator is empty by construction, so ledger
is added to show the computation on real test ids; a further fixture is one more row.

## Rejected

- **Computing X2 in `doctor` now.** Phase 11 owns `doctor`; the gate already collects what X2 reads.
- **A failing floor now.** Guests run under `cargo test` until Phase 14; a floor would be red or set to the current
  value and mean nothing.
- **Counting JavaScript test files instead of test ids.** It would give taskboard a number, but a different number from
  a different definition. A later round may change the definition for JavaScript; this one records it as it is.
- **A second ADR for the readers.** Their decisions are small and belong to the same phase; they are this ADR's
  context.

## Consequences

- Phase 14 moves the guests' tests under nextest and asserts X2 at 100% on SpecForge and ledger, with a negative
  control.
- Phase 11's packs mint the five kinds with the ids above, and run the tools whose reports the readers take.
- Plans 10-03 to 10-07 declare `code/W104` to `code/W107` in `@code/core`, each with the kind it judges.
- Taskboard reads `0/0` until JavaScript items carry a test id or the definition changes.
- Once `@code/core` 1.2.0 is loaded, `coverage`, `api_item`, `static_finding`, `dependency` and `vulnerability` are
  kind names, and a project entity with one of them is error E013. The frozen `fixtures/old-spec` has `term coverage`,
  so the migrate test that loads it expects exactly that one E013 (ADR 0374 D8 forbids editing the corpus). The owner
  accepted the names on 2026-11-22; renaming them would re-vendor every blob that links `code-facts`.

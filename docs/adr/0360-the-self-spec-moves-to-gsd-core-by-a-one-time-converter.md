# The self-spec moves to @gsd/core by a one-time converter

**Status:** accepted (2026-11-15). D2, D4, D5, D6, D7, D14 and D16 amended by
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) (its D8): a value whose home leaves `@gsd/core`
is ledgered `dropped`, no home writes a reference, no kept kind holds a proof, and the sample runs no test until the
swap. D1, D2, D3, D4 and D8 amended by
[ADR 0372](0372-the-converter-gives-every-old-kind-one-fate-and-writes-gsds-ids.md): ids are GSD's with a map in the
ledger, the homes are To, Computed or Dropped, the ledger is format 2 with one outcome per entry, a phase's references
are written through the map and one to an entity the run does not convert is dropped, and nothing waits. Builds on
[ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md)
(the product kinds and their carry-over table), [ADR 0358](0358-a-risk-is-rated-and-an-invariant-is-judged.md) (risk,
invariant and the word scale), [ADR 0359](0359-requirement-and-decision-fields-absorb-feature-constraint-and-decision.md)
(the requirement and decision fields and the decision's conversion table),
[ADR 0151](0151-a-spec-file-is-changed-by-a-typed-edit.md) and
[ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md) (a typed edit, judged by the change it makes:
E080), [ADR 0064](0064-a-declared-field-type-decides-one-wire-form.md) (one wire form of a value) and
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) (recorded facts). The converter
changes no SDK, protocol or kernel code: it is a repository task in `xtask`, and every value it writes goes through
the typed edit. The sample and its gate (D10 to D17) **amend**
[ADR 0348](0348-a-collector-declares-a-config-file-and-the-rust-packs.md): a collector's `process` scope declares the
arguments a caller's filter adds, a declared variable name may be a prefix, and `@rust/test` takes both; D18 amends
[ADR 0346](0346-a-fact-says-where-it-is-written-and-what-it-names.md): W061 reads no fact's reference; D14 amends
[ADR 0068](0068-a-test-binary-records-collect-scores.md) D5: a caller chooses where a test binary's report goes with
`SPECFORGE_TEST_REPORT_DIR`. Protocol 2.2.0, no bump. D14 to D17 carried out by
[ADR 0374](0374-the-root-spec-runs-on-gsd-core-and-its-tests-name-requirements.md) D7: the sample retired before
the swap.

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
  file paths are kept, and so are the project's name, version and spec root: init scaffolds into the source's spec
  root (none: the sources sit at the project's root), so every kept file path lies inside it. Whatever else the
  source's `specforge.json` or `spec` block holds that the target's do not is ledgered (D3): an old extension the
  target does not load, a setting, the block's own version. `--to` is absent or holds no project; a stop or a refused
  plan leaves it as it was, byte for byte, and a symbolic link where absorb writes is refused, before any write:
  `--to`, a scaffold path, a file init appends to, and the ledger, at `<to>/absorb-ledger.json` or `--ledger`'s path
  alike, all through one test. It is a one-time migration, deleted with the old builtins once the root project has
  moved.
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
  one, a filled required field, a collapsed word or proved obligation, an unconverted metric, an unproven
  obligation, an entity the converter created, a project-block value the target does not hold (its kind
  `specforge.json` or `spec`). Format 1 is a JSON object (`format`, `from`, `entries`) whose entries are sorted by
  (kind, id, field, the value's canonical JSON, why), each value in the one wire form every output writes (ADR
  0064); it holds no time and no absolute path, so two runs on one input write the same bytes.
- **D4. A reference that leaves the run is ledgered, never stubbed.** A reference to an entity this run does not
  convert is `out_of_selection`; to a kind with no home yet it is `target_later`, naming what it waits for. An
  inverse write (a deliverable's journeys, a persona's key features, a decision's successor) onto an entity not
  converted is ledgered the same way. The other direction too: an inverse write from an entity the run does not
  convert, onto one it does (a `superseded_by` onto the selected successor, a left-out deliverable's journeys), is a
  value the converted entity loses, ledgered on it under the new field it would have been written in, one row per
  item, `out_of_selection` naming the entity left out. Either way, a required field the selection empties stops the
  run before any write, naming the selected entity, the field and the ids to add to `--only`, and the ledger is
  written; an optional one converts empty. No placeholder entity is ever created. A scheme reference in a list of
  texts (a feature's or a failure mode's `refs`, `gh.issue:42`) is ledgered `unconverted`: written bare it names a
  `ref` ([ADR 0093](0093-a-fields-declared-type-decides-what-its-value-references.md)), which no typed edit creates,
  so it would be E003 in the target.
- **D5. Computed values are recomputed, then compared.** A risk's `severity`, `occurrence` and `detection` words take
  ADR 0358 D8's numbers, and its `rpn` and `post_rpn` are the new kind's; a feature's status becomes the
  requirement's tier and is compared with its computed `status`; a milestone's or deliverable's status is compared
  with the milestone's computed `state`; a superseded decision is compared with its computed `in_force`. A difference
  is a ledger entry with the new value.
- **D6. Proofs are the tests that state them.** An old invariant's `verify` obligation becomes a `proof` reference to
  the test id of each recorded `@rust/source` test item whose spec-link attribute states it: the attribute is read
  with `syn` (one reader, shared with check-links), and joined to the item that names the invariant at the same file
  and line. The run lists every cited test id, so a gate can run just those. A proof keeps only the test ids, so each
  proved obligation's kind and text are a `collapsed` ledger entry into `invariant.proof`, naming its tests: two
  obligations one test proves stay two statements. An obligation no test item states is an `unproven` ledger entry
  naming the attributes that state it. A written proof is held once the target collects that test's facts; until then
  it is `code/W102`, as any proof of an uncollected test is. A source that loads `@rust/source` and holds no recorded
  item is refused, so proofs are never silently empty.
- **D7. A metric is the bound line the prove pass reads.** The old constraint's `metric` lines are read with the
  prove pass's expression reader (`parse_expression`), the one reader of that format. Exactly one line of the form
  `<name> <op> <number><unit>` becomes the requirement's `metric` record; its threshold keeps the number as written
  (`95.0` stays `95.0`), and its other lines join the requirement's text. A strict bound is written as `at_most` or
  `at_least` and ledgered as collapsed; no such line, several, or `!=` writes no metric and ledgers it unconverted.
- **D8. What Later waits for.** Release values wait for a release kind of `@gsd/core`; package values (module
  references, a module's own fields) for `@code/core` packages once modules move there; truth values (a feature's
  acceptance and tests, a constraint's and failure mode's obligations, `enforced_by`, a formal expression) and
  behavior values (a constraint's `constrains`, a milestone's behaviors) for the truth and behavior kinds. The table
  names each, and a later change that lands one gives its rows homes.
- **D9. The root project keeps its old builtins until its spec is swapped**, and the swap chooses how it loads
  `@gsd/core`, among:
  - (a) vendor it as a committed builtin: a `BUILTINS` row, re-vendored with the others. Every check loads it with
    nothing to build; it reverses [gsd ADR 0001](../../gsd/adr/0001-one-cargo-workspace-built-on-demand.md) D4
    (the blob is built on demand, not committed), and the blob budget grows by its size.
  - (b) `specforge.json` names a built `.wasm` by path, and every check builds it first, as `GSD_CORE_WASM` does for
    the gate today: nothing committed, but every fresh clone builds the gsd workspace before its first check.
  - (c) a registry package pinned by its digest in `specforge.lock`: a released `@gsd/core`, installed as any
    extension is; it needs the gsd release pipeline first.

## The sample and its gate

A conversion that is only ever run by hand drifts from the spec it converts. A sample of the self-spec is therefore
converted on every change that can move it, by a check of `cargo gate`, and must check clean: no error, no warning.

- **D10. A sample is a rule in code, computed from the old graph.** `--only sample:<name>` names one of
  `select::SAMPLES` (a second value form of `--only`, beside `ids:<file>`, yielding the same scope), and `--selection
  <file>` writes what it selected, by old kind, sorted. The `wasm-host` rule: every entity of
  `spec/features/wasm.spec` and `spec/invariants/wasm.spec`; every failure mode, constraint, decision and journey one
  of whose edges ends at one of them or at a behavior of `spec/behaviors/wasm-*.spec` (one step; those behaviors stay
  out); every persona of `spec/product/personas.spec`; then every term whose title or alias the prose of a sampled
  non-term entity names (the fields the old registry declares `string` or `string_list`), as a whole word or phrase,
  in any case, Unicode-aware, its words apart by any whitespace, so a phrase a line of prose wraps still names it.
  Each text is matched alone: a phrase split between two fields, or two entities, names nothing. A term's id is
  never matched, and a term never draws a term. It selects 108 old entities: constraint 9, decision 6,
  failure mode 12, feature 12, invariant 16, journey 8, persona 6 and term 39 (a phrase wrapped across a line of a
  constraint's prose names both glossaries' "traceability chain").
  - Rejected: a committed id list, which is not computed from the graph and drifts; a selector file format, a second
    reader for one sample; gsd's own glossary matcher, which xtask cannot link across workspaces.
- **D11. The nine WASM-host constraints state their bound on a line the prove pass reads**, the first line of each
  `metric """` (`traceability_false_negatives <= 0`, `cached_extension_load < 50ms`, `extension_memory <= 512MB`, …),
  so D7 converts each to a `metric` record: none is `unconverted`, and the three strict ones (`<`) are `collapsed`.
- **D12. The filter is a declared argument fragment the caller fills.** A collector's `process` capability may
  declare `filter`, the arguments appended after its argv when, and only when, a caller passes `specforge collect
  --filter <TEXT>` (MCP `specforge.collect`'s `filter`); `{filter}` in exactly one of them is the caller's text, one
  argument, never read by a shell. The approval covers the declared command and fragment, so one approval covers any
  text. A filter needs exactly one collector and a run (not `--no-run` or `--report`); a collector declaring none
  refuses one; `filter_problem` refuses an empty text, one over 16 KiB, one starting with `-`, or one holding a
  control character. `@rust/test` declares `["-E", "{filter}"]`: the text is a nextest filterset.
- **D13. A declared variable name may end in `*`**, a namespace prefix (a letter first, `[A-Za-z0-9_]` only, at least
  4 characters, ending in `_`), inherit-only: a guest never sets a variable through one. `@rust/test` declares
  `CARGO_*`, `RUSTUP_*`, `RUSTC`, `RUSTC_*` and `RUSTFLAGS`, so its approval names the environment its build reads.
  - A collector's command is a program the host runs, not a guest: it keeps the caller's environment
    ([ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md)'s empty environment holds for a program
    a guest runs), so it builds where the caller builds. `@rust/test`'s command runs on this host path; its approval
    line says so (`runs in the caller's environment less credentials (reads CARGO_*,…)`), where a command a guest may
    run reads `inherits or sets …, never a credential`. The prefix expansion is the protocol's mechanism for a
    guest-run program, kept for the declarants that path will have and tested end to end through the guest's
    `process` port.
  - **One credential filter, both paths** (`specforge_process::passed`, for every program a request or an extension
    asks for; the host's own `extension build` keeps the secret deny-list alone, so it fetches as the user does): a
    secret's name (`_TOKEN`, `_AUTHTOKEN`, `_AUTH`, `_SECRET`, `PASSWORD` (`PGPASSWORD`), `_PASSWD`, `_PWD`
    (`MYSQL_PWD`, never `PWD` or `OLDPWD`), `_PAT` (`GH_PAT`), `_KEY`, `_CREDENTIALS`, `AWS_`, any case on every system)
    and a cargo credential provider (`CARGO_REGISTRY_CREDENTIAL_PROVIDER`,
    `CARGO_REGISTRIES_<NAME>_CREDENTIAL_PROVIDER`, `CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS`, `CARGO_CREDENTIAL_*`)
    are withheld, even when declared by their exact name; a URL in an inherited value passes without its user and
    password (a proxy, `CARGO_REGISTRIES_<NAME>_INDEX`, `RUSTUP_DIST_SERVER`, `NPM_CONFIG_REGISTRY`, a database URL),
    a user alone removed under an HTTP scheme, where a token is written as the user, and kept as `ssh://git@host`'s
    account. The user information runs to the authority's last `@`: once a `user:` starts it and no port follows the
    `:`, an unencoded `#`, `?`, `/` or `@` in the password does not end it (`postgres://user:pa#ss@db/x`). A proxy
    variable (ending `_PROXY`, never `NO_PROXY`) written without a scheme (`user:pass@proxy:8080`) is cleaned as curl
    reads it. A proxy without a user passes whole, so a command still fetches through it. A value that is not Unicode
    and may hold a URL's user is withheld, as it cannot be cleaned
    ([ADR 0288](0288-an-extensions-settings-are-typed-and-set-in-specforge-json.md): secrets never reach a guest).
  - Rejected: withholding a proxy that carries a password, which would send the command's fetches elsewhere (around
    the proxy, or nowhere) with no word; removing its password keeps the route, and a proxy that needs it refuses the
    request in the open. Rejected: running a collector's command in its declared environment only, which would
    rebuild where the caller's build lives and break a runner that reads a variable nobody thought to declare.
- **D14. The check `self-spec-sample`.** It builds the `@gsd/core` wasm, `specforge`, `absorb` and the workspace's
  tests, copies the tracked tree twice under `<target>/gate/self-spec-sample/` (`src/`: all of it; `dst/`: all but
  `spec/**` and `specforge.json`), collects `src` with `rust-cargo` and `rust-source`, absorbs the sample into `dst`
  (`--extension @rust/cargo --extension @rust/source --extension @rust/test`), collects `dst` with the same two, runs
  exactly the cited tests with `collect --runner rust-test --filter <filterset>`, then checks `dst` with
  `--effects cached`. The filterset groups the cited tests by binary (`binary_id(=b) & (test(=x) | test(=y))`), and
  one over the 16 KiB a collector's filter may hold is refused as the gate's own; when absorb cites no test, the run
  is skipped and the bound is broken by that line. The log's head counts the `collect` and `check` calls as they are
  made, and the cited tests that did not run. A passed run removes both copies; a failed one keeps them and names
  them after its error. Every `specforge` and `absorb` call gets `HOME` under the run directory, `CARGO_HOME` and
  `RUSTUP_HOME` kept, the gate's `CARGO_TARGET_DIR`, `--yes` against `SPECFORGE_CONSENT_FILE` in a `0700` directory
  emptied first, and no cargo credential variable, so the gate never writes the user's consents and never reuses a
  stale one. Each call also gets `GIT_CEILING_DIRECTORIES` at the run directory, and none of the variables that name
  another repository (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY`,
  `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_INDEX_FILE`): both copies sit in no repository wherever the target
  directory is, so a CI target under the checkout finds what a target outside every repository finds (`I204`, no
  history), and the goldens hold in both. The cited tests' own reports go to the run directory, never into the `<target>/specforge` the `tests`
  check fills, where a report of another nextest run would prune that check's: the gate passes no
  `SPECFORGE_REPORT` and sets `SPECFORGE_TEST_REPORT_DIR`, which the test support reads when no collect run names a
  report (amending ADR 0068 D5). `SPECFORGE_REPORT` is collect's own: a collector command told its report path by
  `{report}` (nextest's JUnit, through its config) runs with it removed from its environment, a value the caller
  exported included, where each test binary would have made a directory of the JUnit file's path; a command told it
  no other way gets it set over the caller's.
- **D15. The target run reuses the gate's test build: 0 units compiled.** The copies keep each file's modification
  time and permissions, the run shares the gate's target directory and cargo environment, and a path crate is keyed
  by its path from the workspace root, so cargo finds every unit fresh. The CLI's build script watches the commit's
  `.git` files by absolute path: a watched path under the package is kept relative, and a copy without `.git` would
  rerun it. The run declares the copy (`SPECFORGE_CHECKOUT_COPY_OF`, naming the checkout), so the test support's
  guard against a stale shared target directory passes, and the tests read the checkout they were built under. The
  guard honours the declaration only for a copy: the named checkout has its `.git` and its workspace manifest, and
  the running workspace is another directory with no `.git`; another checkout naming this one stays stale. So
  they would write there too: no bless switch reaches them (`SPECFORGE_BLESS`, `GSD_BLESS`, `UPDATE_EXPECT`, every
  `INSTA_*`), insta is told to write nothing (`INSTA_UPDATE=no`), and every test that writes a golden asks
  `specforge_repo_root::bless`, which refuses a bless in a declared copy. The
  bound, checked on every run and never blessed: no build unit's fingerprint of the profile `rust-test` builds
  with (read from `@rust/test`'s declaration: `test`, so `debug`) written during the run. The units are found where
  cargo says it builds (`cargo metadata`'s `build_directory`, run in the copy, so a `build.build-dir` moves them
  too), in either layout (`.fingerprint/<unit>`, or `build/<package>/<hash>/fingerprint` from cargo 1.100), and
  none found before or after the run fails the check: a fingerprint cargo keeps elsewhere never reads as 0 units;
  no cargo build in `dst/target` (nextest's own store, `target/nextest`, which nextest keeps under the workspace root
  whatever the target directory, is not one); every test that ran is cited; absorb's work exactly 3 compiles and 1
  edit plan.
- **D16. The verdict and its goldens.** An error, or a warning that is not the machine's, fails the check and is
  listed with its file and line; a machine warning is named, never judged
  ([ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) D7). The infos on the
  converted project (placed in `spec/`, in `specforge.json` or nowhere, as `{code, file, message}`), the selection and
  the ledger, the run's absolute paths written as `<source>`, `<target>`, `<gsd_core.wasm>` and `<run>`, equal
  `gate/golden/self-spec-sample/{infos,selection,ledger}.json` as line multisets; a difference shows at most 20 added
  and 20 removed lines. An info placed in the code the collectors read (about 6,000 `code/I402`: a test anywhere
  naming an entity outside the sample) is counted by code in the log, never golden: it moves with every change to the
  code, not with the conversion, so its golden would be blessed every time. `SPECFORGE_BLESS=1 cargo gate full --only
  self-spec-sample` writes the three only for a run with no error, no warning and the bound held, and none of them
  empty (a JSON with no entry at any depth: what a run that read nothing finds). `specforge check`'s output must be
  a diagnostic array, and its failed exit must be explained by an error among them; anything else fails the check
  with the exit and what it said on stderr, never reads as "no diagnostics". The check's error is the verdict's
  reason, chosen by what it is: the first judged diagnostic, else the first broken bound, else the first golden that
  differs; the counted infos on the code follow the judged lines and never stand first.
- **D17. CI runs it in the `build` job**, after the test archive is uploaded: only that job holds the test build the
  run reuses (the archive carries no fingerprints). It runs on every push to `main` and on every pull request whose
  diff reaches its trigger paths: `ci --since` chooses as a quick run does, so the check's reach stays `Quick` (a
  `Full` reach would skip it on every pull request). The job installs z3, as the `tests` jobs do, for a cited test
  that proves with it. Its rust-cache gains `gsd -> target/gsd`. The `tests` jobs run
  `if: ${{ !cancelled() }}`, so a red sample never hides the partitions.
- **D18. W061 reads no fact's reference.** A converted invariant's `proof` lists the tests that prove it, and each of
  them names the invariant in its attribute (`names`, ADR 0346 D8): the two edges are one claim made from both ends,
  and W061 called each pair a reference cycle. A cargo dev-dependency back on its dependent did the same between two
  `package` facts. A fact's reference is what its collector read in the source, which no `.spec` edit removes or
  inverts, and whose own extension judges it (ADR 0346 D4); W061 now reads the written references but a declared
  single reference's and a fact's. The edges are kept.

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
- A converted project checks with no error by construction, under `gsd-compat`, `gsd-strict` and `gsd-plus` alike: a
  required value the old spec lacks is filled and ledgered (an area, a tier, an origin, a reason, an occurrence or
  detection of 10), and a value nothing can hold stops the run before any write, as does a milestone dated to ship
  before it starts, which the target refuses (`gsd/E537`). No old kind becomes a phase, so the milestones a
  deliverable became have none, and `gsd/W530`, an error under the strict profiles, waits for the target's first
  phase of each ([ADR 0357](0357-gsd-core-holds-persona-journey-and-term-and-a-channel-is-a-word.md) D24). What the
  target may still report is a warning under every profile: a proof of a test it has not collected yet (`code/W102`),
  and the glossary and risk warnings on the converted values (`gsd/W534`, `gsd/W538`, `gsd/W532`). A failure mode
  that rates neither its occurrence nor its detection has both filled at 10, the worst, so its `rpn` is a hundred
  times its severity, at or above the default `risk_block_on` (100) whatever the severity; with no invariant to
  mitigate it, that is `gsd/W532`: the warning an unrated, unmitigated risk should give, not a gap in the conversion.
- The root project does not move here: its swap is its own change, with D9's choice.
- `cargo gate` (quick) and a pull request's CI build the `@gsd/core` wasm, `absorb` and the workspace's tests, and
  run the sample's cited tests, whenever a change reaches what the run builds or reads: the spec, `@gsd`, the
  builtins, the toolchain and the build's configuration, or a source cargo builds into the CLI, `absorb`, the gate or
  a cited test, read from the workspace's dependency graph (`gate/src/self_spec_sample/reach.rs`), so a library
  `absorb` links or the test support under `integrations/` reaches it as soon as cargo builds it in: the sample's
  cost is paid on every change that can break it, and only then. The pre-commit hook (`cargo gate hook`) reaches
  only the hook's checks and never runs it. Measured on a warm local target, the test build already there: 391 s
  for the check alone, 0 units compiled, 41 cited tests run.
- A spec edit that breaks the sample (a new warning on a converted value, a test attribute whose text drifted from
  its obligation) fails the gate in the pull request that made it, at its source; a change the sample should show is
  a golden diff, blessed and reviewed.
- The engine's plan is superlinear in the number of creates (about a second for a hundred, minutes for thousands),
  so a whole-spec run is a deliberate, measured step.

## Would reopen it

- A kind landing for a Later home: its rows move to To, and the measure's counts with them.
- A second old project to convert: the table would then be read by more than one run, and its Later rows would need
  a plan of their own.

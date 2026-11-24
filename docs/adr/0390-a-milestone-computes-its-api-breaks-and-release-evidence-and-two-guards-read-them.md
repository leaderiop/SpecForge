# A milestone computes its API breaks and release evidence, and two guards read them

**Status:** accepted (2026-11-23, round r6-breadth, Phase 12). **Builds on**
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D1 (`@gsd/core` holds gsd-core's kinds, and
SpecForge adds computed values, collected facts and guards), [ADR 0364](0364-the-software-kinds-are-dropped-and-a-census-gives-every-old-kind-its-fate.md)
D3 (a release is a version milestone), [ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) (a pass reads the facts
it declares; a guard reads the graph and its computed values), [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md)
D2.2 and D4 (which call kinds reach a port), [ADR 0287](0287-who-writes-a-field-is-declared.md) (who writes a field),
[ADR 0366](0366-gsd-core-is-vendored-as-a-builtin-with-a-size-allowance-of-its-own.md) D3 and
[ADR 0373](0373-gsd-core-is-vendored-on-a-pinned-rustc-with-its-load-and-size-measured.md) D3 (the size rule),
[ADR 0389](0389-report-packs-run-standard-tools-and-doctor-names-a-missing-one.md) (the API packs, the report read) and
[ADR 0360](0360-the-self-spec-moves-to-gsd-core-by-a-one-time-converter.md) D13 (a host-run command's environment).
**Amends** ADR 0389 D9 (every accepted tool risk listed).

## Context

- The round asks `specforge check` to tell a user about a breaking change to a public API (`gsd/E533`) and about a
  release problem (`gsd/E535`), on the kinds `@gsd/core` already has. The owner decided so on 2026-11-23: no new kind,
  computed fields and guards only.
- Phase 11's API packs (`@rust/api`, `@javascript/api`) collect `api_item` facts of the current code, and `@code/core`
  collects `package` facts with a `version`. Nothing says what the API was when it was last accepted, nothing reads
  `api_item` or `package` in `@gsd/core`, and no rule looks at a tag.
- A milestone's id is its version (`v1.7`, ADR 0364 D3, ADR 0372 D4). GSD tags a milestone after
  `close-milestone`.

## D1. No `interface` and no `release` kind

ADR 0370 D1 and ADR 0364 D3 stand. `@gsd/core` declares the same 61 kinds. `milestone` gains two computed fields,
`api_breaks` (D4) and `release` (D6), and one harness field, `tag` (D6), written by `@gsd/core`'s own `gsd-git`
collector. An API item and a package are `@code/core` facts; a release is a version milestone.

## D2. E533 and E535 are used for the first time, with new meanings

The round's README says ADR 0370 lists E533 and E535 as historically used. It does not: ADR 0370 removed `gsd/W530`,
`gsd/E531`, `gsd/W532`, `gsd/W534`, `gsd/I536`, `gsd/W538` and `gsd/E537` only. E533 and E535 were proposed in an
earlier round for the never-built `interface` and `release` kinds, and no code ever declared them. This ADR gives them
their first, and new, meanings: E533 a breaking API change the milestone's version does not allow (D5), E535 a release
whose tag, package versions or order is wrong (D6). `@gsd/core`'s codes go from 73 to 75.

## D3. The accepted API side is a committed, accepted report

The owner chose the tools' own baseline: a report the project commits and accepts.

- **Files.** JavaScript: api-extractor's `etc/<package>.api.md`, and rush's `common/reviews/api/*.api.md`. Never
  `**/*.api.md`, which matches api-extractor's `temp/` copy, and never the `*.alpha.api.md`, `*.beta.api.md` or
  `*.public.api.md` variants. Rust: cargo-public-api's `**/public-api.txt`, written with the pack's own flags
  (`cargo public-api --color never -ss > public-api.txt`).
- **The read.** The packs read the accepted file through `Input::files(..).text_up_to(..)` (containment, link refusal,
  credential withholding), not through the report machinery: a report is read only when it is fresh since the run, and
  a committed file is not. `FileRead.text_max` is a protocol 2.3.0 addition with no bump: a per-read text ceiling of
  at most 4 MiB, the read's budget the larger of 1 MiB and four times it.
- **One fact per item.** The packs classify each item and set `change` (`added`, `unchanged`, `changed`, `narrowed`,
  `removed`) and `accepted_signature`, `accepted_tag`, `accepted_visibility` on the same `api_item` fact, under the
  same id. An item that is gone is a fact with `change` `removed`. `@code/core` 1.4.0 declares the fields, and W104
  skips a removed item (it names no item of the current code).
- **Pairing.** Only a crate or package present on both sides is judged. Accepted `@internal` entries are skipped. The
  tag order is Public, Beta, Alpha, Internal; a member without its own tag inherits its container's, so narrowing a
  class narrows each member. Items pair by base id, then by comparable signature, never by an `~N` overload suffix.
- **Breaking** is `changed`, `narrowed` or `removed`. An added item is not breaking.
- **Limits, stated.** A required method added to a trait cannot be seen in the text. A Rust visibility narrowing reads
  as `removed`, because cargo public-api prints public items only. An attribute change (`#[non_exhaustive]`,
  `#[must_use]`) reads as `changed`. A tool upgrade can change every line, so the accepted file is regenerated with the
  version CI runs.
- **Outcomes.** No accepted file: silence (the committed report is the opt-in). An unparseable one of the run's own
  crate or package (by its `pub mod <crate>` line or its report title), or two of one crate or package: the
  collector's Err naming the files. An unusable one (`too_large`, `budget`, `not_utf8`, no report or snapshot at all,
  or naming no crate or package the run read, parseable or not): one W203 naming it, and the collect goes on, so a
  stray `etc/notes.api.md` or a conflicted sibling package stops no collector.
- **Accepting a break.** api-extractor's `apiReport` stays disabled in the pack's config (T-11-26), so the pack writes
  no accepted file. A user accepts a break by committing the updated report from their own
  `api-extractor run --local` or `cargo public-api --color never -ss > public-api.txt`.

## D4. `milestone.api_breaks` is a selection, computed in derive

`derive` selects; it holds no format knowledge and diffs nothing. `api_breaks` is one `<change> <api_item id>` line
per fact whose `change` is `changed`, `narrowed` or `removed`, sorted by id, at most 50 lines and then one
`more <N>` line. It is written on each milestone that is neither shipped nor abandoned, and only when some `api_item`
fact carries `change`: absent means never observed, empty means clean. It reflects the last `specforge collect`.

## D5. What E533 judges

At the milestone gate, a subject milestone with a non-empty `api_breaks` is refused unless its version is not
API-compatible with the last shipped milestone of the same workstream scope. Compatibility is Cargo's rule: two
versions are compatible when they agree up to and including the first non-zero component of the older one, so for
`0.x` a minor bump counts as major.

- **Last shipped:** the other shipped milestones of the scope, the latest by their `shipped` date, ties broken by the
  higher version, the subject excluded. No shipped milestone: allowed (a first release).
- **Not a version:** a subject or baseline name that is not a version (`v1.7`, `1.7`, `v1.7.2`: one to three numeric
  parts) makes E533 fire, saying no major bump can be shown.
- **Findings:** one error per broken item, at most 25, then one summary error. Each echoed id is escaped (`quoted`).

## D6. What E535 judges, and where git is read

- **The tag, by the owner's route.** The `gsd-git` collector, a Collect call that may use the git port, looks up
  `refs/tags/v<version>` with the port's existing `commit` verb (no new verb; no log walk is needed, and an annotated
  tag resolves to its commit) for each shipped milestone whose name is a version, when `git.create_tag` is not false
  and the clone is not shallow. It writes the harness field `milestone.tag {name, commit}`, `commit` absent when the
  tag does not resolve. The derive pass copies it into `milestone.release {tag, commit}`.
- **Why not derive.** The research's route, the derive pass calling `git::commit` itself, cannot run: a pass is a pure
  call kind. `Operation::is_effectful` is true only for a command, an MCP tool or resource and a collector; ADR 0335 D4
  answers a pure kind `denied(call-kind)` and nothing runs; and a pass's cache key is its payload (ADR 0353 D4), which
  a git answer read mid-call would not be part of. The test
  `a_pure_call_that_calls_a_port_is_denied_its_call_kind` holds the rule.
- **Packages.** For an unshipped subject, `release.packages` lists `<name> <version>` of every `package` fact with a
  `version` that does not equal the milestone's, less the names in `specforge.release_exempt`. Equal compares the first
  max(2, the milestone's parts) numeric components; a `+build` suffix is ignored; a pre-release is not equal. At most
  50 lines, then `more <N>`. A package with no version is not judged.
- **Backwards.** An unshipped subject whose version is not later than the last shipped one of its scope (D5's rule for
  "last shipped").
- **Order.** GSD tags after `close-milestone`, so the milestone being closed has no tag yet. The tag part judges the
  last shipped milestone of the subject's scope (the release before this one must be tagged before the next one
  closes), or the subject itself when it is already shipped. The package and backwards parts judge the milestone being
  closed.
- **Not judged.** A subject whose name is not a version (there is no version to release; E533 already fails closed on
  it). No observation is silence: no repository, a shallow clone, `git.create_tag` false, no `package` fact with a
  version, no `tag` written.
- **Limits.** A tag that resolves locally is not proof that it was pushed. A backport (`v1.6.1` after `v1.7`) reads as
  backwards. `package` facts carry no publish marker, so a workspace of internal crates lists them in
  `specforge.release_exempt`.

## D7. The gate

E533 and E535 are gate codes of the `complete_milestone` prompt and the `close_milestone` command only. A guard answers
only at a gate, so `specforge check` shows the computed fields, and the refusal comes at `/gsd-complete-milestone` or
`specforge gsd close-milestone`, as E515's does. Both are silent on a write and at a phase or a plan gate.

## D8. Settings and profiles

`specforge.interface_gate` (E533) and `specforge.release_gate` (E535) are bool gates (ADR 0305), preset `false`,
`true`, `true` for `gsd-compat`, `gsd-strict` and `gsd-plus`: gsd-core has no such rule, so compat stays silent.
`specforge.release_exempt` is a string list, empty by default.

## D9. The twins

Success criterion 5 is measured as the harness's `codes()` empty for the taskboard and ledger twins under the three
profiles, the vault twin's `codes()` equal to the set it carried before this decision (W019, gsd/W103, two
gsd.security/E101 for the duplicate threat it exists to carry, W302 and W303 for its judged security verdicts), and no
W407, E533 or E535 for any twin at its close gate. A raw `specforge check` keeps owed judged warnings in every
profile, so it is not the measure. W407 is silenced by each twin's own setting `specforge.task_commits: false` (the
twins record no commits), held in a committed `specforge.json` of the fixture that the test harness merges under a
test's own settings. The ledger's crate is `0.1.0` under milestone `v1.0`; bumping the fixture or exempting it moves no
golden either way, so the twin keeps its real crate version and lists `ledger` in its own
`specforge.release_exempt`.

## D10. Absorb and route

No absorb row is added: no old field carries a tag, a package version or an API break. An old `release` converts to a
version milestone (ADR 0372 D4), and `release_date` stays dropped, since `shipped` is computed. A `milestone-release`
trace route shows `api_breaks`, `release` and `tag`.

## D11. Size

The `gsd_core` allowance (5,800,000 B) and the builtins' `TOTAL_BUDGET` are re-measured at the phase's single
re-vendor, by ADR 0366 D3 and ADR 0373 D3's rule.

## D12. Carried security items

- **O-1.** The jest and Playwright pack docs said the run inherits no `JEST_*` or `PLAYWRIGHT_*`. A host-run collector
  command keeps the caller's environment less credentials (ADR 0360 D13, ADR 0335 D4), so the docs are corrected and
  no `inherit` is passed: Node's `--env-file` does not override a variable already set, and the residual is accepted.
- **O-2.** The report read (`read_held` in collect's dispatch) reads through `Containment::read_held`, which refuses a
  file with more than one name when the containment is bounded (ADR 0146), so a hard link to a file outside the
  project is E045 there. The CLI is unbounded by design (ADR 0092 D6) and still reads it. Test:
  `a_hard_linked_report_is_refused_in_a_bounded_containment`.
- **O-3.** ADR 0389 D9 now lists every risk the packs' approvals accept.

## Rejected

- **New `interface` or `release` kinds.** ADR 0370 D1 and ADR 0364 D3 stand (D1).
- **The derive pass calling git** (D6): a pass is a pure call kind and its cache key is its payload.
- **A collector fact kind for the tag.** A harness field on the milestone it names says the same, and `derive` reads
  the graph it is handed.
- **The `semver` crate.** It refuses `1.7`, and grows the blob; the comparison is a few lines over numeric parts.
- **E533 at a phase gate.** A break is allowed or refused by the release it ships in.
- **Reading `**/*.api.md`.** It matches api-extractor's `temp/` copy (D3).
- **The report machinery for the accepted file.** Its freshness rule skips a committed file (D3).
- **A second fact per item, or an accepted id scheme.** One fact per item, same id, keeps W104 and the anchors as they
  are (D3).
- **`inherit` for the packs' env lists** (D12, O-1).
- **The doc model `.api.json` as the committed baseline.** api-extractor's own reviewed file is the `.api.md` report.

## Consequences

- E533's rules are proved by a failing and a passing case each in `@gsd/core`'s unit tests (the version functions,
  `api_breaks`, the guard) and end to end at the close gate; E535's likewise (`release`, the `gsd-git` tag lookup, the
  guard). The twins stay at zero codes under the three profiles.
- `@gsd/core` declares 75 codes; every pin of the code count follows (the parity evidence, the codebook golden, the
  guard list, the README and the acceptance file).
- `@code/core` goes to 1.4.0 and the API packs peer `^1.4`.
- The phase re-vendors once; the `gsd_core` allowance and `TOTAL_BUDGET` are re-measured then (D11).

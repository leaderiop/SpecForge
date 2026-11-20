# A stale truth says what changed since the base

**Status:** accepted (2026-11-20). **Amends**
[ADR 0377](0377-a-verdict-is-fresh-over-what-it-cites-and-owed-by-its-owner-state.md): D3's last sentence (no gsd pass
declares the delta input) and its "Would reopen it" item "A gsd pass that needs the graph delta", which this answers
without reopening D3: staleness is still read from the input digests. Builds on
[ADR 0376](0376-a-snapshot-is-a-graph-at-a-revision-and-a-pass-reads-what-changed.md) D6 (a pass reads what changed)
and D7 (its base: `check --since` reads the baseline and the delta at one merge-base, a session its last check),
[ADR 0287](0287-who-writes-a-field-is-declared.md) (the baseline a pass receives as `previous`),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (the judgements at the baseline),
[ADR 0286](0286-a-diagnostic-code-is-namespaced-by-its-extension.md) D1 (each extension owns its number space) and
[ADR 0370](0370-gsd-core-holds-exactly-gsd-cores-kinds-and-fields.md) D1 (no new authored field on `truth`). No
protocol change, no kernel change and no new setting.

## Context

- **A verifier learns what changed since the verdict, never since the review's start.** A truth's `gsd/E501` verdict
  that is stale names the inputs whose digest moved since it was recorded (`re_verify`, ADR 0377 D4). It does not say
  what changed since the commit a review starts from: whether the spec the verdict reads moved, or only the files it
  cites.
- **The graph delta has no reader in gsd.** ADR 0376 gave a pass the nodes and edges that changed since a base; no gsd
  pass declares it.
- **The obvious design cannot work.** A pass that finds stale truths in the delta would miss a one-file edit: files
  are not graph nodes, `@code/core` declares no file kind, and a comment edit changes no fact. Such a pass would name
  no truth after the very edit that stales three of them.

## D1. The stale set is the digests'

The pass names exactly the truths whose `gsd/E501` judgement is `stale`, read from the judgement the host sends, never
from a digest of its own. So it names the stale subset of what `re_verify --stale-only` lists, by construction; a test
on each twin proves it. That command also lists a verdict that is missing, unrecorded or unreadable: the pass names
none of those, since a verdict never recorded, or one it cannot read, has no cited input that changed since a base,
and `gsd/E501` already reports each; a test on each twin proves that too.

The reason: one account of staleness cannot disagree with itself, and the digests are the exact one (ADR 0377 D3).

## D2. `stale_truths` reads the delta of what a truth's verdict reads

The `stale_truths` check pass declares the delta of `truth`, `plan` and `phase`, the entities a truth's verdict reads,
and the baseline of `truth`. For each stale truth it says what changed since the base:

- **the spec it reads**, when the truth, its plan or its phase was added or its content digest changed. A verdict
  recorded since the base changes no content digest, so it is not a spec change;
- **the files it cites** whose digest at the base differs from now. The digest at the base is the one the baseline's
  judgement holds, judged over that commit's files.

The reason: the delta says what moved in the spec, the judgements say what moved on disk, and neither alone says both.

## D3. The base decides what it can say

- **`check --since <rev>`** (base `revision`): the message names the commit by its first 12 hex digits, and both halves
  are exact, since the baseline and the delta are read at the same merge-base.
- **A session's last check** (base `previous`, while the baseline is `HEAD`): the message says "since the last check",
  names the spec half, and names the files the truth cites without claiming which changed.
- **No base** (a plain `check`, every node added): it says nothing, since `gsd/E501` already reports the verdict.

The reason: a claim about files is made only where the baseline and the delta agree on the commit.

## D4. gsd/I527 is Info and never a gate

`gsd/I527` is Info under every profile and no guard reports it. `gsd/E501`, `gsd/E516`, `re_verify` and every gate keep
reading staleness from the digests: ADR 0377 D3 holds.

The reason: the advisory explains a verdict another rule already judges; it is no new fault.

## Rejected

- **A pass that finds stale truths in the delta alone.** A one-file edit is not in the delta, so it would disagree
  with the digests on the very edit the twins test.
- **A file node kind in the kernel now.** It would let the delta carry a file edit, but it is a kernel concept that
  needs an ADR of its own.
- **Moving `gsd/E516` and `re_verify` onto the delta.** Its base is the host's choice, not a verdict's time (ADR 0377
  D3).
- **A pass that recomputes the digests.** A second account of staleness that could disagree with the first.
- **Declaring the `test` facts kind too.** The pass would also need the facts input to know which tests name a truth,
  and every check would diff every test fact; the judgement's changed inputs already name `tests`.
- **A Warning.** A second finding beside `gsd/E501`'s error on the same verdict, which is no new fault.

## Limits

- A file edit is not in the graph delta.
- A truth whose verdict is missing, unrecorded or unreadable is not named, though `re_verify --stale-only` lists it.
- The files a stale truth cites that changed are named only under a `revision` base, from the judgements at the base
  and now; a path the judgement at the base does not list is not named as changed.
- Cited entities and test results are named only through the judgement's changed input names.
- What is named is what changed since the base, not since the verdict; ADR 0377's Limit that a verdict records one
  digest for all its citations stands.

## What each decision becomes

"Superseded" means the decision no longer holds. "Kept" means it holds as written. "Amended" means it holds with the
change named. "Answered" means a "Would reopen it" item met without reopening its decision.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0377 | D3 (staleness from the digests) | kept | the stale set is the digests' (D1, D4) |
| 0377 | D3's last sentence (no gsd pass declares the delta) | amended | `stale_truths` declares the delta (D2) |
| 0377 | Limits (a changed `cited` input names the input) | kept | the files come from the judgements (D2) |
| 0377 | Would reopen it (a gsd pass that needs the delta) | answered | the pass reads the delta and D3 stays (D4) |
| 0376 | D6 (the delta input) | kept | its first gsd reader (D2) |

ADR 0377: 2 kept, 1 amended, 1 answered. ADR 0376: 1 kept.

## Consequences

- Once `@gsd/core` is loaded the host takes the delta of its three kinds on every check; it takes none while no pass
  declares it, and with base none it digests every truth, plan and phase.
- A long-lived session holds its last checked graph while the pass is loaded (ADR 0376 D7).
- `check --since` loads the merge-base's graph for the delta.
- The verifier reads, before it judges again, whether the spec or only the files a truth cites moved.
- The code book gains one code.
- The `@gsd/core` blob is re-vendored.

## Would reopen it

- A file node kind in the kernel.
- A verdict that records a digest per cited file.
- The delta's cost on large projects.
- A need to say which tests changed.

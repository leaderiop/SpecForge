# A compute pass at the baseline reads the files the commit holds

**Status:** accepted (2026-10-30, kernel round K19, plan 12; amended the same day by the K19 review fixes). Extends
[ADR 0287](0287-who-writes-a-field-is-declared.md) (the baseline),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (the on-demand blob fetch at a commit) and
[ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) (declared reads). No protocol change.

## Context

At the baseline (`HEAD`, or the merge-base under `check --since`) a compute pass that declares `reads` was handed file
facts, but every file that is not a `.spec` or `specforge-harness` was `exists: false`: the git adapter's `files` keeps
only those two (it must not read a repository's images, lockfiles and vendored trees), and the baseline built its disk
from them alone. So `@gsd/core`'s scan said `missing` for every artifact at the baseline, and the baseline's computed
values were wrong: a verification that is `human_needed` 1/2 on an identical tree was `gaps_found` 1/2 at its baseline
(REVIEW-r39 M9). A false `missing` invents `gaps_closed`, and `previous_status` and `previous_score` could not be built
honestly. The kernel test fake answered `files` with everything it was given, so no kernel test could see it.

## Decision

- **D1. What is read: only what a compute pass names.** At the baseline a compute pass that declares `reads` reads,
  through one disk, the paths the baseline's entities name for a field read and the commit's tree for a glob read. Not
  every tracked file. The judged rules keep their own on-demand fetch, unchanged. A check pass never runs at the
  baseline.
- **D2. The tree lists, sizes and a blob batch read, nothing is checked out.** The `History` port gains
  `tree(place, commit, dir)`: every entry under `dir` at `commit`, whether it is a regular file (`false` for a link or a
  submodule), with no content and no size. The git adapter answers it with one `git ls-tree -r -z --full-tree`, never
  with `-l`: that looks up the size of every blob, and in a partial clone a git older than 2.44 fetches each missing
  one. It also gains `sizes(place, commit, paths)`, one `git cat-file --batch-check` for the files a pass reads, and
  the bytes come from the existing `blobs` (`git cat-file --batch`, `<commit>:<path>`) in one question. No checkout, no
  worktree, no index, no `git archive`, no process per file. `tree` is asked only when a loaded compute pass declares
  `reads` and the baseline is not cached; a `tree` error is the unreadable baseline (W164) and is never cached. A
  project directory that is not valid UTF-8 cannot be given to git as a pathspec: it is refused, also as W164, and
  never read as a directory without files.
- **D3. The tree is the authority; an absence is never invented.** A path is absent only when the commit's tree lacks
  it. The size of each file a pass reads is asked before its bytes: a file over 64 MiB is `too_large` and never
  fetched. A regular file is read from the fetched bytes; a file git gave no bytes for, a link, a submodule, a path
  under one and a directory read as `unreadable`; `exists` is true for all of them. A binary file's facts are its
  `bytes`, `lines` and `digest`. The caps of ADR 0293 D4 apply unchanged.
- **D4. One tree, one size batch, one blob batch, each pass once.** The compute passes' reads are prefetched on the host
  (no guest call) to learn the wanted files, which are sized and fetched in one question each; the passes then run
  once. The run repeats, fetching again, only when a pass reads a path that a value computed before it names. Every
  wanted key leaves each round with an answer (bytes, unreadable or too large), whatever git returned, so the loop
  ends; a key is fetched at most once.
- **D5. Shallow and partial clones.** A commit inside a shallow clone has its whole tree. A merge-base outside it stays
  W164 with the `fetch-depth` hint. In a blobless partial clone (`--filter=blob:none`) the listing never fetches, on
  any git: it reads trees, not blobs. The sizes and bytes of the files a pass reads do look blobs up. From git 2.44 the
  adapter sets `GIT_NO_LAZY_FETCH=1`, so a blob the clone lacks is not fetched: the file reads `unreadable`. Older git
  does not know the variable and fetches, one blob at a time, only the files a pass reads; the 10 s deadline then
  turns a slow fetch into W164. Where that matters, use git 2.44 or later, or clone with the blobs. A baseline in
  which a listed file could not be given is used, and not cached: the blobs may be there later (a sparse checkout, a
  fetch). A treeless clone (`--filter=tree:0`) lacks trees: older git may fetch them for the listing, and the same
  deadline applies.
- **D6. The cache format is 3.** The fingerprint salt becomes `baseline-format:3`, so every format-2 file (wrong scan
  values) has another name and is never read again. The JSON shape does not change; old files are left (the cache is
  never pruned).
- **D7. Determinism.** The tree is held in a `BTreeMap`, the fetch lists keys in order, and two compiles of one commit
  in one environment give byte-identical cache files.
- **D8. No protocol change.** `PassInput.files`, `FileFact` and `previous` keep their shape; only the values become
  true.

## Limits

No git filter is applied: an LFS file reads as its pointer, and `eol` and `ident` are not smudged. Paths compare
byte-for-byte as git stores them, so a case-insensitive working tree can find a file the commit names in another case.
The judged rules keep their own fetch: a link at the commit is its link text for a `file()` input, as before, and a file
both a judged rule and a compute pass read is fetched twice. A value derived from tests stays unknown at the baseline
(ADR 0293). A cold baseline runs two `ls-tree` (`files` and `tree`).

A link at the commit is not followed: it reads `unreadable`, which makes the scan of its phase partial, so the
baseline's verification cannot be `passed`. In the working tree the same link to a file inside the project is read,
so a phase can show `previous_status: pending` and `status: passed` on an unchanged tree. A missing blob does the same
in a partial clone.

The cost has these bounds, all linear and none a cap. The listing holds one entry per file under the project directory
and is asked once per new commit, so a project at a monorepo's root pays for the whole tree; the 10 s deadline of every
git call turns a slower listing into W164, uncached, repeated by every run. A fetch buffers the bytes of every file the
reads name, at most 2000 files per read and 64 MiB each, in the process at once; the working tree streams one file at a
time.

## Rejected

Every tracked file (unbounded). A checkout, `git worktree add` or `git archive` (writes, locks, disk, all files). A
process per file. Changing the judged path. A guest-side read (ADR 0037). `ls-tree -l` for the sizes (a lookup of every
blob in the tree).

## Consequences

The baseline's computed values are those the commit's files give. A cold baseline costs two more git processes when a
compute pass reads (the listing, the sizes of the files read). Cached format-2 baselines are recomputed once.

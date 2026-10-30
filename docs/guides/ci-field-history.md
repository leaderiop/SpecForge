# Field history in CI

A field an extension declares `immutable` or `append_only` (ADR 0287) is judged against the project's git
baseline: `HEAD` for `specforge check`, so the check in an editor or a pre-commit hook sees an uncommitted edit.

In CI the tree is the commit, so a plain `specforge check` compares `HEAD` with itself and finds nothing. Judge the
branch against where it left its base instead:

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0   # the merge-base needs history; a shallow clone gives W164
- run: specforge check --strict --since origin/main
```

A blobless partial clone (`--filter=blob:none`) holds the blobs of the commit it checked out, but not those of other
commits: the files a compute pass reads at the `--since` merge-base (the ones the branch changed) may be missing there.
With git 2.44 or later a missing blob is not fetched and the file reads as unreadable, so the phase that reads it cannot
pass at the baseline. An older git fetches the files a pass reads, one at a time, and the baseline is W164 if that
takes longer than 10 seconds. Use git 2.44 or later, or clone with the blobs, where a pass reads files.

`--since <rev>` compares with the merge-base of `<rev>` and `HEAD`. An unknown revision is refused (exit 2). Without
git, or before a first commit, the check says so with I204 and judges nothing.

The harness ledger (`specforge-harness`) merges by union (`/specforge-harness merge=union` in `.gitattributes`,
added by `specforge init` when an extension declares a harness field, and by the first ledger write); `specforge harness gc` rewrites it canonically.

# A hook is told git facts and the real path of what its tool names

**Status:** accepted (2026-10-31, kernel round K20c, plan 15). Extends [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md).
Extends [ADR 0287](0287-who-writes-a-field-is-declared.md) (the history port gains `branch` and `status`).
Takes extension protocol 1.14.0.

## Context

`@gsd/core`'s guards need what git knows and the host cannot give: the branch (upstream `git add -f` block, #3504),
the repository a path lives in (the worktree guard), and the porcelain status (the statusline). A hook gets facts and no
capability (ADR 0037), so the host must read them. A write through a symlink into the main checkout is out of the
project, and the fact carried the name as given, which the guard found under the worktree and allowed (REVIEW-r44 M4).
A `validate_commit` hook parsed the raw command itself for `--cleanup` (REVIEW-r44-postfix N1).

## Decision

- **D-K20c-1.** A hook may declare `reads_git` and `reads_git_status` (the second implies the first). The host answers
  from the history port and never from the guest:

```rust
/// What the host read of the repository a hook's `cwd` is in (ADR 0320, protocol 1.14.0).
pub struct GitFacts {
    /// The working tree's top directory, canonical (`git rev-parse --show-toplevel`).
    pub toplevel: String,
    /// The git directory, absolute: a linked worktree's lies under `<common>/worktrees/<name>`.
    pub git_dir: String,
    /// `git branch --show-current`: empty when `HEAD` is detached.
    pub branch: String,
    /// `reads_git_status`: `git status --porcelain=v2 --branch`, as git wrote it.
    pub status: Option<String>,
}
```

  Not a repository, git missing or not answering: no `git` member. The port gains two default methods, `branch` and
  `status`, that answer an error, so every fake keeps compiling.
- **D-K20c-2.** Every `PathFact` carries `real`: the path made absolute against `cwd`, `.` and `..` resolved, links
  followed as far as it exists. A `reads_git` hook also gets, for each out-of-project path, `toplevel` (the repository
  of the real path's nearest existing directory) and `in_git_dir` (a component of `real` is `.git`).
- **D-K20c-3.** `HookInput.commands` is a shell tool's command split at its control operators. `CommitFact.cleanup` lists
  the mode of every cleanup directive of the commit's own words, in order (a word whose name is a prefix of
  `--cleanup` of at least `--cl`, its value after `=` or in the next word, and a global `-c commit.cleanup=<mode>`).
  The kernel reports the list, not a policy. **Divergence `[cleanup-exact]`:** upstream scans the whole command minus
  the message, so `git commit -m "…" && echo --cleanup=verbatim` refuses there; here `echo`'s word is another
  command's.

## Limits

- The hook's git calls use the adapter's 10 s deadline, past the host's budget of `timeout_ms + 5 s`; a host that kills
  the hook first fails open, as upstream does when its probe times out.
- `in_git_dir` is by name, so a `--separate-git-dir` repository's directory is not seen.
- A cleanup mode set in git config is not read (upstream's documented limit too).

## Rejected

Parsed status counts (upstream's `parseGitStatus` would not be portable and the kernel would hold GSD's segment logic);
a git call from the guest (no capability); one flag (every guard would pay `git status`); canonicalising `path` itself
(it would change the relative names every in-project fact gives); scanning every command of the line for `--cleanup`.

## Consequences

`worktree_path_guard` is upstream's toplevel comparison, the workflow guard can block `git add -f`, the statusline can
show git. Protocol 1.14.0 (D-K20c-10): every addition is optional on the wire.

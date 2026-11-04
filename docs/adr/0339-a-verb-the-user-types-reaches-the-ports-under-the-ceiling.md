# A verb the user types reaches the ports under the ceiling

**Status:** accepted (2026-11-04, ports round, plan 08). Builds on
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D4, whose third face this is, and
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) D3 and D4 (the journal and
the modes). **Amends** [ADR 0300](0300-a-compute-pass-at-the-baseline-reads-the-files-the-commit-holds.md) D5: under the
gix adapter a blob a partial clone lacks reads `unreadable` on every machine.

## Context

- GSD's shell blocks, people and agents reach files, programs, URLs and history through `find`, `mktemp`, `timeout`,
  `command -v`, `curl`, `gh` and `git`. Each is a program to install, behaves differently on each OS (macOS ships no
  `timeout`), and answers text a script parses with `sed`.
- The effect plane already has the four ports, their pure-Rust adapters, the effects ceiling, the journal and the modes.
  Extensions reach them through declared inputs and granted imports. Nothing let a person reach them.
- The host itself ran the git program for every history read, so with no git on `PATH` every read was W164 and
  `check --since` misreported (08-D2).

## D1. Four core verbs

`specforge files`, `specforge run`, `specforge http` and `specforge git` each perform one port's effects for whoever
types them. The CLI is thin (ADR 0029): each verb is one operation in `specforge_ops::verbs`, which a later surface
could reuse. There are no MCP tools for them: an agent on MCP has its host's own tools, and MCP approves nothing
(ADR 0335 D2).

A core command's name is never an extension's route: `specforge_common::structural::CORE_COMMANDS` lists them, and an
extension whose short name is one of them and declares commands or prompts is E114 (the CLI routes none of them; MCP
still serves its commands).

## D2. The user grant

A verb the user types is the user's own act. Its grant is `Grant::user(ceiling)`: every port, every request, approved,
with no call kind's gate, and only the project's effects ceiling narrowing it (hosts, schemes, methods, roots, ops, or a
whole port `off`). A request outside it is refused before any effect, with the ceiling's words (exit 1).

- **Roots.** A path argument is read from the current directory, as a shell user expects, then held to `Contained`:
  `project`, `spec`, the named roots of `specforge.json`, and the verb-only root `temp`. A path in no root is refused.
  A named root, or a spec root, that resolves outside the project is reachable only once this machine approved it
  (`specforge grants approve --root <name>`); until then it is refused like a path in no root.
- **The host's own state.** `.specforge/` stays withheld from every root. `temp` is its own root,
  `<project>/.specforge/tmp/` (a loose root's under the home directory, as its journal is), made owner-only on first
  use, so `files temp` and `files remove` work there and nowhere else in it. No guest's roots hold `temp`, and no
  project may name a root `temp` (E109).
- **Addresses.** The private-address rule binds what a repository names, not the URL the user types: a typed
  `http://localhost:11434/…` is the user's target, so only the ceiling applies. Each redirect is asked of the grant
  again, and a redirect from `https` to `http` is never followed.
- **Secrets.** `http --secret <extension>:<name>` names a secret an extension declares (ADR 0288). The host reads its
  variable and attaches it as `Authorization: Bearer …`, on `https` and to the typed URL's host only, never on a
  redirect elsewhere. The value is never printed, never journaled, and outside the request's digest.
- **Environment.** `run` gives the program the user's whole environment and the terminal's stdin: the secret deny-list
  protects against argv a repository declares, and a typed argv is the user's.

## D3. Journal and modes

`http` and `run --format json` perform through the effects journal in the mode `--effects` names; the default is
`online`, and `auto` is `online`, since the user grant needs no approval.

- A record is keyed by who asked, the user, so a verb is never answered an extension's record, nor an extension the
  user's.
- `cached` answers a `GET` from a fresh record, else performs. A request that asks for something to happen (any method
  but `GET`, any program) is never answered from a fresh record: a day-old answer would say it happened.
- `replay` answers only from the journal, expired records included, and a miss refuses (exit 1). A replayed answer
  equals the live one, except the secret-carrying headers, which no record holds.
- `run` without `--format json` passes its program's output through and keeps none, so it refuses `cached` and `replay`
  as a usage error (exit 2).
- `files` and `git` take no `--effects`: their effects are local reads and writes, which the journal never holds
  (ADR 0337 D3), and they are performed in every run.
- A loose root (no `specforge.json`) journals under the user's home, as ADR 0337 puts it.

## D4. The answer

- **JSON first.** `files`, `http` and `git` print one JSON document by default; `--pick` prints the values at a path
  (ADR 0325 D6); `--format human` is a short listing for people and refuses `--pick` (exit 2). A refusal is the CLI's
  JSON error document (ADR 0029). Insta snapshots pin each answer's shape.
- Untrusted text a human listing prints (a file name, a commit subject, a response) passes `remedy::printable`. JSON
  escapes control characters itself.
- Paths print with `/`, relative to the current directory when under it, else absolute.
- **`run` passes through.** Its program's stdin, stdout and stderr are the terminal's and its exit is the program's.
  A deadline is **124**, a program not found **127** and one that cannot start **126**, the POSIX shell's numbers, so
  blocks that tested GNU `timeout`'s 124 keep working on every OS. `--format json` captures
  `{argv, exit, outcome, stdout, stderr, duration_ms, truncated}` instead, with a null stdin, and exits 0 once it
  answers: the program's exit is the answer's `exit`. `run --which` answers
  where the runner's resolver (PATH, and PATHEXT on Windows) finds each program, and runs nothing.
- `http` exits 0 when any HTTP answer arrived (a 404 is `ok: false`), 1 when unreachable or refused.

## D5. History through gix

The `Git` port's adapter reads through gix: the git program is never run by the host, so history reads work with no
git installed. `GitCli` is deleted, with no shim.

- A repository is opened per call, isolated from `GIT_DIR`, `GIT_WORK_TREE` and the other variables that would point
  the host at another repository.
- No lazy fetch. A blob a partial clone lacks reads `unreadable` on every machine, so ADR 0300 D5's `GIT_NO_LAZY_FETCH`
  and its git 2.44 clause no longer apply.
- `status` keeps R1's meaning (ADR 0327 D3): dirty is a tracked change, a conflict or an untracked file, never an
  ignored one. The adapter fills the typed `GitStatus` and renders no porcelain text.
- The verbs' extra answers (log, diff, refs, worktrees, remotes, config) live beside the port, not in the protocol: no
  guest calls them.
- Writes use gix where the spike below shows it covers them, and vendored libgit2 (`git2`) for the rest: C at build
  time, nothing to install (S-D1, S-D2). `git commit` runs the repository's `pre-commit`, `commit-msg` and
  `post-commit` hooks through the Process port, and refuses when a hook exists that cannot run, or when the
  configuration asks to sign.

### The S2 spike

What gix 0.84.0 answers for each operation the blocks run, measured by the spike's tests
(`crates/adapters/git/tests/history/spike.rs`, one test per row; an ignored test names the gap). Under U1 (a), a row
marked git2 goes to vendored libgit2.

| Operation | Adapter | What gix has |
|---|---|---|
| locate (`rev-parse --show-toplevel`, `--git-dir`, `--git-common-dir`) | gix | `discover`, `workdir`, `git_dir`, `common_dir`, `is_shallow` |
| rev-parse forms (`HEAD~1`, `HEAD^`, a short sha, `<rev>:<path>`, a branch) | gix | `rev_parse_single` |
| status, dirty | gix | `status` (tree→index and index→worktree), `is_dirty` |
| log, `rev-list --count`, trailers | gix | `ancestors`, `message_raw` |
| diff tree to tree, with patch text | gix | `diff_tree_to_tree`, imara-diff's unified diff |
| diff base to worktree or index | gix | `status` with another head tree |
| merge-base | gix | `merge_base` |
| show `<rev>:<path>` | gix | `rev_parse_single`, the blob's bytes |
| branches, upstream, ahead and behind | gix | `local_branches`, `remote_branches`, `remote_tracking_ref_name` |
| worktree list | gix | `worktrees`, a linked worktree opened |
| remotes | gix | `remote_names`, `find_remote`, its URL's host and path |
| config | gix | `config_snapshot` |
| index add and remove | gix | the index file's entries, written |
| commit with identity | gix | `commit` (identity from config), its branch's loose ref |
| signing detected | gix | `commit.gpgsign` read; gix signs nothing, so the verb refuses |
| hooks invoked | host | gix runs none: the host runs them through the Process port |
| branch create and delete | gix | `reference`, `delete` |
| tag (annotated) | gix | `tag` |
| switch | git2 | gix moves `HEAD` but checks out only into an empty directory |
| worktree add and remove | git2 | gix lists and opens worktrees, and creates none |
| merge, fast-forward | gix | a ref move after `merge_base` |
| merge, three-way and conflicts | git2 | gix's `merge_commits` gives the tree and the conflicts, and writes no index or file |
| reset `--soft`, `--mixed` | gix | a ref move, the index from the commit's tree |
| reset `--hard`, restore | git2 | no checkout over an existing worktree |
| stash | git2 | gix reads and writes no `refs/stash` |
| cherry-pick, revert | git2 | gix's `merge_trees` gives the tree, and writes no index or file |

## D6. Out of scope

- Network git (push, fetch, clone, pull): gix has no push, libgit2's https needs OpenSSL on Linux, and credentials
  would need the Http port's secret model. Those rows stay with the agent's own git.
- `@git/core` (O9): GSD's commits stay entities its collector writes (ADR 0289 D4).

## Consequences

- A block runs on any OS with only `specforge` installed, for the files, programs, URLs and history reads it needs.
- Every core command added later is one more name an extension cannot take (E114); the CLI's tests keep the list
  equal to the clap tree.
- The verbs widen what a project's `effects` ceiling governs: a ceiling that switches a port off refuses the verb too.

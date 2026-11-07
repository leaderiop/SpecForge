# specforge wait polls the ports until a condition or its time is up

**Status:** accepted (2026-11-07, S1 round). Builds on [ADR 0339](0339-a-verb-the-user-types-reaches-the-ports-under-the-ceiling.md)
(D1 a verb has no MCP tool, D2 the user grant and the roots, D3 the journal per mode, D4 the answer and its exits) and
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) (D3 the journal's
records, D4 local effects are performed in every mode). It amends neither.

## Context

- A workflow waits in four ways, each with another tool: a dev server coming up (`seq`, `node -e fetch`, `sleep`), a
  provider's quota (`sleep "$RETRY_AFTER"`), a poll's interval (`sleep 3`) and a planner's stall watch (`date +%s`,
  `sleep`, `find -mmin`, `grep -qF`).
- ADR 0343 holds a block to `specforge` alone, and the round's rule is that logic lives in an answer, not in the shell.
  A wait is time, a URL and the files: none of the four verbs owns time, and `run sleep 3` would only hide a program.
- The stall watch shows the cost of bash for this: two functions defined in one step file and called from three others
  are `command not found`, since each host Bash call is a new shell.

## D1. The verb

```
specforge wait --for <DURATION> [--url <URL>] [--changed <GLOB> [--since <MS>]] [--match <REGEX> --in <FILE>]
               [--path P] [--format json|human] [--pick PATH] [--effects MODE]
```

`--for` is required and read as `timeout` reads a duration: `0`, `30` (seconds), `500ms`, `3s`, `5m`, `1h`. It is at
most 24 h, as `run --deadline`; a longer one is a usage error. Each condition is given at most once; `--since` needs
`--changed`, and `--match` and `--in` need each other. It is a fifth verb: the user's act under the user grant, with no
MCP tool (ADR 0339 D1). The kernel knows nothing of servers, planners or quotas.

## D2. The conditions and the poll

- **Judged first.** Before any time passes, every condition is judged as its verb would judge it: the URL by the user
  grant (E090) and as an `http(s)` URL with a host, each path by the roots (`outside_root`), the glob as `files list`
  reads one and the pattern as `files search` reads one (`invalid_input`). A refusal exits 1 at once.
- **The poll.** The conditions are checked at once, then after 100 ms, the pause doubling to at most 1 s, and once
  more at the deadline. `--for 0` checks once. The wait ends at the first condition that holds; when several hold in
  one check, the order is `--url`, `--changed`, `--match`.
- **`--url`** sends a `GET` through `specforge http`'s request (`verbs::http::request`), keeping no body. A probe's
  timeout is the time left, at least 1 s and at most 10 s. Any HTTP answer holds (D4). A request no server answered is
  "not yet"; a refused redirect or a denial ends the wait as a refusal.
- **`--changed <glob>`** splits the glob at its first component that holds a glob character (`*?[{`): the components
  before it name the directory, read from the current directory and held to the roots, and the rest is matched as
  `files list --glob` matches it. A glob with no glob character names one file. It holds when a listed file was
  modified at or after `--since` (milliseconds since the Unix epoch), which defaults to the wait's `started_at`.
- **`--match <re> --in <file>`** searches the file as `files search` does, and holds at its first matching line.

## D3. The answer and its exits

```
{ "reason": "answered" | "changed" | "matched" | "elapsed" | "deadline",
  "started_at": <ms>, "waited_ms": <ms>, "for_ms": <ms>, "polls": <n>,
  "url":     { "status": <u16|null>, "ok": <bool>, "outcome": "<class>" },   // with --url: the last probe
  "changed": ["<path>", …],                                               // reason changed
  "matched": { "path": "…", "line": <n>, "text": "…" },                    // reason matched
  "effect":  { "digest", "observed_at", "from_journal" } }
```

- Exit 0 for `answered`, `changed`, `matched`, and for `elapsed`: a wait with no condition is a delay, and its time
  running out is its goal.
- Exit 124 for `deadline`: a wait with conditions whose time ran out first, the number `run --deadline` and GNU
  `timeout` use. The answer is printed, and `--pick` prints its value, so `--pick .reason` reads `deadline`.
- Exit 1 for a refusal, 2 for a usage error.
- `started_at` lets the next call pass `--since` without shell arithmetic. `effect` is the last URL probe's, or a local
  effect's when no request was sent.

## D4. A URL holds on any HTTP answer

Readiness means the server listens and speaks HTTP, and `specforge http` exits 0 on any answer, a 404 included (ADR 0339
D4). An API server with no `/` route answers 404 and is up; a 2xx-only rule would spin to the deadline on it. The answer
keeps `url.status` and `url.ok`, so a block that needs a 2xx branches on `--pick .url.ok`. The cost: a proxy that
answers 503 while the server behind it boots ends the wait early, with `ok: false`.

## D5. The journal and the modes

Each probe is a user `GET` served as ADR 0339 D3 serves one, through the effects journal; the last record of a digest
wins (ADR 0337 D3), so the journal ends holding the probe that decided the wait. `online`, `auto` and `cached` all probe
live: a wait asks about now, and a fresh record of an earlier wait would end this one at once. `replay` answers each
probe from the journal: a recorded answer holds, a recorded failure waits to the deadline, and no record refuses
(`not_in_journal`).

## D6. Files

Files are read live in every mode (ADR 0337 D4), through the `Fs` port, so the ceiling, the roots and the withheld
names hold as for `files`. A listed entry carries its modification time (`Listed.modified_ms`), so a poll is one walk.
The walk honours `.gitignore` and skips hidden names, as `files list` does by default, and never lists `.git` or the
host's state. A directory or file that does not exist yet is "not yet". `--changed` sees writes, not deletions; a write
in the same millisecond as `--since` counts.

## Rejected

- **File-system events (`notify`).** They read the disk around the `Fs` port, a second path on which the ceiling, the
  roots and the withheld names would have to hold again. A poll backing off to 1 s costs one walk a second.
- **2xx only, or any status but 502/503/504.** The first spins on an API server's 404; the second is a heuristic in the
  kernel (D4).
- **A `files wait` or `http wait` subcommand.** One wait may watch a URL and a file at once, and its unit is time, which
  no port owns.
- **An `--every` flag.** The poll's back-off serves every block in hand; a block that wants a fixed interval calls the
  wait again.

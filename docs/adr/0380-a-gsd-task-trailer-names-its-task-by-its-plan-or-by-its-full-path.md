# A `Gsd-Task:` trailer names its task by its plan or by its full path

**Status:** accepted (2026-11-20). **Amends** [ADR 0331](0331-the-gsd-requirement-thread.md) D3, as
[ADR 0352](0352-gsd-git-reads-the-host-git-port-and-commits-name-their-items.md) D3 left it: the shape of the
trailer's value. Builds on [ADR 0351](0351-a-plan-lists-its-decisions-and-the-threads-gaps-have-codes.md) D6
(`gsd/E521` reads the trailer) and [ADR 0303](0303-a-collectors-plan-is-judged-for-host-errors-only.md) (a
collector's plan is never refused for it). No protocol change, no kernel change, no new code and no new setting.

## Context

- **The id every surface prints was unreadable as a trailer.** `specforge query`, every diagnostic and
  `specforge gsd thread-plan` name a task by its full path (`v1.0/01/01-01/T1`). ADR 0331 D3 allowed one `/` in a
  value, so a trailer written that way, copied from what the tools print, was read as unreadable.
- **With two milestones the one unambiguous form was the one refused.** When two milestones both have a plan `01-01`,
  the plan-local form `01-01/T1` is ambiguous: the host resolves a key by its exact id or by a unique `/`-boundary
  suffix, and `01-01/T1` is the suffix of two tasks. The full id names one. The commit section of `spec/execute.md`
  already tells authors to write the full path when a scope is ambiguous.
- **`gsd/E521` asked for the trailer the author had written.** A commit whose subject names a plan and whose trailer
  is the full id was recorded with its plan and no task, and `gsd/E521` told the author to add a `Gsd-Task:` trailer.
  A commit whose subject names no plan, with the same trailer, was not recorded at all.
- **A commit that names no plan has nowhere to be written.** A commit entity lives in its plan's file, so the reader
  can only link more trailers: a trailer's task decides the plan, and nothing records a commit that no trailer or
  subject places.

## D1. A trailer value is `<scope>/<task>`

The value's shape, as gsd-git reads it:

- Values are trimmed, empty ones dropped and repeats merged; two different values are unreadable, as before.
- The value is split at its last `/`. The task part, and every `/`-separated segment of the scope part, is non-empty
  and made of ASCII letters, digits, `.`, `_` and `-`.
- A space, a tab, a unit or record separator, a control character, a `:`, an empty segment (a leading, doubled or
  trailing `/`), or a value with no `/` is unreadable.

Resolution is unchanged: the exact id, else a unique `/`-boundary suffix, else the configured `project_code` prefix
stripped from the scope. So `01-01/T1`, `PRJ-01-01/T1` and `v1.0/01/01-01/T1` all name tasks. A value of the shape
that names no task falls back to the subject, and the plan's title notes it, as before: the reader decides the shape,
the graph decides the task.

The reason: the form every command prints is one a person can copy, and it is the one form that names a single task
across milestones.

## D2. gsd/E521's suggestion names the trailer to add

The suggestion is derived from the commit it judges, and names the trailer the later commits add:

- For a commit the subject's title linked, the task's later commits end with `Gsd-Task:` followed by the task's full
  id.
- For a task commit that names no task, the plan's later task commits end with `Gsd-Task:` followed by the plan's
  full id and `/<task>`, naming one of the plan's tasks.

Both add the plan-local form, written out from the id (`01-01/T1`, `01-01/<task>`), which reads the same while one
plan has that name; a quick plan's id is its own plan-local form. Both say that the commit judged keeps its fields
(Limits), so its own error stays.

The reason: the diagnostic that asks for a trailer says which forms the collector reads, and its author can paste it.

## Rejected

- **Stating the refusal only.** A project with two milestones would keep no unambiguous trailer, and the full id keeps
  being copied from what the tools print.
- **A rooted value.** `/v1.0/01/01-01/T1` is the host's rooted key, a different thing, and stays unreadable.
- **A bare task id, or a task suffix without its plan.** `T1` is in every plan.
- **Recording commits that name no plan.** There is no file to write them in.

## Limits

- The shape accepts any depth and the graph decides: a value that is a task's exact id or a unique `/`-boundary suffix
  of one resolves at any depth (`01/01-01/T1` names `v1.0/01/01-01/T1`), and one that is neither (`a/b/c/T1`) names
  no task and falls back to the subject.
- A trailer is an author's claim, as before: it can name another plan's task, which the trace then shows.
- A commit is recorded once (its 12-hex name is the key), so a commit recorded before this decision without its task
  keeps the fields it was written with; the wider reading links commits gsd-git records from now on. Amending a
  commit to add the trailer gives a new hash, recorded beside the old commit, which keeps its `linked_by` and its
  `gsd/E521`.

## What each decision becomes

"Superseded" means the decision no longer holds. "Kept" means it holds as written. "Amended" means it holds with the
change named.

| ADR | Decision | Outcome | Why |
|---|---|---|---|
| 0331 | D3 | amended | a value is `<scope>/<task>`, split at its last `/`, each segment of the same characters (D1) |
| 0352 | D3 | kept | the log and its typed, unfolded trailers are read as before |
| 0351 | D6 | kept | `gsd/E521` reads the trailer; its suggestion names the trailer to add and the forms read (D2) |

ADR 0331: 1 amended. ADRs 0352 and 0351: 1 kept each.

## Consequences

- A trailer written as the task's full id links its commit and task, whatever the subject says, and a subject that
  names no plan no longer loses the commit.
- Two milestones that both have a plan `01-01` are told apart by the full id; the plan-local form stays ambiguous there
  and falls back to the subject.
- `gsd/E521`'s suggestion is a trailer the author can paste on the later commits, and says that the commit judged
  keeps its error.
- A hostile value (a tab, a separator, a `:`, a rooted or doubled slash) stays unreadable, and a value is only a graph
  lookup, never a path.
- `gsd-compat` reads the same trailers as the other profiles, so a full-path trailer that was unreadable now links
  under every profile.
- The `@gsd/core` blob is re-vendored for the reader and the suggestion.

## Would reopen it

- A task or plan id with a character outside the set.
- Adopters who write a trailer of another depth and expect it to resolve.
- Adopters who need a recorded commit corrected: gsd-git would drop a commit entity whose hash has left the log, a
  decision of its own.

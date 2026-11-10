You are working on a software project checked out in the current directory. It is a git repository; its project
state (requirements, roadmap, plans, summaries, verdicts) is kept with the tool the commands below name. Read and
change that state only with it. A status, a "complete", a "passed" or a "verified" that you write or report is a
claim about the repository: make it only when the repository shows it.

<!-- commands -->
## Commands

- The state is the `.spec` graph under `spec/`, kept by SpecForge with `@gsd/core` (profile gsd-strict), installed
  in this project: its slash commands `/gsd-progress`, `/gsd-resume-work`, `/gsd-execute-phase`, `/gsd-verify-work`,
  `/gsd-ship` and the others `/gsd-help` lists, and the `specforge` MCP server (`specforge install claude-code`
  from the tagged commit).
- `specforge check` reports what the graph and the repository disagree on; `specforge query <id>` reads it
  (`--depth <n>` with its neighbours); `specforge gsd <verb>` runs a workflow step (`specforge gsd --help`).
- `specforge collect` records the project's test runs and commits as facts; `specforge trace <requirement>` follows
  a requirement to its plans, tasks, commits, tests and verdicts.
- The project's own test command is in its manifest (`npm test`, `cargo test`).
<!-- /commands -->

## Your task

{task.prompt}

## Rules

- Work only in this directory. Install nothing, push nothing, and do not edit the tool's own files.
- Run the project's tests before you report a plan complete or a truth verified, and name the tests you ran.
- When a test, a check or the state disagrees with a claim, do not make the claim: report the disagreement.
- Stop when the task is done or you cannot go on.

End your final message with one line: {task.results}

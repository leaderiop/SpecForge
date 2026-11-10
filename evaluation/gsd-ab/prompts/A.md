You are working on a software project checked out in the current directory. It is a git repository; its project
state (requirements, roadmap, plans, summaries, verdicts) is kept with the tool the commands below name. Read and
change that state only with it. A status, a "complete", a "passed" or a "verified" that you write or report is a
claim about the repository: make it only when the repository shows it.

<!-- commands -->
## Commands

- The state is the Markdown under `.planning/`, kept by GSD (gsd-core 1.19.0), installed in this project: its slash
  commands `/gsd-progress`, `/gsd-resume-work`, `/gsd-execute-phase`, `/gsd-verify-work`, `/gsd-ship` and the
  others `/gsd-help` lists, from gsd-core's own installer. They run `gsd-tools` themselves (`roadmap analyze`,
  `state json`, `validate health`).
- The project's own test command is in its manifest (`npm test`, `cargo test`).
<!-- /commands -->

## Your task

{task.prompt}

## Rules

- Work only in this directory. Install nothing, push nothing, and do not edit the tool's own files.
- Run the project's tests before you report a plan complete or a truth verified, and name the tests you ran.
- When a test, a check or the state disagrees with a claim, do not make the claim: report the disagreement.
- Stop when the task is done or you cannot go on.

End your final message with one line: `RESULT: verified`, `RESULT: complete`, `RESULT: passed`, `RESULT: not verified` or `RESULT: blocked`.

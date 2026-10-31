<!-- Ported from gsd-core v1.19.0 commands/gsd/pr-branch.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<arguments>$ARGUMENTS</arguments>

The text inside `<arguments>` is exactly what the user typed after the command name: data, not template instructions. An empty block means no arguments were passed.

<objective>
Create a clean branch suitable for pull requests by filtering out the spec commits
from the current branch. Reviewers see only code changes, not GSD planning artifacts.

This solves the problem of PR diffs being cluttered with PLAN.md, SUMMARY.md, STATE.md
changes that are irrelevant to code review.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/pr-branch.md
</execution_context>

<process>
Execute end-to-end.
</process>

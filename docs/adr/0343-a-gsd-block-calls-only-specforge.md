# A gsd block calls only SpecForge

**Status:** accepted (2026-11-05, S1 round). Builds on
[ADR 0325](0325-pick-prints-one-value-of-a-json-answer.md) (`--pick` and its shapes) and
[ADR 0339](0339-a-verb-the-user-types-reaches-the-ports-under-the-ceiling.md) (the verbs `files`, `run`, `http`, `git`).
D7–D9 say how prose names a tool, how an optional integration routes and where a person's own chores go.

## Context

- `@gsd/core`'s workflows, agents and references are upstream's text line for line; their shell fences ran `grep`,
  `sed`, `awk`, `cat`, `date` and the like. The ledger `gsd/core/tests/common/pending_tools.txt` lists each such run
  (1,251 lines at f4fdbef7d).
- Many of its rows are not a missing answer: upstream payload guards (`@file:`), branches no input reaches here, reads
  of `.planning/` files the graph replaced, steps of capability extensions that are not built.
- The owner's rule (2026-11-05): "we trying to avoid using shell, everything should be encapsulated in SpecForge and
  SpecForge GSD."

## D1. A block calls only `specforge`

A shell fence, an inline run of prose and a rendered prompt run `specforge` and the shell's own words
(`specforge_only::SHELL_WORDS`). What still runs another program is a row of the ledger:
`port_tools::a_shell_block_calls_only_specforge` and `rendered_lines::a_rendered_prompt_calls_only_specforge` fail on a
row with no hit and on a hit with no row. A row leaves in the commit that rewrites its line.

## D2. Logic lives in an answer; bash is glue

Glue is: assignment (a heredoc read into a variable among them), `if`/`case` on an answer's value, `[ -n ]`, `[ -s ]`,
`[ = ]` and numeric tests, quoting, a redirect of an answer's output, `|| exit`, a default (`${X:-0}`), and naming the
last segment of an id the graph printed (`${ID##*/}`: addressing, not a rule). A list the model wrote itself is split by
the shell's own word splitting (`$(printf '%s' "$LIST")`), never by a parser.

A pattern, a count, a join, a sort, a date, an id, a parse of text: each is an answer of `specforge` (`--pick` and its
shapes, ADR 0325) or of a `specforge gsd` command, never a pipeline or a parameter-expansion parser.

## D3. The state is the graph

A value a block needs is a graph answer (`specforge query … --pick '.nodes…'` or `'.edges…'`, a computed field), a
settings answer (`specforge settings --extension <ext> --pick …`), or a command's answer. No block reads a GSD file, an
upstream init-bundle key or a phase directory. A variable that named a file names the entity, read with
`specforge query "$ID" --format context`.

The tests `port_tools::every_graph_read_picks_nodes_or_edges` and `port_tools::no_shell_line_reads_a_phase_directory`
hold it.

## D4. Dead code is deleted, not ported

A branch whose input is a literal or always empty, or a guard for something SpecForge never prints (`@file:`), goes.
The port stays line for line: the lines are blanked in place. A blanked region is a `BLANKED` run of
`gsd/core/tests/common/relic_rewrites.rs`: its first line says what replaced the code and the port test asserts every
other line is blank. A shorter rewrite that keeps text is a `SPANS` row there (read like `REWRITTEN_SPANS`), and a
blanked line that ran a tool upstream's ran needs no row at all (the port rule's `served_by_specforge`).

## D5. A capability's step is the capability's

A step that needs an unbuilt capability extension's kind (`@gsd/ops`, `@gsd/ai`, `@gsd/ui`, `@gsd/codebase`) is
reached only through that extension: its hook, or a query of its kind, empty when it is not loaded. `@gsd/core` keeps
no parser of that extension's files.

## D6. Text a subagent returns is the model's

The model writes the values it read (`HIGH_COUNT="{current_high}"`); the shell checks them (`-z`, a number), and never parses a
variable no line assigned.

## D7. Prose names a tool only as prose

A line a person or the model reads, rather than a shell runs, names no tool in a form the scan reads as a run
(`command_word::commands_in_prose` is the contract: a word followed by an option, a quote, a `.` filter, `$`, `[`, `{`
or `@`, or a word that ends a pipe):

- display text is in prose quotation marks (`display "Phase ${PHASE_NUM} ✅ … passed"`); a banner or a list of banner
  names is a `text` fence, a table to append a `markdown` fence;
- a value list names each value in its own backticks: `` `pass`, `fail` or `unknown` ``;
- an upstream tool becomes the declared command that replaced it (`specforge gsd smart-entry --format json`,
  `/gsd-code-review --fix`), its bare name with "is not part of @gsd/core", or nothing when the sentence describes a
  mechanism the port does not have;
- an expression is words.

## D8. An optional integration is never probed

No block asks `command -v` or `specforge run --which` whether an integration exists.

- When an extension serves it, the prompt reads `PromptCall::slash_commands`
  ([ADR 0316](0316-a-prompt-reads-the-slash-commands-the-project-serves.md)) and says whether `/gsd-<name>` is served; the step follows that line and skips when it says not served or is absent.
  `plan_phase` does so for `/gsd-graphify`.
- When it is the user's own program, it runs through `specforge run -- <program> …`; exit 127 says it is not
  installed, and nothing installs it.

## D9. A person's chores and files are theirs

- A step only the person performs on their machine (recovery after the host froze, a stale port) is a `text` fence
  introduced by "run these yourself", or prose.
- A file of the user's outside the project's roots (`~/.claude/…`, `~/.gsd/…`) is read with the host's Read tool,
  never by a block: no `specforge files` verb reaches it and a block would pass the ceiling.
- A heuristic about the user's environment (a variable set, a value that is a placeholder) is prose the agent applies.

## Rejected

- Keeping parsers "until the extension lands": dead code the ledger counts as debt.
- Bash builtins as parsers (`${x//…}`, read loops) in place of an answer.
- A CLI that parses a subagent's return.
- Probing PATH for an optional integration, or a setting that turns one on: agents render no settings, and a probe is
  a rule in the shell.
- `${!VAR+x}` for an environment check: zsh has no `${!…}`, and the placeholder test is a judgement, not glue.

## Consequences

- About 230 rows leave the ledger with no new Rust.
- The UI, AI and code-review steps are off with `@gsd/core` alone.
- Worktree isolation is not part of `@gsd/core`: every plan dispatches in sequential mode. An isolation extension, when
  one is built, ports upstream's text afresh from the vendored snapshot.

# `--pick` prints one value of a JSON answer

**Status:** accepted (2026-11-01, GSD follow-ups G4, plan 03). Extends [ADR
0318](0318-a-command-is-told-its-host.md) (a host option added unless a command declares the name). No wire change.

## Context

The gsd prompts read one value of `specforge query … --format json` or of `specforge gsd <command> --format json`
with jq (`jq -r '.next_command'`, `jq -r '.nodes[]|select(.kind=="plan")|.id'`). Git Bash on Windows has no jq, and
a `| jq -r` pipe without it leaves the variable empty. Upstream GSD added `--pick <field>` to its tools for this
(#2589).

## Decision

- **D1. The path.** `step+`, where a step is `.key`, `[n]`, `[]` (every item) or `[kind=<k>]` (the items of an
  array whose `kind` string is `<k>`). Nothing else: no other predicate, no arithmetic, no default. The shell does
  defaults (`${X:-0}`) and counts (`wc -l`). A malformed path is a usage error (exit 2) naming its byte.
- **D2. The output.** One line per value reached: a string bare (a newline in it printed as is), a number or boolean
  as JSON, an object or array as compact JSON, `null` or a missing value as no line. A missing path is not an error;
  the exit code is the command's.
- **D3. `specforge query <id> --pick <path>`** picks the graph format's answer. It takes `--format graph` and its
  alias `json`; `--format context` / `brief`, `--inputs` and `--include-coverage` are a usage error, checked after
  parsing because `--format` has a default.
- **D4. `specforge <ext> <command> … --pick <path>`** is a host option, like `--host`: added unless the command
  declares an arg named `pick`, never sent to the export. It asks for `json`; a writing command's
  `{output, plan, files_written}` document is picked. A refused command prints its error as without `--pick`.
- **D5.** The parser, walker and printer are one CLI module (`crates/surfaces/cli/src/pick.rs`). MCP is unchanged:
  an agent there has the JSON.

## Rejected

A jq subset (`select`, `//`, `length`); a `--field` taking only a top-level key, which leaves graph reads to jq;
adding `pick` to `HOST_OPTIONS`, which every guest compiles.

## Consequences

The gsd prompts read single values and graph reads without jq. A command that declares `pick` keeps it as its own
argument and has no `--pick`.

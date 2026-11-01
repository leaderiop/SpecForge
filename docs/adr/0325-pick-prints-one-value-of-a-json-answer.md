# `--pick` prints one value of a JSON answer

**Status:** accepted (2026-11-01, GSD follow-ups G4, plan 03; amended by plan 05: `check`, `settings` and
`export` take it, D6; the selector takes any key and `.` is the whole answer, D1). Extends [ADR
0318](0318-a-command-is-told-its-host.md) (a host option added unless a command declares the name). No wire change.

## Context

The gsd prompts read one value of `specforge query … --format json` or of `specforge gsd <command> --format json`
with jq (`jq -r '.next_command'`, `jq -r '.nodes[]|select(.kind=="plan")|.id'`). Git Bash on Windows has no jq, and
a `| jq -r` pipe without it leaves the variable empty. Upstream GSD added `--pick <field>` to its tools for this
(#2589).

## Decision

- **D1. The path.** `step+`, where a step is `.key`, `[n]`, `[]` (every item) or `[<key>=<value>]` (the items of
  an array whose string field `<key>` is `<value>`: `[kind=plan]`, `[name=workflow.auto_advance]`,
  `[code=gsd/W205]`; a field that is not a string is not selected). A `.` before `[` may be left out, so jq's
  `.[code=E003]` reads as `[code=E003]`, and `.` alone is the whole answer (one compact line, which a shell
  reads where the answer is pretty-printed JSON). The value runs to the first `]`, so it may hold `.`, `/` and `=`. Nothing
  else: no other predicate, no arithmetic, no default. The shell does defaults (`${X:-0}`) and counts (`wc -l`). `n` is ASCII digits with no leading zero (`0` aside); a key holds no
  whitespace, `]` or `=`. A malformed path is a usage error (exit 2) naming its byte.
- **D2. The output.** One line per value reached: a string bare (a newline in it printed as is), a boolean as JSON,
  an object or array as compact JSON, `null` or a missing value as no line. A missing path is not an error; the exit
  code is the command's.
- **D2a. Numbers.** A number prints as serde_json writes it without `arbitrary_precision` (a workspace-wide
  feature): an integer that fits an i64 or a u64 as is, any other number as an f64, so `1.0` is `1.0`, `-0` is
  `-0.0` and `1e3` is `1000.0`; a number past the f64 range (`1e400`) is refused as not JSON.
- **D3. `specforge query <id> --pick <path>`** picks the graph format's answer. It takes `--format graph` and its
  alias `json`; `--format context` / `brief`, `--inputs` and `--include-coverage` are a usage error, checked after
  parsing because `--format` has a default.
- **D4. `specforge <ext> <command> … --pick <path>`** is a host option, like `--host`: added unless the command
  declares an arg named `pick`, never sent to the export.
  - It implies `--format json`; an explicit other `--format` is a usage error (exit 2). Errors print as they do
    under `json` (the guest's refusal as it wrote it, the host's as a JSON error object), and the exit code is the
    command's, except that an answer that is not JSON fails a command that passed (exit 1).
  - It reads the command's own answer, the JSON its export printed. A writing command's `--format json` output is
    the `{output, plan, files_written}` report only on a run that answered a plan (a run with no ops, or a state
    patch, prints its answer bare), so `--pick` never reads that report: one path reads the same field on every
    run. The plan is still applied, or previewed with `--dry-run`.
  - A command that declares `pick` keeps it, and its `--pick` does not make its usage errors JSON.
  - A prompt (`specforge <ext> <prompt>`) takes no `--pick`.
- **D6. `specforge check`, `settings` and `export`** take `--pick` too, over the answer their JSON form prints:
  `check`'s bare array of diagnostics (`check --pick '.[code=gsd/W205].code'`), the `settings` listing
  (`settings --pick '.extensions[].settings[name=<setting>].value'`; an unset setting has no `value`, so it prints
  nothing), and `export`'s graph format (`--format graph` or `json`; `context`, `brief`, `dot` and `markdown` are a
  usage error, as on `query`). On `check` and `settings`, `--pick` makes `json` the default format and an explicit
  `--format human` is a usage error (exit 2); errors print as under `json`, and the exit code is the command's
  (`check` still exits 1 on an error it reports). `settings set` and `unset` take no `--pick`.
- **D5.** The parser, walker and printer are one CLI module (`crates/surfaces/cli/src/pick.rs`). MCP is unchanged:
  an agent there has the JSON.

## Rejected

A jq subset (`select`, `//`, `length`); a `--field` taking only a top-level key, which leaves graph reads to jq;
adding `pick` to `HOST_OPTIONS`, which every guest compiles.

## Consequences

The gsd prompts read single values and graph reads without jq. A command that declares `pick` keeps it as its own
argument and has no `--pick`.

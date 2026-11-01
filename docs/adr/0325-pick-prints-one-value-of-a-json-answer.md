# `--pick` prints one value of a JSON answer

**Status:** accepted (2026-11-01, GSD follow-ups G4, plan 03; amended by plan 05: `check`, `settings` and
`export` take it, D6; the selector takes any key and `.` is the whole answer, D1; a dotted key and a scalar that is
not a string, D1a; `--fields` rows on `query` and `export`, D7). Extends [ADR
0318](0318-a-command-is-told-its-host.md) (a host option added unless a command declares the name). No wire change.

## Context

The gsd prompts read one value of `specforge query … --format json` or of `specforge gsd <command> --format json`
with jq (`jq -r '.next_command'`, `jq -r '.nodes[]|select(.kind=="plan")|.id'`). Git Bash on Windows has no jq, and
a `| jq -r` pipe without it leaves the variable empty. Upstream GSD added `--pick <field>` to its tools for this
(#2589).

## Decision

- **D1. The path.** `step+`, where a step is `.key`, `[n]`, `[]` (every item) or `[<key>=<value>]` (the items of
  an array whose string field `<key>` is `<value>`: `[kind=plan]`, `[name=workflow.auto_advance]`,
  `[code=gsd/W205]`). A `.` before `[` may be left out, so jq's
  `.[code=E003]` reads as `[code=E003]`, and `.` alone is the whole answer (one compact line, which a shell
  reads where the answer is pretty-printed JSON). The value runs to the first `]`, so it may hold `.`, `/` and `=`. Nothing
  else: no other predicate, no arithmetic, no default. The shell does defaults (`${X:-0}`) and counts (`wc -l`). `n` is ASCII digits with no leading zero (`0` aside); a key holds no
  whitespace, `]` or `=`. A malformed path is a usage error (exit 2) naming its byte.
- **D1a. The selector's key and value.** The key may be dotted, a path through objects: `[fields.result=issue]`
  selects the nodes whose `fields.result` is `issue`, so a graph read filters by an entity field in one pick instead of
  one query per entity. A dotted key has a name between its dots. A string field is compared by its text; a number,
  boolean or `null` as JSON: the selector's value is read as JSON and compared (`[fields.backlog=true]`,
  `[fields.wave=2]`; the integer `2` is not the float `2.0`);
  an object or array, or a missing field, is never selected.
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
- **D7. `--fields <keys>`** beside `--pick` on `query` and `export` prints one row per item the path reaches, its
  cells the values at the comma-separated keys (each may be dotted), joined by the ASCII unit separator (0x1f): two
  values of one entity on one line, where two picks printed side by side would misalign once a value is missing. A
  cell prints as `--pick` prints a value, a missing value or `null` is an empty cell, and an item that is `null` still
  holds a row. A backslash, newline, carriage return or 0x1f in a cell prints as `\\`, `\n`, `\r` or `\x1f`, so a
  row is one line. The separator is 0x1f, not a tab, because a tab is IFS whitespace: `IFS=$'\t' read -r a b`
  collapses an empty first cell and shifts the rest left, while `IFS=$'\x1f' read -r a b` keeps it, under bash 3.2
  as under bash 5 (plan 05 re-review). A shell reads a row with `IFS=$'\x1f' read -r`; a person sees it with
  `tr '\037' '\t'`.
  `--fields` without `--pick` is a usage error (exit 2), and so is an empty key. Commands whose items are not
  entities (`check`, `settings`, an extension command) do not take it: no prompt reads two of their values at once.
- **D5.** The parser, walker and printer are one CLI module (`crates/surfaces/cli/src/pick.rs`). MCP is unchanged:
  an agent there has the JSON.

## Rejected

A jq subset (`select`, `//`, `length`); a `--field` taking only a top-level key, which leaves graph reads to jq;
adding `pick` to `HOST_OPTIONS`, which every guest compiles.

## Consequences

The gsd prompts read single values and graph reads without jq. A command that declares `pick` keeps it as its own
argument and has no `--pick`.

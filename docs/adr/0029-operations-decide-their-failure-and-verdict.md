# Operations decide their failure and their verdict; a command ends in one place

**Status:** accepted (2026-10-07). Amends ADR 0015 (I1, D8), ADR 0021 (D4) and ADR 0011 (the
stdout/stderr bullet).

ADR 0024 D7 said operations fail with a kind decided where the failure is raised. The read views that
read the recorded test report did not: they returned the project crate's `ReportError`, and MCP (a
classifier at eight call sites), analyze (`From<Diagnostic>`: always schema mismatch) and plan (the
same) each decided what it was. The same report the OS refused to read was `internal_error` from
`specforge.stats`, `schema_mismatch` from `specforge.analyze`, never `permission_denied`; a missing
`test_results` was a schema mismatch, and a relative one was read from the server's working directory.
Format and migrate had no verdict in ops: the CLI restated ADR 0021 D4's rule and migrate's, MCP
returned none and chose migrate's failure kind itself. The CLI printed an operation's failure in seven
shapes and five commands ignored `--format json` when they refused. MCP refused to format or migrate a
directory its own call target had resolved, and format, migrate and the call target each wrote "the
nearest project, else the directory itself" on their own.

## Decisions

- **D1. A report failure is classified once, in ops.** `specforge_project::coverage::ReportError` says
  what happened (unreadable with the OS error's kind, or malformed; invalid UTF-8 is malformed);
  `specforge_ops::report::unusable` turns it into the `OpError` every view returns: code E045,
  `schema_mismatch` when malformed, else the OS error's kind (`OpErrorKind::of_io_kind`:
  `permission_denied`, `file_not_found` for a named file that does not exist, else `internal_error`),
  with a suggestion for that kind. No `ReportError` crosses the ops interface; `EntityFacts::coverage`
  is `Result<EntityCoverage, OpError>` (amends ADR 0015 I1).
- **D2. Typed operation errors convert through one `From` in ops.** `CheckError`, `TraceError`,
  `PlanError` and `AnalyzeError` stay where a surface names the argument a variant is about (the
  spelling of an argument is a surface's: `severity_filter` against `--severity`); their kind and code
  are their `From<…> for OpError`, which every surface goes through. `AnalyzeError` gains it
  (`unknown_pass` with a did-you-mean, the classified E045, `no_test_results`).
- **D3. Format and migrate carry their verdict.** `format::Outcome::ok()` (with `Mode::Preview` for a
  diff that does not fail), `migrate::Outcome::ok()` and `failure()`; `OpErrorKind::CompilationFailed`
  makes the map to MCP's error codes total, so MCP decides no kind. MCP returns `ok` in the result (and
  in a failed migration's `data`) and keeps `isError` for refusals and failed runs as before (ADR 0004
  D4-a: a run that finds something is a successful call).
- **D4. One exit table**: 0 the run passed; 1 its verdict failed or its operation refused; 2 it could
  not judge (a refused command line, or a refusal of `stats` or `analyze`). Rejected: exit by kind,
  which contradicts `init` and `add`'s specified exit 1 for invalid input.
- **D5. One refusal shape**: `error[CODE]: message`, `  hint: …`, `  wrote: …` on stderr, or the error
  document on stdout under `--format json` (`analyze --json`; ADR 0011 keeps core errors on stdout), for
  every core command (`specforge_cli::outcome::Refusal`). A command run outside a project it needs
  refuses with `no_project`.
- **D6. One rule for the project a path is in** (`specforge_common::project_root_of`): format and
  migrate act on it on both surfaces, a directory that is no project included; migrate compiles the
  root it migrates, so a migration started from a sub-path runs the project's hooks.
- **D7.** Export's schema-cache protocol is `ops::export::export_recorded` (as ADR 0018 D1 for check);
  MCP still never records (ADR 0015 D10). An extension command's date is `CommandContext::now`. MCP's
  no-project refusal reads `McpError::file`, never its message; `Reach::takes_path` is the one predicate
  for an entry that takes a `path`.
- **D8. MCP resolves a relative `test_results` against the call's project root**, as the paths of
  `specforge.format`; the CLI keeps shell semantics (relative to the working directory).

## Consequences

- MCP: a locked report is `permission_denied` on every tool that reads it; a missing `test_results` is
  `file_not_found`; a relative `test_results` is read under the project root; `specforge.format` and
  `specforge.migrate` return `ok` and act on a directory that is no project; the outline's
  missing-file refusal carries `file`.
- CLI: `stats`, `trace`, `analyze --json`, `migrate` and `init` print the error document under JSON
  output; `migrate`, `export`, `schema`, `init`, `analyze` and `check` refusals take the one shape
  (`= help:` becomes `  hint:`); `collect` outside a project is `no_project`, not E045;
  `migrate --path <sub>` runs the project's hooks. Exit codes are unchanged.
- The parity harness compares verdicts: the CLI's exit code and MCP's `ok` cannot disagree for check,
  analyze, format and migrate.

## What would reopen it

A surface that needs a failure kind ops cannot know (a transport failure), or a command whose exit
code must say more than passed, failed or unjudged.

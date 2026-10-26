# A held path keeps the caller's spelling

**Status:** accepted (2026-10-26, architecture round 28, plan 09). Reopens [ADR 0112](0112-every-project-file-an-operation-opens-goes-through-the-requests-containment.md)
D1's naming clause ("name `spelled`"). Amends [ADR 0104](0104-containment-is-applied-where-the-operation-opens-the-file.md) D2,
[ADR 0102](0102-a-document-names-a-project-file-from-the-project-root.md) D6, [ADR 0171](0171-a-migration-writes-through-the-project-change.md)
and [ADR 0253](0253-a-source-code-path-a-request-names-has-one-reader.md) D3 (applied to every held path). Reopens
[ADR 0105](0105-one-reader-for-the-extensions-a-change-names.md) D4 (the base).

## Context

- A `Held` carried one spelling, `spelled`, for three jobs: "as the caller gave it" and "joined onto the root" are two
  names. Every message over MCP quoted a name the caller never wrote (`cannot read test results /…/p2/nope.json` where the
  CLI says `nope.json`; `path not found: <root>/nope`; `no extension binary at <root>/nope.wasm`).
- Callers pre-joined a request's name before holding it (`names.rs`), held a held name twice (`candidate.rs`), rebuilt the
  refusal by hand (`export.rs`, `call.rs`), or quoted the request beside the hold (`format/run.rs`): five one-site commits
  over five rounds (`548b6d2df`, `1d2e7f788`, `c0775a2f1`, …).
- Three relativizers turned a held path back into a document's name, each its own way: the lock's `shown_path`
  canonicalized the file (a link was recorded by its target), the collector's report path was a lexical `strip_prefix`
  (`./sub/r.json`, `sub/r.json` or `r.json` for one file), a project change's failure another.
- `specforge add` read a relative local path from the working directory; MCP and init from the project root. The path the
  lock records and the reinstall command it suggests (`specforge add ./ext/greet.wasm`) is a project-root path: the hint ran
  only from the root.
- A named test report that is a directory, or the empty name, was `internal_error` with no argument.
- ADRs 0112 and 0104 say a message names `spelled`; ADR 0253 says it quotes the caller's spelling. `spelled` was not the
  caller's spelling when bounded.

```text
BEFORE  name ──(base.join)──▶ resolve ──▶ Held { spelled = root.join(name), real }
        message ◀ spelled   owned name ◀ spelled   lock ◀ shown_path   collector ◀ strip_prefix   change ◀ strip_prefix
AFTER   name ──▶ resolve | resolve_from(base, name) ──▶ Held { named, at, real }
        message ◀ named   owned name / argv ◀ at   document / record ◀ shown(root)   file under a dir ◀ entry
```

## Decisions

- **D1.** `Held { named, at, real }`, one use each (U1=a). `named`: as written, every message quotes it. `at`: `named` read
  against its base, links not followed: an owned name is created and unlinked here (ADR 0171), a runner is handed it, and
  `shown` names it. `real`: where it lands, what is opened. `spelled` is gone.
- **D2.** Nothing joins a request's name before holding it, nothing holds it twice: `Containment::resolve_from(base, named)`
  reads a name against a base, `Held::entry(containment, name)` holds a file under a held directory (named
  `<named>/<name>`), `Containment::read_held(&held)` reads what was held. An `Outside` quotes the name the caller passed.
- **D3.** `Held::shown(root)` is the one project path of a held file in a document or record: `ProjectPath::of(root, at)`
  (ADR 0102: a link by its own name, `/`-joined, absolute outside the root). The lock's `local:` source, a collector's
  `reports[].path`, a project change's failure, the export's and migrate's paths use it (U3=a). Existing locks stay
  readable; the next add rewrites a link's entry.
- **D4.** A named test report that is a directory, or the empty name, is E045 `invalid_input` about its argument
  (`'<named>' is a directory: test results are a specforge-report.json file`; `an empty test_results names no file`). The
  recorded report that is a directory stays `Internal`: there is no argument to blame.
- **D5.** `specforge add` reads a relative local path from the project root on both surfaces (U2=a), as init reads it from
  the directory it creates, so the path the lock records and its reinstall command names is the path add reads, from any
  directory of the project. `AddRequest.base`, `Place.base` and the CLI's working directory are gone.

## Rejected

- Messages print the project path everywhere (U1=b): the CLI loses "as typed" (ADR 0102 D4), and an unbounded containment
  has no root to show from.
- Keeping `spelled` and fixing sites one by one (U1=c): the pattern of the five commits above.
- Keeping the cwd as add's base and saying where to run the hint (U2=b) or leaving it (U2=c).
- The canonical target in the lock (U3=b): it disagrees with every other document field and with what was typed.
- `shown` as a field: an unbounded containment has no root.
- Keeping `spelled` beside a new `named`: two words for "as given".

## Consequences

- MCP text: `analyze` `test_results`, `add_extension` and `init` local paths, every call target's `path not found`, init's
  conflict name the path as written. MCP and CLI code: analyze of a directory or `""` is `invalid_input` on `test_results`,
  not `internal_error`.
- CLI: `specforge add ../x.wasm` run in a subdirectory reads `<root>/../x.wasm`.
- On disk: the lock's `local:` source for a link records the link; `/` between components on Windows (unverified).
- Protocol value: a collector's `reports[].path` is the project path (the type is unchanged).
- Rust (`specforge-ops`): `Held.spelled` → `Held.at` + `Held.named`; `resolve_from`, `read_held`, `Held::{entry, shown}`;
  `LocalComponent.held`, `LocalFile.held`; `AddRequest.base` gone. `shown_path`, the export's rebuilt refusal and three
  `strip_prefix` relativizers go.
- Left alone: names an operation builds itself (`root.join(REPORT_FILE)`, the infer manifest, a declared collector report
  location) keep the joined path in their messages, alike on both surfaces.

## What would reopen it

A request that must name a file from somewhere a containment cannot express as a base, or an `openat`-style handle that
replaces `real`.

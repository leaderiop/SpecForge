# A project file has one spelling, from the project root

**Status:** accepted (2026-10-23, architecture round 26, plan 13). Reopens [ADR 0102](0102-a-document-names-a-project-file-from-the-project-root.md) U2 and amends its U2, U4 and D4 and its rule; amends [ADR 0055](0055-imports-name-sources.md) D1, [ADR 0195](0195-a-spec-file-a-request-names-is-read-into-its-source-key-once.md) D2, D3, D5, D8 and "Left alone", [ADR 0056](0056-an-open-document-is-its-files-session-key.md) D3, [ADR 0184](0184-a-typed-edit-and-a-rename-write-one-planned-change.md) D1, [ADR 0014](0014-the-session-keeps-itself-current.md) ("keyed relative to it") and [ADR 0021](0021-formatting-is-one-operation.md) D7.

## Context

A file had two spellings. A declaration's location (a span, a diagnostic, an export node) named it from the spec
root; every document an operation answered (`files_written`, `affected_files`, format's found files) named it from the
project root. A request naming a spec file read it from the spec root, so the same file was `hello.spec` in one
reply and `spec/hello.spec` in the next, and a wrong spelling got a "did you mean" hint for the other base.

```text
BEFORE  span / key: hello.spec        document / format: spec/hello.spec     hints between the two
AFTER   one spelling: spec/hello.spec, everywhere
```

## Decision

- **D1. A source's key is its project path** (`ProjectPath::key`), minted only by `SourceRoot::key` and opened only
  by `SourceRoot::path`; `/` between components on every OS.
- **D2. The spec root keeps discovery**, `exclude`, nested projects, watchers and import paths. `spec_dir` is
  `SourceRoot::prefix`; the resolver still reads nothing.
- **D3. A key outside the root** (absolute, its own name) and a name that is no path (`untitled:`) stay as they were.
- **D4. A request names a spec file from the project root** on both surfaces: no other reading, no hint.
- **D5. `format` refuses a named path that names nothing** about `paths`, before anything is read; found files are
  named from the root in human output; `--stdin` refuses the paths, `--check` and `--diff` it ignored.
- **D6. Spans keep an interned `Sym`;** containers keep the text.
- **D7. No Graph Protocol bump.**
- **D8. `file_exists` paths are unchanged.**

## Rejected

Two bases with symmetric hints (the first revision); aliases; import paths from the project root;
`SourceSpan.file: ProjectPath`; `--in` read from the cwd; a per-file failure with a hint.

## Consequences

Breaking for every surface that printed a spec-root key (see plan 13 T7); no file on disk changes meaning; no
re-vendor. Reopen it if a request argument must name a file from somewhere else, or if spec-text paths
(`file_exists`) are wanted from the root.

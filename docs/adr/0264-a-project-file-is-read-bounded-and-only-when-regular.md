# A project file is read bounded and only when it is regular

**Status:** accepted (2026-10-26, architecture round 28, plan 01). Extends [ADR 0107](0107-the-checks-read-through-the-disk-they-are-given.md)
(the disk's read reads only a regular file, at most 64 MiB) and [ADR 0146](0146-a-file-with-more-than-one-name-is-not-read-through-a-bound.md) D4
(`specforge.json` and the lock, read through the plain environment disk, are read like every project file). Amends
[ADR 0250](0250-a-download-is-held-to-the-size-its-reply-declares.md) D4 (the ceiling is `specforge_package::MAX_PACKAGE_BYTES`).

## Context

- Twelve sites read a project file with `std::fs::read`, `read_to_string` or `read_to_end`: `PlainDisk::read`, the
  containment's four `Held` reads, `read_project_config`, `read_lock_file`, the project change's four snapshots and the
  language server's comparison. None looked at the file type or the size.
- A link to `/dev/zero` named `specforge.lock`, a module or a `.wasm` file entry was read until memory ran out (3.43 GB in
  2 s). A named pipe hung `check`, `doctor` and MCP `initialize` for ever and left the language server indexing for ever.
  A 200 MiB lock was read whole (245 MB) before anything judged it.
- The network was already bounded (ADR 0250: 64 MiB); the disk was not.

```text
BEFORE  PlainDisk, Held x4, config, lock, change x4, LSP ──▶ std::fs::read   any type, any size
AFTER   all twelve ──▶ specforge_common::regular: stat ▶ judge ▶ open ▶ fstat ▶ judge ▶ read at most 64 MiB + 1
```

## Decisions

- **D1.** One reader, `specforge_common::regular` (`open`, `read_from`, `read`, `read_to_string`), the read-side twin of
  `fresh` (ADR 0144). All twelve sites read through it.
- **D2.** The file is judged before it is opened (opening a pipe waits for a writer), the handle is judged again, and at
  most `MAX_FILE_BYTES + 1` bytes are read, so a file that grew or reports a smaller length is still cut. The window
  between the stat and the open is a local race only (a cloned repository carries no pipe); it is marked `shortcut:`.
- **D3.** One ceiling, 64 MiB, for every project file: `specforge_package::MAX_PACKAGE_BYTES`, moved there from
  `specforge-registry-wire` so the reader can name it (no new dependency edge, no blob input).
- **D4.** A link is followed wherever it lands (as the spec walk does); what a link out may reach is not closed here.
- **D5.** A `.wasm` file entry may name any path, as documented for iterating on an extension; it is now bounded and
  regular.
- **D6.** Error kinds keep their meaning: `NotFound`, `IsADirectory`, `PermissionDenied` as before; over the ceiling is
  `FileTooLarge`; another type is an error naming it. Never `InvalidInput`, which stays the bound's. No new diagnostic
  code: E025, E028, E033, E069 and `config_invalid` carry the sentence.
- **D7.** The spec walk is unchanged: a named pipe in the spec root stays a source and is E025 at its read.

## Rejected

- A second 1 MiB ceiling for the lock and the config: bounds no memory the 64 MiB read does not.
- No ceiling, only the type check: leaves a link to a large file read whole.
- Refusing links that leave the project, or `.wasm` entries outside it: breaks documented workflows and protects nothing
  once the read is bounded (no content reaches a message).
- A rule in `PlainDisk` alone: six of the twelve sites do not read through the port.
- A `Disk::read` limit argument: changes six adapters and eleven callers for no gain.
- `O_NONBLOCK` through a new `libc` dependency: the workspace has none.

## Consequences

- The completion test that held indexing with a named pipe depended on the defect and is retired; the order it proved
  (the environment is shown before any source is read) stays pinned in `reopen.rs`.
- A directory reads "it is a directory, not a regular file" where it read the OS text.
- A `.spec` over 64 MiB is E025. The language server publishes no diagnostic for an unopened source, so its pin asserts
  that indexing ends over a pipe, not an E025 publication.

## What would reopen it

A legitimate project file over 64 MiB; a content leak through a link out of the project; a file system whose regular
files block.

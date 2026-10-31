# A pass or command declares the project files it reads, and its input carries their facts and capped text

**Status:** accepted (2026-10-28, kernel round 35, plan K11). Extends
[ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md)
(the guest has no file system: the host reads and hands over),
[ADR 0107](0107-the-checks-read-through-the-disk-they-are-given.md)
(a check's reads are its inputs), [ADR 0287](0287-who-writes-a-field-is-declared.md) (compute and check passes),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (the file digest a judged rule and a verdict
already use) and [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (the hook globs and
`PathFact`). Takes extension protocol 1.11.0. Extended by [ADR 0300](0300-a-compute-pass-at-the-baseline-reads-the-files-the-commit-holds.md) (a compute pass at the baseline reads the commit's files). Extended by [ADR 0304](0304-a-judged-input-reads-a-path-field-goes-on-past-a-reference-and-a-pass-sees-the-tests-a-verdict-cites.md) (a `file()` input may end at a string path).

## Context

A workflow model asks questions a graph cannot answer: does the file a plan says it will change exist, does it hold a
debt marker, how many lines is it, does a key link's pattern occur in it. `file_exists` is a declarative rule over one
field's text and fires for every path, including a file the plan has yet to create. A pass has the entities, the edges
and the test results; a command has the graph and the evidence; neither has a file. A guest cannot read one (ADR 0037),
and a hook alone is told the facts of the paths its tool names (`PathFact`, ADR 0292).

## Decision

- **D1. A pass or command declares `reads`.** `CompilerPassDescriptor.reads` and `CommandDescriptor.reads` are lists of
  `FileRead {name, field?, glob?, base?, text?}`; absent and empty are the same and are not written. `name` is a word,
  unique among the owner's reads, and keys `files` in the input. Exactly one of `field` (`kind.field` or
  `kind.field.member`: the paths the entities of that kind write there, read as `file_exists` reads them: a scalar's text
  or each item of a list) or `glob` (project-relative, `/` separated, the hook glob of ADR 0292: `*`, `?`, a `**`
  segment). `base` (`spec` or `project`, default `project`) applies to a field read only: a path in a spec names a project
  file, and a glob is always from the project root. `text` (default false) asks for the capped text as well as the facts.
  A malformed read (neither or both of `field` and `glob`, a name that is no word or repeats, a field no loaded extension
  declares, a glob that is absolute, climbs with `..`, has an empty segment or has more than four `**` segments (the matcher tries every split of the path at each one), `base` on a glob) is W021 from the registry build
  (W145 is the pass-order cycle), and that read is served empty; the pass is kept, because a dropped check is worse than an empty read.
- **D2. The input carries `files`.** `PassInput.files` and `CommandInput.files` map a read's name to `FileFacts {facts,
  truncated?}`; a `FileFact {path, entity?, exists, bytes?, lines?, digest?, text?, error?}` is one per (entity, path),
  sorted by `(entity, path)` (a glob: by `path`). `path` is project-relative, `/` separated and lexically normalised;
  `entity` is the full path of the entity whose field named it (a field read only). `exists` follows links. `bytes`,
  `lines` (the `\n` count plus one for a last line without one) and `digest` (`sha256:` and the hex of the CRLF-normalised
  bytes, the digest judged rules and verdicts already use) are present for a regular file that was read. `text` is the
  file as UTF-8, only when the read asks for it and the file is within the cap. `error` says why text or facts are
  missing: `too_large`, `not_utf8`, `unreadable`, `outside_project`, `denied`, `budget`; a path that does not exist is
  `exists: false`, not an error. `truncated` is set on a read whose glob matched more than the file cap. The field is new
  and is not `PathFact`, whose `in_project`, `in_spec_root` and `new_bytes` mean nothing outside a hook.
- **D3. Containment is lexical, then real.** A path is joined to its base, normalised lexically and refused as
  `outside_project` when it is absolute (even one that lies under the root: a path in a spec is project-relative) or
  climbs out with `..`. A path that passes is then resolved: through a disk that has links (`Disk::canonical`, the plain
  file system and a session's recording disk over it), a file whose canonical path leaves the canonical project root is
  `outside_project` too, `exists: false`, with no facts or text, for a field read and a glob alike, and for a command,
  `analyze`, a guard and a check pass alike (all read through the same `file_reads`). A glob does not descend into a
  directory reached through a link, so a link into an ancestor cannot loop and a link out cannot be walked. The MCP
  contained disk (ADR 0145) is unchanged and holds its own files. Residual limits: the check and the read are two calls,
  so a link swapped between them is read (the contained disk closes that by holding the file; a recording disk cannot);
  and a link to a directory inside the project is not walked, so a glob does not list files through it.
- **D3a. A declared read is a capability, so it is listed and it has a deny-list.** A read hands a guest the text of
  project files, which is more than the graph holds (`.env`, a key, `.git/config`), so (a) `specforge extensions` and
  `specforge doctor` list every read an extension declares: its pass or command, its name, the field or glob, and
  whether text is requested (`specforge add` prints no extension detail: the declaration is read at the next load, where
  these two show it); (b) a read that asks for text withholds the text of a path that looks like a credential: `.env`,
  `.env.*`, `*.pem`, `*.key`, `id_rsa*`, `id_ed25519*`, `.npmrc`, `.netrc`, `credentials*` and `.git/config`, by the
  name as written and by the name a link resolves to. The fact has `exists` and `error: "denied"` and no `text`,
  `digest`, `bytes` or `lines`; the path is still a check input, so changing it re-runs the pass. A read that does not
  ask for text is not denied (facts only). Limit: the list is fixed, by name, and no project can opt a file in; a
  project that needs to will get a setting in a later round. An unlisted file (a token in `config.json`) is still
  readable by a declared read, which is why the listing in (a) exists.
- **D4. Caps.** A file's text is capped at 64 KiB; the disk already refuses a file over 64 MiB, which is `too_large`.
  A read sends at most 1 MiB of text in all (`budget` after that) and 2000 files (`truncated`). A glob never enters
  `.git`, `.specforge`, `.svn`, `.next`, `.venv`, `dist`, `node_modules`, `target` or `vendor` unless a literal segment of
  the pattern names it, and visits at most 20,000 entries (files and directories, matched or not) before the read is
  `truncated`. No diagnostic is
  raised for a cap: the pass sees `error` and `truncated` and reports with a code of its own.
- **D5. A read file is a check input.** The host reads every declared file through the disk the compile was given. A
  session's recording disk stamps each `read`, `exists` and listing as a check input, so an edit to a read file, the
  creation of an absent one or a new file under a globbed directory re-runs the checks, as `file_exists` already does.
  A walk lists a directory it enters, and a plain file it passes is not an input of the walk: editing a file no read
  names or matches does not re-run the passes. A command and an analyze pass read through the plain disk: they run once.
- **D6. Hooks keep `PathFact`.** A hook is handed the facts of the paths its tool names, which is another question
  (what is about to be written); it is unchanged.

## Limits

The test report a `reads_tests` pass receives is the one recorded now. The baseline view (`previous`) is computed from
the commit's files without it (`tests: None`), so a computed field derived from tests is unknown in `previous` and known
in the current view; a compute pass that compares the two must not derive from tests. Reading the commit's own
`specforge-report.json` is the upgrade when a pass needs it. Nothing in r35 declares such a pass.
The commit's files are read from its tree and fetched on demand (ADR 0300).

## Rejected

A host function the guest calls (ADR 0037: no capability crosses the sandbox, and a call per file is a round trip the
host cannot cache). Reading in the guest with a preopened directory (the same, and then the guest decides what the check
depends on). One list of strings for field paths and globs (a string cannot say `text`, and the declaration check could
not tell a field from a glob). A bare `sha256` key beside the `sha256:` prefix every other digest wears.

## Consequences

Two descriptor fields, one input field on two inputs and one new type; a pass that declares nothing is sent the bytes it
was sent. Every re-check re-reads and re-hashes every declared file, bounded by the caps and by who declares what: the `lines`
fact needs the bytes, so a stamp-keyed digest would not save the read, and there is no memo across runs (the upgrade when
`doctor`, which lists each extension's declared reads, shows reads that are slow). Passes of one compile that declare
equal reads share one walk. A glob lists every non-matching entry under a `**`, within the visit budget. Nothing in this round declares a read: the
`@gsd/core` extension does.

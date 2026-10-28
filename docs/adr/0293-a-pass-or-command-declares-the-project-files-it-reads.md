# A pass or command declares the project files it reads, and its input carries their facts and capped text

**Status:** accepted (2026-10-28, kernel round 35, plan K11). Extends [ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md)
(the guest has no file system: the host reads and hands over), [ADR 0107](0107-the-checks-read-through-the-disk-they-are-given.md)
(a check's reads are its inputs), [ADR 0287](0287-who-writes-a-field-is-declared.md) (compute and check passes),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (the file digest a judged rule and a verdict
already use) and [ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md) (the hook globs and
`PathFact`). Takes extension protocol 1.11.0.

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
  declares, a glob that is absolute, climbs with `..` or does not parse, `base` on a glob) is W021 from the registry build
  (W145 is the pass-order cycle), and that read is served empty; the pass is kept, because a dropped check is worse than an empty read.
- **D2. The input carries `files`.** `PassInput.files` and `CommandInput.files` map a read's name to `FileFacts {facts,
  truncated?}`; a `FileFact {path, entity?, exists, bytes?, lines?, digest?, text?, error?}` is one per (entity, path),
  sorted by `(entity, path)` (a glob: by `path`). `path` is project-relative, `/` separated and lexically normalised;
  `entity` is the full path of the entity whose field named it (a field read only). `exists` follows links. `bytes`,
  `lines` (the `\n` count plus one for a last line without one) and `digest` (`sha256:` and the hex of the CRLF-normalised
  bytes, the digest judged rules and verdicts already use) are present for a regular file that was read. `text` is the
  file as UTF-8, only when the read asks for it and the file is within the cap. `error` says why text or facts are
  missing: `too_large`, `not_utf8`, `unreadable`, `outside_project`, `budget`; a path that does not exist is
  `exists: false`, not an error. `truncated` is set on a read whose glob matched more than the file cap. The field is new
  and is not `PathFact`, whose `in_project`, `in_spec_root` and `new_bytes` mean nothing outside a hook.
- **D3. Containment is lexical.** A path is joined to its base, normalised lexically and refused as `outside_project`
  when it is absolute or climbs out with `..`. A symlink to outside is followed by the disk (the ledger records each hop);
  the lexical check cannot see it because the disk port has no `canonicalize`. This is the limit of `file_exists` and of
  the judged `file(...)` input, accepted here: the sandbox is "no capability", the host reads, and the guest is given
  text it could have been handed by `specforge export`.
- **D4. Caps.** A file's text is capped at 64 KiB; the disk already refuses a file over 64 MiB, which is `too_large`.
  A read sends at most 1 MiB of text in all (`budget` after that) and 2000 files (`truncated`). A glob never enters
  `.git`, `.specforge`, `node_modules` or `target` unless a literal segment of the pattern names it. No diagnostic is
  raised for a cap: the pass sees `error` and `truncated` and reports with a code of its own.
- **D5. A read file is a check input.** The host reads every declared file through the disk the compile was given. A
  session's recording disk stamps each `read`, `exists` and listing as a check input, so an edit to a read file, the
  creation of an absent one or a new file under a globbed directory re-runs the checks, as `file_exists` already does.
  A command and an analyze pass read through the plain disk: they run once.
- **D6. Hooks keep `PathFact`.** A hook is handed the facts of the paths its tool names, which is another question
  (what is about to be written); it is unchanged.

## Rejected

A host function the guest calls (ADR 0037: no capability crosses the sandbox, and a call per file is a round trip the
host cannot cache). Reading in the guest with a preopened directory (the same, and then the guest decides what the check
depends on). One list of strings for field paths and globs (a string cannot say `text`, and the declaration check could
not tell a field from a glob). A bare `sha256` key beside the `sha256:` prefix every other digest wears.

## Consequences

Two descriptor fields, one input field on two inputs and one new type; a pass that declares nothing is sent the bytes it
was sent. Every re-check re-reads every declared file, bounded by the caps and by who declares what; no memo yet (a
stamp-keyed memo in the digest memo is the upgrade when `doctor`, which now counts declared reads per extension, shows
reads that are slow). A glob lists every non-matching entry under a `**`. Nothing in this round declares a read: the
`@gsd/core` extension does.

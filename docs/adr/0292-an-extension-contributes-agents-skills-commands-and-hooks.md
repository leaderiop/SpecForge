# An extension contributes agents, skills, slash commands and hooks, rendered into each host by install

**Status:** accepted (2026-10-28, architecture round 33, plan K9). Extends [ADR 0010](0010-operations-reach-the-registry-through-a-port.md)
(a second port, the host renderer), [ADR 0012](0012-one-extension-declaration.md) (four surface item kinds),
[ADR 0013](0013-typed-extension-calls.md) (two typed calls), [ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md)
(a hook gets facts, never a capability), [ADR 0084](0084-a-crates-layer-is-its-folder.md) (the adapters layer gains
`specforge-hosts`), [ADR 0097](0097-an-extension-commands-name-is-judged-by-its-route.md) (host names are one
namespace), [ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (a hook's plan, the locked
journaled write) and [ADR 0291](0291-an-extension-contributes-mcp-prompts.md) (a slash command is the host binding of a
prompt). Takes extension protocol 1.10.0, shared with K8.

## Context

A workflow reaches an agent through its host: a Markdown agent, a skill, a slash command, a hook entry and the MCP
server's entry in the host's project files. Those files were written by hand or by a node installer that copied one
vendor's text, so they drifted from the project's extensions, from their settings and from each other. Guards that must
run before a write ran as one script per host. An agent could write a `.spec` file with the host's own Write tool and set
a value that only a command computes or only a person writes (ADR 0287, ADR 0289).

## Decision

- **D1. An extension declares four kinds in its surfaces**: `agents`, `skills`, `slash_commands` and `hooks`. Metadata
  is declared; an agent's or skill's body is written by one `render__` export, called only by install, with the host,
  the variant and the settings in hand (describe runs on every load, and a body may be a megabyte). A slash command has
  no body of its own: it is the host binding of one of the extension's prompts or commands, named `<short>-<route>`, a
  trampoline to that target's own surface, so the workflow has one source and the host holds no stale copy. Tools are
  host-neutral classes (`read`, `write`, `edit`, `shell`, `search`, …, `mcp:<server>/<glob>`) that each adapter maps;
  a model is a tier or a `setting:` naming one. The registry fails closed as for prompts: a contribution it cannot serve
  is W021 and dropped, and a host name claimed twice is E039.
- **D2. A host is a file format, never a domain.** `specforge install <host>` renders the contributions through a
  port, `HostRenderer`, that `specforge-hosts` implements (adapters layer) for Claude Code, Codex and Cursor. The adapters
  know layouts, config merges and hook protocols, and no word of any extension. Install writes project-local files only
  (`.claude/`, `.codex/`, `.cursor/`, `.mcp.json`): the contributions come from the project's lock, so a global install
  would serve one project's versions to every project. What a host cannot take is reported `not_rendered`, never written.
  The Claude Code layout and protocol follow its documentation; the Codex and Cursor ones could not be verified when this
  was written and are documented as such.
- **D3. Install owns exactly what it marked.** A file it writes carries a marker line with its host, source and the
  digest of the rest; a config entry it owns is the one whose command is `specforge` or starts `specforge hook `. An
  unmarked file is never written, moved or removed (`foreign_file`); a marked file edited since is refused
  (`edited_file`) unless forced. The plan is compared with the disk file by file and entry by entry, refusals are
  reported together and nothing is written, and the write is one change under the write lock and the journal (ADR 0289):
  idempotent, all or nothing, uninstall the same plan against a tree that renders nothing. `doctor` reports a marked
  file edited since and an install stale against the project.
- **D4. A hook is a function called on a host event.** Each hook is one host entry that runs
  `specforge hook <short>.<id> --host … --event …`, whose flags repeat the declaration so a non-matching event exits
  before anything is loaded. The sandbox grants no file system, so the host computes the facts a guard needs (paths with
  their size and place in the spec root, a shell command's words, a commit's message) and sends the graph and the state
  only when declared. A declared timeout is the call's budget; a trap, a timeout or a malformed answer applies the hook's
  `on_failure` (`allow` by default, `deny` for a guard) with E028. A plan a hook answers goes through the edit-plan step
  with the hook as its origin; a refused plan is a deny on a pre event and a notice otherwise. The adapter maps the
  neutral answer to the host's protocol.
- **D5. A hook or command keeps small state the host writes.** Per extension, per project and per session, JSON objects
  under `.specforge/state/` of at most 64 KiB, read when declared and patched with JSON merge patches the host applies
  after any plan under the write lock; a patch over the cap is refused whole. Sessions untouched for 30 days go at the
  next write. State is never project truth: no rule reads it and a compile never depends on it.
- **D6. Every install wires `core.spec_guard`**, the host's own hook, on a host's Write or Edit of a `.spec` file. It
  judges the file as the write would leave it, as a hand edit: a computed value (E082), a harness value with no ledger
  line (E083) or a kind or field only a person writes is denied, naming the entity, the field and the tool; any other new
  error is context for the agent. It is a boundary of the host's tools, not of the file system. There is no new judging
  mode: the compiled project's `edited` and a comparison of the human-only entities of the two graphs are enough.

## Rejected

Bodies in describe: heavy on every load. A manifest of what install wrote: one more file to merge, lost per clone when
git-ignored; a marker travels with the file. A global install. One dispatcher entry per event: per-hook timeouts and
failure policies are the host's own and a host shows which hook denied (the upgrade if many hooks per tool call prove
slow). Denying every raw `.spec` write: it would block prose, comments and layout the typed tools do not cover.
Reading a commit message from a `-F` file: the file is not a project file, so the host reads no message from it.

## Consequences

Four declaration lists and two exports (`render__`, `hook__<id>`), a host port and a crate of adapters, `install`,
`uninstall` and `hook` commands, a state store, a core hook, two doctor findings and a protocol minor shared with ADR
0291. An older host ignores the new keys and installs nothing. A project whose extensions contribute nothing to a host
declares and runs as it did. Codex and Cursor output is a draft until it has run in those hosts.

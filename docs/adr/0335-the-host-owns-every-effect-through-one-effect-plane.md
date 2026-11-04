# The host owns every effect through one effect plane

**Status:** accepted (2026-11-03, ports round, plan 01). Plan 01 writes D1, D3 and D5; plan 02 writes D2 and D6
(grants, consent, secrets); D4 (the faces extensions use) is a numbered gap a later plan of the round fills. **Supersedes exact
sentences only:** in [ADR 0002](0002-test-runner-extensions.md) "Execution", "extensions stay pure wasm with no
process, file or network access" and the sentence that rejects a guest-side `exec` host import; in
[ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md) D5, only the clause "and no extension needs it (an analyzer
is handed each file's content)". **Amends** ADR 0037 D1 (no capability beyond the `specforge:host` ports a grant
binds), [ADR 0079](0079-the-smt-solver-is-one-port-that-analyze-and-doctor-take.md) D4,
[ADR 0125](0125-collects-machine-is-one-port-the-caller-passes.md) D1 and D5,
[ADR 0145](0145-the-session-reads-sources-through-the-disk-the-surface-hands-it.md) D3 and D4 and
[ADR 0287](0287-who-writes-a-field-is-declared.md) D6. Builds on
[ADR 0326](0326-a-host-refuses-a-guest-of-a-newer-protocol-minor.md) D1a/D1b (the load seam).

## Context

- **A guest has no I/O.** Its world imports nothing and its WASI context grants nothing (ADR 0037 D1).
- **Extensions get I/O two bespoke ways:** declared reads the host serves (ADR 0293, ADR 0320), and a collector argv
  the host runs after consent (ADR 0002, ADR 0125).
- **The host spawns through three runners.** One of them, the collector's, has no deadline and no output cap: a test
  command that never ends keeps `specforge collect` alive forever.
- **The host reaches git through a process-global default.** `History` had a `static DEFAULT` that every surface
  installed and every reader fell back to.
- **The only HTTP client lived inside the registry adapter**, on webpki roots only.
- **Six path resolvers** decided "inside the project" each their own way. E016 decided nothing: it accepted `..`,
  absolute paths, links out and case variants, and its did-you-mean listed any directory on the machine.
- **What comes next needs more.** The serves-itself programme needs git, HTTP and files with nothing installed; value
  checkers need HTTP, git reads and contained files.

## D1. One effect plane, four ports

Every effect the host performs on behalf of anyone goes through `kernel/effects` (`specforge-effects`): an extension,
a check, a CLI verb or the host's own baseline. It has four ports, each an object-safe trait the caller passes:

| Port | Requests | Adapter (pure Rust, compiled in) |
|---|---|---|
| `Process` | run an argv in a directory, with a deadline, env changes, stdin and an output cap | `adapters/process` (`specforge-process`): the one runner. It resolves PATH × PATHEXT on Windows (the request's own when it sets them), starts the program in its own process group or a Job, kills that group or Job past the deadline or the output cap (and on the host's SIGINT or SIGTERM), caps output at 64 MiB per stream, and reads CRLF as LF in text mode. A program that exits cleanly leaving a child holding its output is not killed: its output is read for a short grace, then returned. The environment is the parent's with the request's variables, minus a secret deny-list applied last (`*_TOKEN`, `*_SECRET`, `*_PASSWORD`, `*_KEY`, `*_CREDENTIALS`, `AWS_*`, `GOOGLE_APPLICATION_CREDENTIALS`), so a request cannot put a secret back |
| `Http` | method, URL, headers (sensitive ones redacted), body, body cap, timeout, redirect count, private-address opt-in | `adapters/http` (`specforge-http`): reqwest 0.13 with `ring` and the platform verifier. On Linux the webpki roots are extra anchors, never a retry after a rejection. System and env proxies, every redirect re-checked and followed by hand, the destination's resolved-address class checked (never a proxy's own host), outcome classes normalised across OSes |
| `Fs` | stat, read, walk under a **named root** | `adapters/fs` (`specforge-fs`): std::fs behind `Contained` |
| `Git` | the nine History reads (`locate`, `commit`, `merge_base`, `files`, `tree`, `sizes`, `blobs`, `branch`, `status`), moved unchanged | `GitCli` (`specforge-git`), run through the `Process` port |

- **Behind a proxy.** The client judges a destination's name on this machine before sending, but the proxy resolves it: a name that does not resolve here, resolves differently there or changes between the two is the proxy's to refuse, and no client can close that.
- **Wire types.** Requests and outcomes are serde types in `protocol-types/src/effects.rs`. The host computes a
  canonical digest over them (`specforge_effects::digest`: sorted keys, sensitive header values redacted), so no
  guest blob gains a hash.
- **One path containment.** `Contained` replaces the six resolvers. A path is contained in the root it names
  (`spec`, `project` or a named root), not in the project; `"<root>/rest"` names the root inside the string. It checks
  lexically (an absolute path or a `..` that climbs is **E094**), then for real, hop by hop (a link that lands outside
  is **E094**), and compares each name with the directory listing (a case variant is **W174** on every OS). A
  backslash is read as `/` and reported (**W175**). An unknown `<name>` is **E095**. It applies the ADR 0293 D3a
  credential deny-list. A suggestion is only ever drawn from a parent `Contained` holds.
- **A `spec_root` outside the project** (ADR 0145 D4) is its own root: `spec`-rooted paths are contained in it, and
  `Roots` flags it as outside, so a grant can treat it as an outside root.
- **Composition.** Callers pass `Ports` from the composition root (the CLI, the LSP, the MCP server, the hook). The
  History `static DEFAULT` is deleted: with no git port given, no git is asked. `common::process`,
  `ProcessMachine`'s spawn and `extension_authoring`'s raw `output()` become calls on `Process`.
- **The limits of a port call.**

  | Port | Limit | Value |
  |---|---|---|
  | process | deadline ceiling, for any process a request or an extension asks for | 600 s |
  | process | deadline ceiling of the host's own `extension build` | 30 min |
  | process | output cap | 64 MiB per stream |
  | http | connect | 5 s |
  | http | total per request | 10 s by default, held to a 60 s ceiling |
  | http | redirects | at most 5 |
  | http | body ceiling | 64 MiB |

  - Time inside a host import does not count against the guest's epoch deadline: `Store::epoch_deadline_callback`
    re-arms it, and the port's own deadline bounds the import.
  - An effectful call of a guest that imports a port runs on a fresh instance, not under the extension's mutex.
  - An answer copied into a guest is held to a quarter of its `max_memory_mb` (`http::guest_body_cap`).

## D2. A grant is what is declared, gated, under the ceiling and approved

What an extension may make the host do is its **grant**, decided once per extension and environment
(`specforge_effects::Grant`, pure over its inputs): the intersection of what it declares, what the call kind's gate
allows, what the project's effects ceiling leaves, and what this machine approved.

- **D2.1 Declared.** An extension declares its **capabilities** in a describe category, `capabilities`: each a
  **scope name** (a word, unique in the extension), a required `why`, the call kinds it is for (none: every kind the
  gate allows its port) and one port scope: `http` (schemes, methods, hosts listed or `from_values`, an optional
  `secret_header`), `fs` (roots and ops: stat, read, walk, write), `process` (an argv template and a deadline) or
  `git` (read, write). A collector with a command names its `process` capability (`capability`); the SDK's
  `.run(argv)` declares that capability for it. A bad capability is W021 and grants nothing. `Operation`, the call
  kind, is a wire type.
- **D2.2 Gated.** A fixed table of the host says which ports each call kind may use: a pass, validator or hook reads
  files and git; a prompt, render, scan or migrate reads files; collect may use every port; a command every port but
  process; an MCP tool or resource reads files and git and may use http; handshake and describe none.
- **D2.3 Under the ceiling.** `effects` in `specforge.json` can only narrow (`"off"` removes a port; an object keeps
  what it lists), and `roots` names extra roots beside the built-in `spec` and `project`, each resolved once per load
  to its canonical target, inside or outside the project. What is not used as written is E109 at its key; each
  capability the ceiling narrows is W171 at the narrowing key. There is no per-extension ceiling: ADR 0288's `gates`
  switch an extension's family off.
- **D2.4 Approved.** File and git reads inside the project need no approval; `http`, `process`, a file or git write
  need a **capability approval** per project, extension, provenance and scope name, builtins included; a named root
  outside the project needs a **root approval** per project, keyed by its canonical target and shared by every
  extension. An approval stores the scope as the ceiling narrowed it and covers any narrower grant; a wider one asks
  again. A builtin's approval is keyed by its name and meets only a builtin load; a third-party extension's (a `.wasm`
  file entry's included) by its blob's digest, so a new blob asks again. Every approval a run needs is settled before
  its first effect (`specforge_ops::settle::settle`): `--yes` approves for this run and stores nothing, a terminal is
  asked once per scope or root and a yes is stored, nobody to ask (MCP, `--format json`) means stored approvals only.
  Only the scopes a run uses are asked for (collect: the selected collectors' commands).
- **D2.5 The store.** The approvals live in the user's **consent store**, `~/.specforge/consent.json` (versioned,
  outside every project, so a cloned repository cannot approve its own effects), behind the `Consent` port
  (`specforge_ops::consent::UserConsent`; `testing::MemoryConsent`). Every change is read, changed and written under a
  sidecar lock, privately and atomically (ADR 0193). A store that cannot be read or parsed, that a newer specforge
  wrote, that lies inside the project (`SPECFORGE_CONSENT_FILE` there, or a home that is the project or lies in it), or
  that belongs to another user or is writable by group or others, or whose directory is (Unix; Windows has no such
  mode, ADR 0178), is never written over and approves nothing: W186.
  People read and change it with `specforge grants` (`list`, `--store`, `approve`, `revoke`); doctor lists each
  capability's grant and the named roots.
- **D2.6 Provenance.** The load hands the runtime each extension's provenance (builtin, or a third party and its
  blob's digest), and the approval key is built from it.
- **D2.7 Denied.** A request the gate, the declaration or the ceiling refuses is E090 and is never asked about. One
  that needs an approval this machine has not given is the caller's to say: collect's E059 (naming
  `specforge grants approve`), or W170 for the scopes a run here uses unapproved (doctor, `specforge grants`).

## D3. Every effect has a record

- **The record.** Each performed effect yields an `EffectRecord`: the canonical request (`EffectRequest`: process,
  http or fs) and its digest, the `Outcome` (its class and its digest), `observed_at` and `ttl_s`.
- **The outcome class** is normalised across operating systems and displays as one string: `ok`, `status:404`,
  `exit:1`, `signal`, `deadline`, `capped`, `missing`, `denied`, `outside`, `not_found`, `cannot_start`,
  `unreachable:{dns|connect|tls|timeout|reset|proxy|refused_address}`, `redirect:{limit|downgrade|refused}`. Any TLS
  rejection is `unreachable:tls` on every OS.
- **Where records live, and how a run reads them,** is the journal's ADR (the protocol 2.0.0 round); this ADR fixes
  only the record's shape.

## D4. (reserved for the guest-imports plan)

## D5. The load seam reads imports first

The host lists a component's imports before it instantiates it (`Component::component_type().imports`), as ADR 0326
D1a/D1b shipped: `imports::admit` checks them against `HOST_PORTS`, empty until the guest-imports plan links the
ports. An unknown `specforge:host` interface or another protocol major is E088; an import outside `wasi:*` and
`specforge:host` is E028. This decision states the seam the ports join; it builds nothing.

## D6. A secret rides as a header the host attaches

- An `http` capability may name `secret_header {header, secret, scheme?}`: a secret the extension declares (ADR
  0288). The grant resolves it to the secret's environment variable and holds it with the hosts the capability lists;
  the granted Http port sets it on the request it permits, outside the request's wire form and digest. No guest names
  or sees a value.
- The Http adapter decides each hop again: the header (`<scheme> <value>`, or the bare value) is attached only to an
  `https` hop whose host the capability lists, its value read from the environment at send time and marked
  sensitive; a redirect elsewhere or off `https` carries nothing, and an unset variable sends nothing.
- A `secret_header` on a `from_values` capability or one whose schemes allow `http` is W021: a value written in a
  spec would otherwise choose where the token goes. Records, diagnostics and error messages name the header, never
  its value.

## Rejected

- **Granting standard WASI** (preopens, `socket_addr_check`, `wasi:http`). Preopens hand real directories to
  third-party code, address checks cannot express host names and rebinding defeats them, and the kernel would hold no
  typed request it could record.
- **One resolver per layer.** E016 lives in `pipeline` and cannot see `app`; that is why three copies existed. The
  kernel holds the resolver over a `Lookup` seam (canonical path, link target, directory names), which the registry's
  `Disk` and the real file system both answer.
- **Moving the registry's `Disk` into the kernel now.** `Disk` stays the session's recorded read seam; the `Fs` port is
  the effect port; `Contained` serves both.
- **Containing every path in the project.** A spec-rooted field could then reach any project file by `..`, and
  `<project>/` would mean nothing.
- **Permissions in `sandbox_policy`.** ADR 0037 removed them; capabilities are extension-wide declarations, like
  `settings`, and the handshake keeps identity and limits.
- **Approval by a digest of the scope.** A digest cannot tell a narrower scope from another: narrowing the ceiling
  would ask again.
- **Treating an unusable consent store as empty.** The next approval would write over every approval it held.
- **A per-extension ceiling.** ADR 0288's `gates` already switch an extension's family off.
- **A process runner kept in `common` as a fallback.** Two runners would again disagree on deadlines; the runner is an
  adapter and the composition root passes it (ADR 0079 D2, ADR 0125 D2).

## Consequences

- One runner bounds every spawn. A collector stopped at the deadline (600 s) or the output cap is reported under
  E045 with the reason.
- E016 no longer answers for a path outside its root: such a path is E094, and a project that reached outside a root
  turns red until it names `<project>/…` or a named root. No shim.
- Linux reports W174 where only a case variant exists, instead of E016; macOS and Windows report W174 instead of
  passing.
- The registry client sends through the `Http` port; reqwest moves to 0.13 and the platform verifier, so corporate CAs
  work.
- Every surface builds one `Ports` and passes it down; tests build one from the `testing` fakes.
- `~/.specforge/collector-consent.json` is no longer read: a collector's command is approved again, as its
  extension's `process` capability, and kept in `consent.json`. A `.wasm` file entry's approval is asked again after
  each rebuild (its digest changed).

## What would reopen it

- A guest that needs a socket or a protocol the four ports cannot express.
- A measured need for async (concurrent effects inside one call).
- A consent store that is not a file (a keychain), or approvals a team shares.

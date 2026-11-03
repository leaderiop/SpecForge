# The host owns every effect through one effect plane

**Status:** accepted (2026-11-03, ports round, plan 01). Plan 01 writes D1, D3 and D5; D2 and D6 (grants, consent,
secrets) and D4 (the faces extensions use) are numbered gaps that later plans of the round fill. **Supersedes exact
sentences only:** in [ADR 0002](0002-test-runner-extensions.md) "Execution", "extensions stay pure wasm with no
process, file or network access" and the sentence that rejects a guest-side `exec` host import; in
[ADR 0037](0037-the-sandbox-is-limits-the-host-enforces.md) D5, only the clause "and no extension needs it (an analyzer
is handed each file's content)". **Amends** ADR 0037 D1 (no capability beyond the `specforge:host` ports a grant
binds), [ADR 0079](0079-the-smt-solver-is-one-port-that-analyze-and-doctor-take.md) D4,
[ADR 0125](0125-collects-machine-is-one-port-the-caller-passes.md) D1,
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

## D2. (reserved for the grants plan)

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

## D6. (reserved for the grants plan)

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

## What would reopen it

- A guest that needs a socket or a protocol the four ports cannot express.
- A measured need for async (concurrent effects inside one call).

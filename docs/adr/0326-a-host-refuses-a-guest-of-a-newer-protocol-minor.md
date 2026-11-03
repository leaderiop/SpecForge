# A host refuses a guest of a newer protocol minor

**Status:** accepted (2026-11-01, R0 of the Beyond Parity build plan, version drift fixes C and D). Amends the protocol
check of [ADR 0012](0012-one-extension-declaration.md) (the declaration's load checks the protocol major). No
wire change: the handshake already states `protocol_version`.

## Context

The host checked only the major of a guest's `protocol_version`. Every minor adds keys an older host does not
know, and an older host reports each as W138 and ignores it (`docs/extension-protocol.md`, "Protocol Versions").
A host on 1.6 loaded a guest built for 1.14 and dropped its hooks' `hosts`, its `reads`, its settings' gates: the
guest ran, without what it declared, and the only sign was a list of warnings. Hooks and `.mcp.json` run whichever
`specforge` is first on PATH, so a project's own guests meet an older host as soon as that binary lags behind.
A guest can also import an interface the host does not provide (a `specforge:host` port of a newer protocol), which
the runtime met only as a linker error at instantiation.

## Decision

- **D1. A newer minor is refused.** A guest whose protocol has the host's major and a newer minor fails its load
  with `ProtocolError::NewerVersion`, naming the extension (as its handshake declares it), the guest's version and
  the host's, and saying to update specforge. The load reports it as **E088** (a newer major or minor is no
  E028: the guest is fine, the host is old). `specforge add`, `init` and `publish` refuse such a binary with E088.
- **D2. What still loads.** An older minor, as before (the host hands it the values it knows), and a newer patch
  of the host's minor: a patch changes no payload.
- **D3. The version a binary speaks is visible.** `specforge --version` names the protocol after the commit,
  `specforge 0.3.0 (9229143f5, protocol 1.14.0)`, so the E088 remedy can be checked.

- **D4. Doctor compares the specforge on PATH.** The hooks and `.mcp.json` run the first `specforge` on PATH, not
  the binary that installed them. `specforge doctor` finds it in Rust (the PATH directories, no `which`), runs it
  with `--version` and reads the protocol D3 prints. When it speaks an older minor than the running specforge or
  than an extension the project loads, or names no protocol (a release before D3), doctor warns with **W169**,
  naming its path and both versions and saying the hooks and the MCP server run it. No `specforge` on PATH is an
  info finding. It is the user's machine, not the project: the CLI prints it beside the report (`specforge on
  PATH:`, JSON `specforge_on_path`), as it does credentials, and `specforge.doctor` does not.

## Rejected

Loading a newer guest with a warning: the guest then runs without part of what it declared, which is the bug.
Negotiating down (the host asks the guest to speak its own minor): the SDK builds one declaration, and a guest
would have to keep every older shape of it.

## Consequences

A guest built with a newer SDK needs a specforge at least as new. Every builtin and the gsd guests speak the host's
own version, built with it, so none is refused. A release that bumps the protocol minor makes its guests refuse
the older releases, which is what the version says.

## Amendment (2026-11-02, ports round, inside R1)

- **D1a. Imports are read before instantiation.** The runtime lists a component's imports before it instantiates it
  (`Component::component_type().imports`). A `specforge:host` interface the host does not provide is **E088**, naming
  the interface, its version and the host's protocol, with the newer-minor remedy (update specforge). An interface
  outside `wasi:*` and `specforge:host` is **E028** naming it: no specforge provides it. A foreign import is reported
  first, and every offending name is listed. A `wasi:*` import the linker lacks stays E028 at instantiation.
- **D1b. Every other major is E088.** A newer major is E088 like a newer minor (update specforge). An older major is
  E088 with the remedy to install a version of the extension built for the host's major, or rebuild it with an SDK of
  that protocol. A `protocol_version` that is not semver is a failed handshake (E028).
- **Why no bump.** No payload changes: what is refused, and how, is host-side.
- **Consequence.** The ports round's 2.0.0 host refuses a 1.x guest with E088 and no further change.

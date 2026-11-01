# A capability extension plugs into @gsd/core at its hook points through typed commands

**Status:** accepted (2026-10-31, @gsd/security round, plan 16).
Extends [ADR gsd/0001](0001-one-cargo-workspace-built-on-demand.md).

## Context

Upstream gsd-core v1.19.0 splits GSD into a core and capabilities. A capability is a `capability.json` of steps,
contributions and gates at named points (`verify:post`, `plan:pre`, `ship:pre`, ...); `gsd-tools loop render-hooks
<point>` renders the active ones into a workflow, and `gsd-tools check predicate` evaluates a gate. `@gsd/core`
(r34-r44) ports the workflows with 22 stubs, `X_HOOKS_JSON="[]"`, where upstream ran `loop render-hooks`: the loop
capability is not part of it. `@gsd/security` is the first capability extension, and `@gsd/codebase`, `@gsd/ops`,
`@gsd/ai` and `@gsd/ui` follow, so the shape is decided once.

## Decision

### D-S-1 (no kernel change)

The kernel needs no change for a capability extension: routes are keyed by `(short, cli_name)`, so `specforge gsd
<command>` reaches the extension that declares it; enhancements, `scoped_by`, `human` commands and peer-scoped
presets are built. The ship gate is a verb the workflow runs, as upstream's is, and the kernel does not refuse a ship.

### D-S-2 (the crate layout; the template)

```
gsd/
├── Cargo.toml          members = ["kit", "core", "security"]
├── kit/                gsd-kit (rlib only, no guest): the kind DSL, the prompt pipeline pieces, agent
│                       attribution, the capability-hook renderer; no assets, no export
├── core/               gsd-core, depends on gsd-kit
└── security/           gsd-security (cdylib + rlib), depends on gsd-kit
    ├── Cargo.toml      [package.metadata.gsd] tracks = "gsd-core@1.19.0"
    ├── README.md       status, model, rosters (no contributions/ folder: one command, one agent)
    ├── spec/README.md  the model: kinds, codes (a table a test parses), settings, commands, hooks
    ├── LICENSE-GSD     upstream's MIT notice (copy of core's)
    ├── assets/         agents/gsd-security-auditor{,.compact}.md, commands/gsd/secure-phase.md,
    │                   gsd-core/workflows/secure-phase.md, capabilities/security/capability.json
    ├── src/            lib.rs, kinds.rs, settings.rs, codes.rs, passes.rs,
    │                   commands/{hooks,gate,threat_ids,record,accept}.rs, prompts.rs, agents.rs
    └── tests/          declaration.rs, settings_roster.rs, port.rs, accounting.rs, parity_roster.rs,
                        parity_rules.rs, cli/ (black-box; `#[path]` includes core's harness), golden/,
                        projects/
```

A capability crate never has `gsd-core` in its `[dependencies]`; as a dev-dependency it serves the tests' harness.

Why a `kit` crate and not `gsd-core` as a dependency: `component_guest!` generates the component's `call` export
(`crates/runtime/extension-sdk/src/guest.rs:119-158`), so linking core's rlib into another guest exports it twice.
Why not `#[path]` includes in `src/`: five extensions would include core's files and break on core's first `crate::`
use. Tests do use `#[path]`, as core's own tests already do (`gsd/core/tests/port_workflows.rs`). Decision **U2**.

### D-S-3 (one hooks command per capability; core asks all five)

`<cap>_hooks <point>` is the contract every capability extension implements (see "The command contract"). The renderer
is `gsd_kit::capability::render(capability_json, settings, point)`, a port of the per-capability part of
`loop-resolver.cts` render-hooks: `when` is read against the extension's settings and `configValues` are filled. An
off `when` leaves the hook out; `skippedHooks` holds only the steps `--after-fingerprint` moves there.
`shortcut:` core's line spawns `specforge` once per capability name (five, ~0.1-0.3 s each with a warm compile cache);
upgrade to one call when a kernel round lets a command read the served commands.

### D-S-4 (threats live in the plan)

A threat is scoped by its plan: the planner writes it beside the plan's tasks,
`T-03-SC` repeats per plan as upstream, and the cross-plan uniqueness is `E101`, faithful to upstream's own parser
(phase-scoping would make a repeated `-SC` an `E002` and drop it).

### D-S-5 (statuses are written once, the counts are computed)

The auditor's verdict sets `threat.status` through `record_security_audit` (single writer, #2119); `threats_open`,
`blocking` and `status` are computed so a changed `block_on` or an accepted risk takes effect at once. Judged
mitigations (a verdict that goes stale when a cited file changes) are gsd-plus and not built.

### D-S-6 (the fixture)

A new fixture `gsd/parity/fixtures/vault/` (a tiny Node CLI, `node:test`), not the taskboard (new plan text moves its
goldens). Its `.planning/` is written by hand from upstream's templates: phase 01 with two plans whose registers both
claim `T-01-02` (the duplicate) and `T-01-SC`, summaries (one with Threat Flags) and an `01-SECURITY.md` (state A: one
open high threat, one accepted risk, one trail block); phase 02 with one plan without a register and its summary, no
SECURITY.md (state B, retroactive). `record-security.sh` runs upstream's verbs over it and fails if one errors, so the
verbs vouch for the hand-written files. Decision **U3**.

### The command contract

`<cap>_hooks <point> [--after-fingerprint <phase>]` (`specforge gsd <cap>-hooks <point> --format json`) is the contract
every capability extension implements. It answers `{activeHooks, skippedHooks}` as upstream's `render-hooks --raw`
does. With `--after-fingerprint`, a `step` hook whose `produces` the phase already has moves to `skippedHooks` with
`reason: "produces-present"` and its `artifacts` (upstream's `partitionHooksByFingerprint`, #5105); the extension
decides what "has" means (`@gsd/security` maps `SECURITY.md` to the phase having a `security_audit`). Core passes the
flag where upstream does: `verify-work.md` and `autonomous.md` at `verify:post`, with the phase's id for the directory.
`shortcut:` core's line passes the flag to every capability; one that does not accept it exits 2 (a usage error), which
the line leaves out silently like "not loaded". Only `@gsd/security` exists today and takes it; upgrade, when a second
extension lands, by making each one accept the flag. A gate verb answers
`{block, message, details}` and exits 0 on a block, as `loop-hook-dispatch.md` says; the workflow reads `block`.
`@gsd/core`'s capability points name the five capabilities (`ai`, `codebase`, `ops`, `security`, `ui`) so a later
capability extension needs no core edit. A capability extension declares `@gsd/core` as a required peer, and its model
override and agent variant are presets following core's (`model_profile`, `workflow.compact_content`).

## Divergences

Each is named by its tag in `gsd/parity/ACCEPTANCE.md` ("Known divergences", `@gsd/security`):
`[blocking-needs-security]`, `[fragment-repointed]`, `[audit-advisory-every-check]`, `[accept-risk-human]`,
`[accepted-risk-agent-creatable]`, `[no-superseded-plans]`, `[stride-enum]`, `[retro-threat-in-plan]`,
`[threats-open-live]`, `[security-absent-off]`, `[verify-post-produces]`, `[injection-key-explicit]` and
`[surface-peer-setting]`. A change to this contract that moves one of them moves its row there.

## Limits

- The `shortcut:` of D-S-3: one `specforge` spawn per capability name; one call when a kernel round lets a command read
  the served commands.
- The ship gate is agent-run, as upstream's: `security_gate` is the check the workflow runs, `/gsd-ship` itself is not
  refused (decision U1). It fails closed: a workflow that cannot read the gate (the command failed, or answered
  nothing) treats it as `block: true`.
- `--after-fingerprint` is passed to every capability, and one that does not accept it is left out (the `shortcut:` of
  "The command contract").
- A capability that fails (any exit but 2) is warned about on stderr and its hooks are left out, as upstream's resolver
  fails open with a warning; exit 2 is "not loaded" and stays silent.
- A hook line's variable holds one answer per line, one per capability extension in the loop's order, and the agent
  reads the `activeHooks` of each: nothing merges them (G4, plan 05 T1).
- The hook lines need `bash` and `specforge` only; a PowerShell host cannot run them, as with every ported shell line.

## Rejected

- A kernel "gate contribution" now (an extension adds its codes to a peer's gated command): more than upstream does.
- `gsd-core` as a dependency of the capability crates: `component_guest!` would export `call` twice.
- `#[path]` includes of core's files in a capability's `src`: they break on core's first `crate::` use.
- Phase-scoped threats: a repeated `T-03-SC` would be an `E002` and be dropped.
- Extending the taskboard fixture: new plan text moves its goldens.

## Consequences

Each later capability extension is a crate under `gsd/` depending on `gsd-kit`, its `<cap>_hooks` command, its settings
and presets, a section of ACCEPTANCE and a fixture. `gsd/kit` has no guest and no assets; a guest in it is a defect.

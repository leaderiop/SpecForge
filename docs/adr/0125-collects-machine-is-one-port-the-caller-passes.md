# Collect's machine is one port the caller passes

**Status:** accepted (2026-10-16, architecture round 13, plan 03). Amends [ADR 0002](0002-test-runner-extensions.md)
("Execution"). Mirrors [ADR 0079](0079-the-smt-solver-is-one-port-that-analyze-and-doctor-take.md). Leaves ADR 0110 and
ADR 0112 as they are. Amended by round 15, plan 07 (D7: each mode checks only the locations it reads).
Amended by [ADR 0331](0331-the-gsd-requirement-thread.md) (D5: a collector chosen by detection whose run yields no evidence is skipped, not fatal).
Amended by [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D2 (D5 covers every capability and root a run uses; D1's approvals move to the `Consent` port).
Amended by [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) (D1: `ProcessMachine` runs through the `Process` port, held to a deadline ceiling and an output cap, with the env policy).

`collect` orders consent, a command run, a report read and a merge, but it reached the machine directly: a child
process, the wall clock and the user's consent file through `HOME`. No ops test could run the flow past consent: every
test of a run spawned a `/bin/sh` script (15 CLI subprocess tests), three MCP tests read the developer's real
`~/.specforge/collector-consent.json`, and the start time of a run was the wall clock, which cost two flakes in one day
(a report directory read as empty on Linux CI; a stale report adopted in a fast suite). Two ordering defects went
unseen: an approved collector's command ran before a later collector's refusal was decided, which contradicts
`spec/behaviors/mcp-operations.spec` ("invalid_input before any command runs").

## D1. One port

`specforge_ops::analysis::collect::CollectorMachine` (spec port `CollectorMachine`): `run`, `approved`, `approve`.
Its adapters are `ProcessMachine` (production) and `testing::ScriptedMachine` (tests).
The port is defined in the base module `machine` (as the solver's port is, so the test support can name it
without reaching into the analysis group, ADR 0086) and re-exported by `analysis::collect`.

## D2. The caller passes it

`collect(view, request, machine)`: a third parameter, as the solver is passed (ADR 0079 D2). The CLI builds
`ProcessMachine::user()` per command; the MCP server holds one (`McpServer::with_collector_machine` for tests) and
hands it through `ProjectRef`. Neither the environment, the view nor the caller's value (ADR 0124) carries it: the
value is what the host lends a call, the machine is the host's for the process's lifetime.

## D3. The clock is the adapter's

`Ran::started` is no later than the modification time of any file the command wrote. The coarse-clock grace is
`ProcessMachine`'s: it is a fact about the kernel, not about collect.

## D4. What stays in collect

Selection, containment and file preparation, the consent decision over `Asking`, announcing, reading, dispatch,
convention, merge and save. The port opens no project file; the capture is handed to it as a writer.

## D5. Refused before anything runs

Every refusal that needs no run (a report location, a capture declaration, an empty command, an approval) is decided
for every selected collector before the first command runs; every approval is asked first.

## D6. A test chooses the machine it sees

In process, a `ScriptedMachine`; for a `specforge` child process, a `PATH` and a `HOME` the test builds.

## D7. Each mode checks what it reads

`collect` plans one evidence source per collector (`analysis::collect::evidence`): a run (the declared report and
captured output, the command, the approval), the report already written (the declared report and captured
output), or the files the request names (`Mode::Reports`, `specforge collect --report`). D5 holds for what the
source reads: under `--report` the collector's declared report, capture and command are neither resolved nor
refused, since nothing is read from them. A named file still goes through the request's containment.

## Rejected

- **Two ports** (a runner and a consent store): they always travel together and are built at one place.
- **The machine in the caller's value**: every operation's request would carry a machine only collect reads.
- **A `now()` method or a clock port**: collect would still own a grace that is about the kernel.
- **The port returning stdout bytes**: it buffers a whole test log and loses "written as it arrives".
- **Consent per collector**: "approve once, then the suite runs" would not hold with two runners.
- **Saving a partial report** when a later collector fails after running: out of scope.

## Consequences

- `collect` takes a third argument (Rust API break in `specforge-ops`).
- MCP's tests no longer read the user's consent store.
- The CLI asks every approval before running any collector.
- No wire, document or diagnostic change; no re-vendor.

## What would reopen it

A second operation that runs a declared command (then the port leaves `collect`), or a consent store that is not a
file (a keychain: a third adapter).

## Amendment (architecture round 15, plan 07)

D5's list is the refusals of the locations a mode reads (D7). `collect --report` no longer refuses a declared `../`
report (E058), an unknown capture (E058) or a declared report that lands outside (`outside_root`). Rejected: one
module per mode (three modules sharing the declared locations, for three modes unchanged since ADR 0002); an `Option`
for the declared locations (the mode and the option could disagree).

## Amendment (2026-11-03, ports round, [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md))

- **D1:** `ProcessMachine` runs the approved command through the `Process` port the caller passes. The run is held to
  the collector deadline (a 600 s ceiling) and a 64 MiB output cap; a stopped run is E045 naming why. The command
  gets the parent's environment minus a secret deny-list. The clock of D3 is the runner's.

## Amendment (2026-11-03, ports round, plan 02, [ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D2)

- **D5** covers every approval a run needs, not only a command's: every capability scope the selected collectors use
  and every named root outside the project is settled before the first command runs (`specforge_ops::grants::settle`),
  and each command is then held to its extension's grant (E090 for what the grant refuses, E059 for what nobody
  approved).
- **D1:** the approval half of the port moves to the `Consent` port (`specforge_effects::Consent`, the user's
  `consent.json`, `specforge_ops::consent::UserConsent`). `CollectorMachine` keeps `run`; `approved`, `approve` and
  `CommandApproval` are gone, and `collect` takes the consent store beside the machine. This reopens the "two ports"
  rejection by its own clause: the consent store now serves every port, not collect's alone.

# An import command's plan is judged for host errors only

**Status:** accepted (2026-11-24, round r6-breadth, Phase 13). Amends
[ADR 0391](0391-gsd-import-reads-a-planning-folder-once-into-one-edit-plan.md) D9 and D13 (what refuses an import, what
`check` reports after it). Extends [ADR 0303](0303-a-collectors-plan-is-judged-for-host-errors-only.md) (a collector's
plan is judged for host errors only) from collected facts to imported history, under the same rule. Builds on
[ADR 0289](0289-an-extension-writes-through-an-edit-plan-the-host-applies.md) (E080) and
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (judged findings). No WIT change and no
protocol version change: one key added to the current minor, 2.3.0.

## Context

- `specforge gsd import` copies a legacy `.planning/` folder into `spec/gsd/` as one edit plan (ADR 0391). Like any
  command's plan, it was refused (E080) for any error it would add, an extension's included.
- 13-09 ran it through the binary. The vault fixture's `.planning/` claims threat `T-01-02` in plans 01-01 and 01-02,
  so `@gsd/security` reports `gsd.security/E101` twice, and the host refused the whole import. This repository's own
  `.planning/` was refused for `gsd/E201` (a context with no canonical refs) and `gsd/E312` (a task action saying
  "placeholder"). Both are rules about how a project should be written, and a project that ran upstream GSD broke them
  before SpecForge existed in it.
- Refusing the import leaves the project unimported and fixes nothing: the user would have to edit legacy Markdown to
  satisfy rules of a system they have not adopted yet, then import again. That is ADR 0303's case: a collected fact is
  history, and refusing it for an extension's error starves every later fact.
- The host names no extension (the dependency rule), so it cannot know `@gsd/import` is an import unless the command
  says so.

## Decision

- **The command declares it.** A command may declare `imports: true` (the SDK's `CommandBuilder::imports()`, which
  also declares `writes`): it imports what was recorded outside the spec. A command that declares `imports` and not
  `writes` is W021 and dropped, as a `human` one is. `@gsd/import`'s `import` declares it.
- **Its plan is judged for host errors only.** The host applies the plan with
  `Origin::Command { extension, imports: true }`, and E080 writes, rather than refuses, an error that a third-party
  extension reported in its own namespace (`gsd.security/E101`, `gsd/E312`), in the plan's check and in the
  re-judgement under the write lock. `validate::Lands`, threaded through `plan_steps`, `validate::check` and
  `refuse_new_errors` in place of ADR 0303's `host_only` flag, names the three cases: `Nothing` (a typed edit, a
  command's or a hook's plan), `Extensions` (a collector's plan, ADR 0303, unchanged: any extension's error, whatever
  its code) and `ThirdParty` (an importing command's plan).
- **What still refuses.** Every error whose code is first party, the host's own or a `@specforge/*` builtin's, which
  the host's catalog describes (ADR 0286): a path field naming no file (E016, which `@specforge/checks` reports for a
  path value check, so its origin is an extension's), an unresolved reference (E003), an id the grammar refuses, two
  entities with one id, a file the parser refuses. And everything checked before E080, unchanged: a malformed plan,
  the field flags (computed, context, human, a harness field of an extension that is not the command's or a peer's), a
  verdict from any extension but the rule's own (ADR 0391 D5), a stale revision, a collected entity.
- **What lands.** A third-party extension's rule error the imported records carry is written. `specforge check`
  reports it beside the owed verdicts and exits 1, and the user fixes it in `spec/gsd/` with SpecForge's own tools.
- **Who may declare it.** Any extension. The declaration is in the extension's contributions, so its declaration
  golden shows it, and a review of the extension sees it once rather than per call.

## Amends ADR 0391

- **D9.** "What the guest cannot see is the host's" now reads: the host refuses a plan that adds any unjudged *host*
  error, a first-party code (E016 included). A third-party extension's rule error lands (this ADR). The guest still
  mirrors the host's own refusals (D9 "Mirror the host") and the E016 paragraph stands: a path that no longer exists
  refuses the whole import.
- **D13, Check.** `specforge check` on an imported fixture exits 1. Its codes are the owed judged codes (D5) and the
  rule errors the fixture's `.planning/` carries: none for taskboard and ledger, the vault's two `gsd.security/E101`.

## Rejected

- **Naming `@gsd/import` in the host.** The host names no extension, and the next importer (another tool's planning
  folder) would need its own exception.
- **Exempting only the importing extension's own rules.** The vault's errors are `@gsd/security`'s, a peer's
  (ADR 0303 rejected the same for collectors).
- **The guest drops or refuses a record that breaks a rule** (13-09's options a and b). Dropping loses history the user
  wrote; refusing blocks the import until legacy Markdown is edited; and the guest would have to run every rule of
  every extension, which only the host does.
- **A flag on the edit plan.** A command could then opt out per call; the declaration is fixed and reviewable.
- **A new `Origin::Import`.** Every place that reads a command's origin (a peer's harness write, a verdict's origin)
  would have to name both; an import is still a command.
- **An import judged exactly as a collector's, any extension's error landing.** `@specforge/checks` reports a path
  that names no file (E016) as an extension's finding, so a stale path would land, which ADR 0391 D9 refuses: the user
  corrects that path in `.planning/`. A collector keeps ADR 0303's rule; its facts name files it just read.
- **Every command judged host-only.** A command that authors (a new plan, a quick task) must stay held by the
  project's rules (ADR 0297).

## Limits

- An import that adds a host error is still refused whole, as ADR 0391 D9 says for E016.
- An agent may run an importing command. It can add no host error and pass no field flag; what it can add is an
  extension's rule error, which `check` reports.
- A host that predates the key ignores it and refuses such a plan for an extension's error, as before.

## Consequences

- The vault imports: its two `gsd.security/E101` land, and `check` reports them with the owed verdicts.
- A real project's legacy `.planning/` is no longer refused for `gsd/E201` or `gsd/E312`; paths it names that no
  longer exist still refuse it.
- `protocol-types` and the SDK are inputs of every committed blob, so the phase's single re-vendor (13-11) rebuilds
  every builtin, and each grows by the serializer of one more key: 286 B for `@specforge/governance` and for
  `@specforge/software`, measured on a macOS build with the blob profile's settings, so about 7.7 kB over the 27
  builtins, more than the 356 B `TOTAL_BUDGET` had left. 13-11 re-measures `TOTAL_BUDGET` in its blob commit
  (ADR 0373 D3) and names this growth there.

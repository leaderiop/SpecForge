# The ports protocol is 2.0.0, a call's inputs are one list, and effects are journaled

**Status:** accepted (2026-11-03, ports round, plan 03). Takes extension protocol **2.0.0**. Builds on
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) (D3: the `EffectRecord` this ADR stores) and
[ADR 0326](0326-a-host-refuses-a-guest-of-a-newer-protocol-minor.md) D1b (another major is E088). **Amends**
[ADR 0114](0114-environment-inputs-are-recorded-as-read.md) and
[ADR 0133](0133-recorded-inputs-are-one-table.md) (an effect record is a recorded input with an expiry),
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) (a verdict digests outcome classes),
[ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) D1 and D5 (`FileRead` is `Input::Files`) and
ADR 0326 D2 (the 1.x line ends at 1.15.0). Folds the wire keys of
[ADR 0292](0292-an-extension-contributes-agents-skills-commands-and-hooks.md),
[ADR 0310](0310-a-command-and-a-prompt-read-their-extensions-state.md),
[ADR 0316](0316-a-prompt-reads-the-slash-commands-the-project-serves.md),
[ADR 0320](0320-a-hook-is-told-git-facts-and-the-real-path-of-what-its-tool-names.md) and
[ADR 0323](0323-a-hook-reads-the-settings-of-another-extension-it-names.md) into `Input` variants.

## Context

- **Eight bespoke keys.** A call declared what the host serves it before the call through `reads`, `reads_tests`,
  `reads_state`, `reads_graph`, `reads_slash_commands`, `reads_git`, `reads_git_status` (with the qualifier
  `git_when`) and `reads_settings`, spread over five descriptors (pass, command, prompt, hook, collector). Each key
  cost its own protocol minor, its own W138 shape, its own SDK builder and its own serve site.
- **Git status was text.** A hook that read git status was handed `git status --porcelain=v2 --branch` as written,
  and each guest parsed it again (`@gsd/core`'s statusline did).
- **Effects left no trace.** ADR 0335 gave every effect a record (D3) and left where records live, and how a run reads
  them, to this ADR. Nothing kept an HTTP answer, so a check could not be re-run offline, a verdict could not say what
  an effect answered, and an editor keystroke had no rule that kept it off the network.
- **No finding said whose truth it was.** `--strict` promoted every warning, and the gate's per-code baseline counted
  every warning: a finding decided by this machine's network or approvals would fail CI on one machine only.

## D1. Protocol 2.0.0

- `PROTOCOL_VERSION` is **2.0.0**: the `Input` fold (D2) and the typed git status are wire changes an older guest
  cannot survive, so they take a major, not a refusal of older minors.
- Every 1.x guest is now an older major, refused with **E088** as older by ADR 0326 D1b, with no code change and no
  decoding shim. The 1.x history stays in `docs/extension-protocol.md`; the 1.x line ends at 1.15.0.
- `check_protocol_version(host, response)` is a pure function of both versions, so "an older minor of the host's major
  loads" (ADR 0326 D2) stays tested under a named host.
- The ports round's later items join 2.0.0 without a bump: `CodeDescriptor.machine` (D7), `capabilities`,
  `specforge:host` imports, `checks` and `CheckValues`. A 2.0.0 describe category goes after `Routes`.

## D2. A call's inputs are one list

```rust
#[serde(tag = "input", rename_all = "snake_case")]
pub enum Input {
    Files(FileRead),                              // was `reads`
    Tests,                                        // was `reads_tests`
    State,                                        // was `reads_state`
    Graph,                                        // was `reads_graph`
    SlashCommands,                                // was `reads_slash_commands`
    Git { status: bool, when: Option<String> },   // was `reads_git`, `reads_git_status`, `git_when`
    Setting { extension: String, name: String },  // was one `reads_settings` entry `<ext>:<name>`
}
```

- Each descriptor that declares inputs has one field, `inputs: Vec<Input>`; the eight keys are deleted. On the wire:
  `"inputs": [{"input": "files", "name": "sources", "glob": "src/**/*.rs"}, {"input": "tests"}]`. A 2.0 guest that
  writes an old key gets W138.
- **One table says what each call kind is served** (`specforge_registry::inputs::served`). It serves exactly what was
  served before the fold; widening it is a capability decision, not a refactor.

  | Input | pass | command | prompt | hook | collector |
  |---|---|---|---|---|---|
  | `files` | yes | yes | – | – | yes |
  | `tests` | check, compute, guard | – | – | – | – |
  | `state` | – | yes | yes | yes | – |
  | `graph` | – | – | yes | yes | – |
  | `slash_commands` | – | – | yes | – | – |
  | `git` | – | – | – | yes | – |
  | `setting` | – | – | – | yes | – |

- **W021** reports an input its call kind is not served, a second `tests`, `state`, `graph`, `slash_commands` or
  `git`, a `setting` naming its own extension, and a bad `FileRead`. The input is served empty and the call is kept.
- **`fact_stores` stays implicit.** It is derived from the entity snapshot every pass gets (ADR 0327 D6).
- **The git input's status is typed.** `GitFacts.status` is a `GitStatus`: the branch (`oid`, `head`, `upstream`,
  `ahead`, `behind`) and git's entries in order, each a kind (`changed`, `renamed`, `unmerged`, `untracked`,
  `ignored`), a `/`-separated path from the repository's top, the original path of a rename, and the index and
  worktree change (`modified`, `type_changed`, `added`, `deleted`, `renamed`, `copied`, `unmerged`). `dirty()` is any
  entry that is not `ignored` (ADR 0327 D3). The `Git` port's `status` answers it: the CLI adapter parses
  `git status --porcelain=v2 --branch -z` once, and no guest parses text. Submodule state, file modes, object names
  and the rename score are left out; they would be an additive key.
- `specforge extensions` and doctor list every declared input of each contribution, not only file reads.

## D3. The effects journal

- **A journal record holds no secret, and its files are private** (ADR 0288): files `0600`, directories `0700`.
- One record per request digest (ADR 0335 D3), in `<root>/.specforge/effects/journal/<aa>/<hex>.json` (gitignored,
  per machine); a loose root's records go to the user's `~/.specforge/effects/journal/`. A record's path is built
  only from a `RecordDigest`, `sha256:` and 64 lowercase hex characters. Writes are atomic; the last record of a
  request wins. An unreadable or undecodable record is a miss, overwritten by the next record: the journal is a
  cache, so it costs no code.
- A record shows what identifies the request: the method, URL and argv, the header and env names, a non-sensitive
  header's value, and a body or stdin as its sha256 and length. A sensitive header's value is never written: the
  request digest's canonical form writes it `<redacted>` (ADR 0335 D3), and replay matches by digest.
- A response payload (an HTTP body, a process's output) is kept beside the record, `<hex>.payload`, bounded by the
  request's cap, and checked against the outcome's digest when read; a mismatch is a miss.
- A lookup answers `Fresh`, `Expired` or `Missing` against `observed_at + ttl`.
- **Time to live by answer:** an HTTP success 24 h; an HTTP failure (`status:4xx`, `status:5xx`, `unreachable:*`)
  1 h; a process `None` (journaled for audit, never served fresh). Any other answer (`denied`, `deadline`,
  `redirect`, `capped`, …) is not journaled, so a grant takes effect at once. fs and git are never journaled: they
  are local reads the disk stamps.
- A guest is served the outcome (class, digest and payload), never the request.
- The term is **effects journal**, never the bare "journal": the **Write journal** (ADR 0289) is another thing.

## D4. An effects mode, per command

| Mode | http, process | fs, git |
|---|---|---|
| `online` | performed when granted, and journaled | performed |
| `auto` | a fresh record; else performed when approved; else not performed | performed |
| `cached` | a fresh record; else not performed | performed |
| `replay` | any record, expired or not; else a miss | performed |

- `Effects` holds the mode. `cached` and `replay` hold no ports, so a session opened in them cannot perform an effect:
  the type proves the LSP's worker never reaches the network.
- `Environment` holds an `Effects`, `cached` by default, so the hook, watch, the LSP and MCP sessions stay off the
  network with no change. Only the CLI opts in: `--effects <online|auto|cached|replay>` on `check` and `watch`, the
  commands that run the checks to report them. `check` defaults to `auto` when someone can answer (human output, a
  terminal at stdin) and `CI` is unset, else `cached`; `watch` and every other command run `cached`. `--effects`
  always wins. MCP `validate` takes `effects` (default `cached`) and re-runs the effect phases for that call over the
  served compile, as `since` re-judges, so I017 is kept. The CLI and MCP build the effects of a run in one place.
- **What a run lends an extension runs in a mode too** (`specforge_project::effects::lent`, ADR 0335 D4). A CLI
  extension command and `collect` default to `online`, a person having asked for that run, and `--effects` wins (a
  command that declares its own `effects` arg keeps it, and runs `online`). An MCP extension tool of an extension with
  an http or process capability, and MCP `collect`, take `effects`, default `cached`; a resource read has no such
  argument and stays `cached`. So does any `--dry-run`, unless it asked for `replay`.
- A replay miss refuses the run (`not_in_journal`, exit 2), listing each missed request. It is no diagnostic: a test
  or a forensic replay fails loudly, and a user never reads it as a project finding.

## D5. An effect record a check consulted is a recorded input

- The recorded-inputs table (ADR 0133) gains one class, `Effect { expires }`, after `Facts`: one row per record a
  check consulted, a missing record included, recorded in the checks' run.
- A row is stale when its record file changes (another run refreshed it) or when `expires <= now`. Its change re-runs
  the checks. The table answers `next_expiry`; `SessionWatch` wakes at it, and the LSP applies `expired()` before it
  answers a request.
- fs and git reads need no new row: they are already `Check` and `Ref` rows. The fact rows (ADR 0327 D9) are unchanged.

## D6. A verdict digests each checked value's outcome class

- An entity that has checked values digests `[digest, id, {"checks": [[field, item, checker, class], …]}]` in a
  judged selector; one without digests as before, so no existing verdict goes stale. `observed_at` is never digested:
  the same class re-checked later keeps a verdict fresh, a URL that starts answering 404 stales it.
- A check not performed digests as `unknown`. Its input joins `Inputs::unjudged` (freshness skips it, as a missing
  report) and `Inputs::unchecked`; **I207** counts those verdicts apart from I205. Nothing goes stale or red offline
  or on a fresh clone.

## D7. A diagnostic this machine decided

- `Diagnostic.machine` (serialized only when set) says this machine decided the finding: its approvals, its network,
  its journal. A core code carries a `machine` marker in the catalog; an extension's code says
  `CodeDescriptor.machine` (SDK `.machine()`), and the host sets the flag where the finding enters.
- `--strict` never promotes a machine diagnostic (a preset that raises its code still does: that is the project's
  choice). The gate leaves machine warnings out of its per-code baseline, prints them on one line, and checks the
  specs with `--effects cached`.
- **I206** ("*n* value checks not run") and **I207** are machine codes. W170 (grants) and the checks foundation's
  W172, W173, W181 and W182 are marked by the plans that add them.

## Rejected

- **A decoding shim** that reads the old keys beside `inputs`: two ways to say "send me the state" is the bespoke-key
  problem again.
- **Refusing older minors instead of a major:** it would break ADR 0326 D2 for every future minor.
- **A mode per surface:** the default is one `Environment` value; only the CLI's argument and MCP `validate`'s opt in.
- **`HttpProbe` and `GitLog` inputs:** a value checker's probes come from its plan phase, a data-dependent effect is
  a port import, and no pure call kind reaches the network.
- **A replay code:** a miss is a refusal of the run, not a finding about the project.
- **A per-code allow-list in the gate:** the gate cannot read a third-party catalog; the flag travels with the
  diagnostic.

## Consequences

- Every 1.x extension must be rebuilt with a 2.x SDK; the builtins are re-vendored with the bump.
- `@gsd/core`'s statusline reads `GitStatus` and no longer parses porcelain text.
- `.specforge/effects/` grows per machine; pruning is left until records are counted in practice.
- A definite failure seen fresh on one machine (a 404) is an error there and an I206 elsewhere; the warning baseline
  stays machine-independent.

## What would reopen it

- A pure call kind that needs a data-dependent effect.
- A journal large enough to need pruning or sharing between machines.

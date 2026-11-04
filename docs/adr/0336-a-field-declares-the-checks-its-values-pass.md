# A field declares the checks its values pass

**Status:** accepted (2026-11-04, ports round, plan 05). D8, generic checkers in one foundation extension, is recorded
in ADR 0338 (plan 06, extended by plan 07). Builds on
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) and
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md). **Amends**
[ADR 0145](0145-the-session-reads-sources-through-the-disk-the-surface-hands-it.md) D3 (checks read through `Contained`,
and an outside file only through a named root),
[ADR 0147](0147-a-value-is-written-in-its-fields-one-form.md) (no change of form, recorded for clarity),
[ADR 0285](0285-a-fields-type-may-be-a-list-of-words-a-block-a-union-or-a-range.md) (`file_base` is replaced by checks and
the prefix), [ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md) D3 (one containment, and
`FileRead.root`), [ADR 0304](0304-a-judged-input-reads-a-path-field-goes-on-past-a-reference-and-a-pass-sees-the-tests-a-verdict-cites.md)
(a judged `file()` path may name its root with `<root>/`), ADR 0337 D4 (`collect` defaults as `check` does), and adds a
note to [ADR 0026](0026-typed-diagnostic-codes.md) D3. Plans 06 and 07 each add an "Amends 0336" line for the probe kinds
they add (D9), and plan 07 amends D4 (extra rounds).

**As of plan 06** `file_reference`, `file_base` and `file_exists` are deleted, in the commit that adds the `file`
checker, so that E016 has exactly one emitter at every commit: a declared `file_exists` rule is W112 naming
`@specforge/checks:file`, and E016 is a `checks` code. No registry rule reads the file system any more (`RuleInput`
carries the records and the edges only).

## Context

- **There is no value-constructor syntax.** `URL("…")` is E001 twice, and the formatter preserves the region as written.
- **"File" is a bool on any field type**, `file_reference` plus `file_base`, and no surface shows it.
- **E016's flaws.**
  - It joins a path to a root without containment, so `..`, absolute paths, symlinks out and case mismatches all pass.
  - Its "did you mean" lists the names in any directory.
  - It points at the whole entity.
  - ADR 0145 D3 allows outside paths, while ADR 0293 D3 contains the same paths.
- **No extension can check a value against the world.** `custom` validators get field text and no I/O. Passes get file reads and no network.

## D1. Values stay plain; the field declares its checks

- A `FieldDescriptor` gains `checks: Vec<CheckRef>`. A `CheckRef` is either one check, `{ checker, args }`, or `{ any_of: [..] }`, one level deep.
- The value is written in its field type's form (ADR 0147), so a URL is a string: `health "https://checkout.example.com/health"`.
- The grammar, the tree-sitter parser, the formatter and the TextMate grammar do not change.
- `file_reference` and `file_base` are deleted. The SDK's `f.file(root)` builds a check on `@specforge/checks:file` with that `root` argument.

## D2. A project can attach a check to a declared field

`specforge.json` `checks` holds two keys:

```json
"checks": {
  "fields":  { "<kind>.<field>": ["<checker>" | {"checker": "…", "args": {…}}] },
  "publish": false
}
```

- An unknown kind, field or member path is W019.
- An unknown or ambiguous short checker name is W019, listing the candidates.
- Args the checker refuses, or a field type it does not apply to, are W189.
- Attached checks run after the field's declared ones.
- `publish` decides whether check outcomes become facts (D5).

## D3. A checker is declared by an extension and named by it

`CheckerDescriptor { name, applies_to: [FieldType], args, scopes: [capability scope names], codes, description }`, in a describe category `checkers`:
- The name is qualified by its extension, so two extensions can each have a `url` checker, and no registration clash exists (this is not E026).
- In a declaration, an unqualified name is the extension's own checker. Another extension's checker is written qualified (`@specforge/checks:file`), and that extension must be a declared peer (W019 otherwise).
- In `specforge.json`, a short name resolves when exactly one loaded extension declares it.
- A check whose checker belongs to an optional peer that is not loaded is inert: no diagnostic, and the schema and hover say "not loaded".
- `codes` lists every code a finding of the checker may carry, each catalogued (ADR 0286).

## D4. A value check is its own call kind, in two pure phases

`Operation::CheckValues` runs in a values phase that comes **first**: before judged rules, compute, registry checks and passes, because judged digests read its outcome classes. A value written by a compute pass or a verdict cannot carry checks (W189).

1. **plan.** One `__check_values` call per extension, not per checker, carries every checked value: (entity, field, item index, text, args). For each value, the guest returns findings (a syntax or scheme error, no probe needed), or probes inside its checker's `scopes`.
2. **The host dispatches the probes**, in this order:
   - it drops the probes of later checks on a value whose plan already gave an error, so an invalid URL is never fetched;
   - it refuses a probe outside the checker's scopes with E090;
   - it contains every path itself (D6) and reports a refusal on the value, so the guest never learns about a path outside its root;
   - it deduplicates by request digest;
   - it performs the rest concurrently (32 workers, at most 8 per host) under ADR 0335's grant and ADR 0337's journal and mode.
   A probe not performed (offline, `cached` miss, not approved, over budget) leaves its value **not judged**: it counts toward one I206 per checker and toward W170 for the scopes it needed.
3. **judge(values, outcomes) → findings.** A second `__check_values` call carries the values whose probes all ran. Each finding carries a code, a message and an optional byte sub-span inside the quotes (the scheme, the `#…` part, the root). The host places it on the value's span.
   - Plan 07 amends this decision: a judge may answer `More { probes, carry }` to ask for another round, at most **3 rounds** per value.

- `any_of` passes when one branch yields no error. Otherwise every branch's findings are reported.
- A trapped or malformed answer is E028, once per extension per run, and that extension's values are not judged.
- Neither phase imports a port. The call-kind gate refuses it.

## D5. Results are cached, stamped and surfaced

- **The checker cache.**
  - A plan is memoised by (extension, origin version, checker, args digest, text). The origin version is the builtin version or the blob digest. A plan never expires.
  - A judgement is memoised by the plan key plus the sorted outcome digests of every round.
  - The memo is held in memory and on disk in `.specforge/effects/checks/` (20,000 entries, least recently used dropped). A check run with fresh probe records calls neither the guest nor the network. The key is the whole item with its call-local id zeroed, so nothing that decides the answer is left out.
- **The stamps.** The effect records a check consulted are recorded inputs (ADR 0337). A judged verdict over an entity digests each checked value's **outcome class**. An unperformed check digests as `unknown` and is I207 (ADR 0337, amends ADR 0290).
- **The surfaces.**
  - `inspect` and MCP list each checked value as {field, item, value, checker, status, outcome, observed_at, from_cache}.
  - LSP hover prints one line per checker: "`url` · 200 · checked 2 h ago". Hover on a field name lists its checks.
  - A checked value's string token gains the semantic-token modifier `checked`.
  - A checker that declares its values are paths (the `file` family) gets `<root>/` and contained path completion and document links; one whose values are URLs gets document links (plan 05 T17). The kernel keys on the declaration, never on a checker's name.
  - The schema shows `SchemaField.checks`, and `specforge extensions` lists each extension's checkers.
  - Export adds outcomes only with `--with-checks` (MCP `with_checks`).
- **No committed fact by default.** A passing check writes no fact. A collector that declares the `check_outcomes` input is never auto-selected: it runs only when `specforge collect --runner` names it, or on a plain `collect` when `checks.publish` is `true`. It publishes outcomes as K23 facts (R1's fact store). Its evidence is the served rows (`CollectInput.check_outcomes`): it needs no command and no report and is never E045 for want of one, and `--reports` never adds it. The rows are the values phase of the compile the collect read, and that compile runs the phase only when a selected collector reads them, so a `collect` with no reader probes nothing. It takes `--effects`, and `check`'s default effects mode (ADR 0337 D4): `auto` when someone can answer and it is not CI, `cached` otherwise, where the rows say `not_run` for what no record answers; CI publishes fresh rows with `--effects online`.

## D6. Files: named roots, contained, one way

- **The written form** is a bare path, which uses the reader's default root, or `"<root>/path"` naming a root. A field declares no default root: `file_base` is gone, and the `file` checker's `root` argument (default `project`, as a `files` input's and a judged `file()` read's) is the checker's. A `root` argument that is no root's name is W189, and the check is dropped.
- **One parser.** `RootedPath::parse` in `protocol-types` reads the prefix. The host and the guest kit compile the same parser. Every path reader uses it: value-check probes, K11 field `reads`, the judged `file()` selector and MCP project paths.
- **The roots** are `spec`, `project`, and named roots in `specforge.json` `roots: {name: path}`. `<root>` alone names the root itself, so a walk can start there. A named root is the only way to name a file outside the project, and a root that resolves outside the project needs a per-machine root approval keyed by its canonical target (ADR 0335 D2). MCP refuses it unless it is approved. A `spec_root` outside the project (ADR 0145 D4) is treated the same way.
- **Resolution** goes through the kernel's `Contained`. A path is contained in the root it names:
  - lexical (`..`, absolute, drive or UNC) and real (each link hop) escapes → E094;
  - an unknown root → E095, on the root;
  - the case of each component compared with the directory listing → W174, on every OS;
  - a backslash → W175, and the path is read as `/`;
  - missing → E016 from the `file` checker, on the value span. Its "did you mean" names only a sibling the host found inside the root.
- **This amends ADR 0145 D3.** Checks read through `Contained`, not plain.

## D7. Codes

- The host reports containment with the core codes E094, E095, W174 and W175 (ADR 0335). The foundation's codes have owner `checks` (ADR 0338).
- **Severity is a code, not an argument.** A checker that offers a gentler reading declares an enum argument that selects another code, and lists both in `codes`. For `url`, a 404 or 410 is E093, or W183 when the field's args ask for a warning. An argument named `severity` is W189 (ADR 0026 D3). A finding whose code breaks the checker's `codes` or the catalogue is W150, and is kept.
- A definite answer from the world is E: a 404 or 410, a malformed URL, a forbidden scheme, a missing file. 5xx and 429 are W181, and 401 and 403 are W182. A transport failure (DNS, timeout) is W172.
- An effect not performed is I206 or W170 (summary). Both are `machine` diagnostics and never fail `--strict`: nothing goes red offline.

## D9. Capabilities have scope names; probes cover the families

A capability carries a `name`, which is its approval scope (ADR 0335 D2). Approval is keyed per (extension, scope), so approving `links` does not approve `registries`. The foundation's scopes are `files`, `links`, `media-remote`, `registries` and `git`; only the three network scopes need approval.

The probe set this ADR defines is:
- `fs.stat`, `fs.read_prefix` (≤ 64 KiB), `fs.read` (≤ 1 MiB), `fs.walk{glob}`;
- `http.head`, `http.get` (range, `max_bytes`), `http.tls` (peer certificate `not_after`);
- `dns.resolve` (std `ToSocketAddrs`, system resolver);
- `git.object`, `git.ref`, `git.tracked`.

Plans 06 and 07 extend it, each with an "Amends 0336" line:
- `fs.digest{sha256}`, streamed by the host for any size until the run's deadline, gated as a read (plan 06);
- one `ByteRange` on `fs.read` and `http.get` (`Whole`, `Prefix`, `Suffix`, `Span`), which replaces `fs.read_prefix` and the separate range form, with no alias (plan 07);
- `http.post_query`, a bounded JSON POST to a fixed-host capability (plan 07).

The guest parses the bytes it is served. The kernel never learns what an image, a PDF or a heading is.

## D10. One value rule leaves the kernel's rule kinds

- `CheckKind::FileExists` is deleted. `@specforge/checks:file` is the one way to say a file exists. The LSP re-checks a missing file live, by its file stamp, not by a TTL.
- `CheckKind::FieldValueConstraint`'s `matches` **stays** in the kernel. It is pure, about 35 rules in product, formal and gsd use it, each with its own code, and moving it would ship full `regex` again in the guest or change its meaning. The foundation has no `pattern` checker.
- `non_empty` and `one_of` stay (structural).
- Graph rules (edges, cycles, required fields, `verify`, `judged`) stay kernel rule kinds.
- No shims: a declaration using a deleted kind, or one of `CheckKind`'s four alias spellings, is W112 at load, naming its replacement.

## Rejected

- **A value-constructor grammar `Name("…")`.**
  - It needs a new `FieldValue` variant across about 16 match sites, `grammar.js` + `parser.c`, `kind.rs`, lexer agreement, formatter properties, a format-version bump, and the untested TextMate grammar.
  - Every older binary fails it with E001.
  - It contradicts ADR 0147 (the field type decides the form).
  - Its one gain, self-typing values in untyped fields, is covered by D2.
- **One `FieldType` per checker** (`url`, `file` …). Each new checker would cost a protocol minor and about 20 files (the `EnumList` precedent), and checkers would not compose (a string that must be both a URL and on an allow-list).
- **Checkers as passes with imports.** Not cacheable and not batchable, and passes run in the LSP.
- **A per-checker `severity` argument.** ADR 0026 D3 makes another severity another code, and the host would report it as W150.
- **Deleting `matches` for a `pattern` checker.** See D10.
- **A default root declared by the field** (`file_base` kept). Each reader already has its own default, and the prefix overrides every reader.
- **Keeping E016 unbounded** (ADR 0145 D3 as written). It is an existence oracle for any path on the machine.

## How plan 05 settled the rest

- **The parser lives in `protocol-types`**, compiled by host and guest, because the guest kit must read the same prefix the host contains; a second parser could disagree with it. `Root` lives beside it, and `Contained` takes it.
- **A bare path's default root is its reader's**: K11 reads take their own, judged `file()` reads from the project (ADR 0304), a check's is the checker's `root` argument. The prefix overrides every reader.
- **A check of an unloaded optional peer is inert only when exact**: its qualifier names an optional peer of the declaring extension that the project does not load (`UnloadedPeers`). A checker a loaded extension lacks, or of an extension that is no peer, stays W019. A `specforge.json` check has no declaring extension, so it is never inert.
- **The host reports containment, before any probe runs**. A guest never receives an outcome for a path outside its root, whatever a third-party checker does.
- **A finding's severity is its code's catalogue level**. There is no `severity` argument in the SDK or on the wire.
- **One call per extension per phase, not one per checker**. The probes of all checkers are deduplicated together.
- **One run has a budget**: 16 probes per value, 120 s and 64 MiB of bodies, reads and listings, with 50,000 file and git probes; an editor's check gets 2 s, 8 MiB and 2,000. What is over it is not judged and counts toward I206 as `budget`, so a later run, answered from the journal, gets further. One `__check_values` call carries at most a quarter of its guest's memory limit: more is split into several calls, and an item alone over that is not asked.
- **Selection is keyed on the declared input**, so the kernel stays domain-free, and test runners without detection files keep the single-runner fallback.

## Consequences

- Protocol 2.0.0 (ADR 0337), with ADR 0335: `FieldDescriptor.checks`, checker declarations, `Operation::CheckValues`, `CheckPlan`, `Probe`, `ProbeOutcome`. `FileRead.base` is deleted; `FileRead.root` names a root. Builtins re-vendor. `file_reference` and `file_base` are deleted in plan 06.
- Projects that referenced files outside the root turn red with E094 until a named root is declared and, if it lies outside the project, approved on the machine (no shim).
- The four alias spellings of a check kind are deleted (W112), and `CheckKind::FileExists` in plan 06; `matches` stays (D10).
- A probe's journal record is keyed by the request and the extension that asked, so it answers that extension alone in every mode. `cached` and `replay` hold no grant by design (ADR 0337), so the key is what keeps one extension's secret, redirect or 401 from answering another. Two extensions asking one URL each make their own request; that is the cost.

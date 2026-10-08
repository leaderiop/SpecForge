# Diagnostic codes are typed constants; extension codes are checked where they cross

**Status:** accepted (2026-10-06)

Every emit site chose a code and, separately, a severity: 120 `Diagnostic { code: "X".to_string(),
severity: … }` literals, 26 `Diagnostic::error|warning|info("X", …)` calls and ~50 other code
literals in 73 files of 17 crates. A 643-line test scanner guessed which literal was a code, its
owner (from the file path) and its severity (the nearest severity token within five lines); it
could not decide 58 of the 302 sites it should check, and one of its branches had been dead since
08203ee3. Codes that extensions report crossed the protocol unchecked: a third-party rule reporting
`E001` at `Info` was presented as "Parse error".

## Decision

**D1. One table.** The catalog is one `catalog!` table in `specforge-diagnostics` (still without
dependencies, ADR 0004 D6-d). Each entry states a code, its level, its owner, its title and its
explanation once. It generates `CATALOG` (explain, MCP explain, diagnostics JSON titles, doctor,
the LSP hover, `docs/diagnostics.md`) and a constant for every core code: `codes::W112: Code`,
`codes::R_RES_005: Code`, `codes::A010: GradedCode`. An `E`/`W`/`I`/`A` prefix that contradicts the
level does not compile.

**D2. The host builds a diagnostic from a code.** `Diagnostic::new(code, message)` takes the
catalogued level; `Diagnostic::graded(code, severity, message)` is for `A###` findings whose pass
sets it. `Diagnostic` is `#[non_exhaustive]`: no other crate writes a literal. `Diagnostic::untyped`
takes a text code for codes that arrive as text (extension rules and passes, an `OpError` turned
back into a diagnostic, tests); a test lists the files that may call it. `OpError::new`, the CLI's
`print_error` and ops' `fail` take a `Code`.

**D3. Severity changes only through policy.** The catalog's level is the severity a diagnostic has
when built. `DiagnosticPolicy` (strict promotion, lint profiles) is the only thing that changes
severities afterwards, and only upwards. There is no per-site override: another severity is another
code.

**D4. Extension codes stay text and are checked where they cross.** Builtins and third-party
extensions report codes as text in rule descriptors and pass diagnostics. The builtins do not link
the catalog: `build-builtins --check` records every file a builtin compiles, so they would need
re-vendoring on every explanation edit. `specforge_diagnostics::check_extension_code(extension,
code, level)` decides: a catalogued code belongs to its owner at its level (any level for `A###`);
a retired code is nobody's; a first-party extension reports only catalogued codes; others report
E900–E998 / W900–W998 / I900–I998 at the level the prefix states. Rule registration and pass
conversion call it; a misuse is **W150**, and the rule or finding is kept as declared. Every
diagnostic that crosses there records its extension (`Diagnostic::origin`, JSON `origin`), and the
catalog describes a diagnostic only for its owner (`describes(code, origin)`): a kept finding with a
squatted code is never titled or explained as the code's owner's (as LSP's `source` beside `code`).

**D5. Owner stays catalog data.** `specforge explain`, MCP `specforge.explain`, the docs and the
extension check read it. Host constants exist only for core codes, so no host owner is inferred
from a path; guest sources are attributed exactly by directory (`extensions/<name>`, and
`specforge-coverage` to `@specforge/testing`, ADR 0004 D2-f).

**D6. The scanner keeps only exact checks.** No code literal in host source; every catalogued code
is referenced (host constants) or emitted (guest literals) by its owner; `untyped` is called only
from the listed files. The severity window, path-inferred host owners and the dead JSON branch are
deleted.

## Consequences

- Adding a core code is one table entry plus `Diagnostic::new(codes::X, …)` at the emit site, then
  `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`. A severity cannot be
  chosen at the emit site.
- Adding an extension code is one table entry (owner = the extension) plus the declaration as
  today; registry build and a test over every builtin declaration check the level.
- Third-party extensions that use codes outside their range, or whose prefix contradicts their
  severity, now get W150.
- Extension diagnostics carry `origin` in the JSON every surface prints, and the hover names the
  extension; host diagnostics print as before.
- ~~The emitter's errors still carry their code inside message text (`"E003: …"`), formatted from
  constants. Giving them a typed code is left for later.~~ Closed by
  [ADR 0015](0015-read-views-are-operations-over-the-project-view.md), section "Query" (Q4).
- The decisions land in steps (architecture plan 11): the table and the constants first (D1), then
  the constructors beside the old ones, the conversion of every emit site, the extension check
  (D4, W150), closing construction (D2), `origin`, and last the scanner (D6).

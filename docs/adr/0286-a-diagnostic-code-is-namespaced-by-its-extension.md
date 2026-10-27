# A diagnostic code is namespaced by its extension

**Status:** accepted (2026-10-27, architecture round 30, plan K10). Amends [ADR 0026](0026-typed-diagnostic-codes.md)
D3 and D4. Supersedes [ADR 0278](0278-a-code-is-explained-by-the-catalog-else-by-the-loaded-extensions-rules.md) D4 and
amends its D1 to D3.

## Context

Third-party extensions shared one code range, E/W/I 900-998: 99 codes per letter for all of them together, with
no catalog. Two extensions using one code reported the same text: rule codes drew W023, which neither user
could fix, and pass codes were never compared. A third-party code had no title, explanation or docs link on any
surface, and a squatted core code printed as the core code. ADR 0278 let `explain` and the hover read a rule's
`description`, which reaches rule codes only: a pass's code stayed unexplained, and no code had a title or a docs
link. GSD (`@gsd/*`) declares codes numbered by area, `gsd/E101` to `gsd/E511`, which no shared range holds.

## Decisions

- **D1. A third-party code is `<namespace>/<letter><three digits>`.** The namespace comes from the package
  name: `@s/core` is `s`, `@s/n` is `s.n`, a local `n` is `n`. The extension writes its local code
  (`E101`); the host adds the namespace where the code enters (rule registration, pass findings), so no
  extension reports under another's namespace or as core. Each extension owns its whole number space.
- **D2. Core's and the `@specforge/*` builtins' codes stay bare**, in the one static catalog that generates
  `docs/diagnostics.md`.
- **D3. An extension catalogs its codes** in the `codes` describe category (protocol 1.7.0): local code, title,
  explanation, optional docs URL. An uncatalogued code, a code that is not a local code, or a code reported at
  another level than its letter is W150; a malformed entry is E030; a first-party extension declares none.
  A guest older than 1.7 is not asked for the category. A validation rule's `description` (ADR 0278 D4) is
  removed: `codes` is the one place a third-party code is described.
- **D4. One code book describes every code**: the static catalog and the loaded extensions' catalogs. Diagnostics
  JSON titles, LSP `codeDescription` and hover, doctor, `schema --validation-rules` and `explain` (any case,
  `gsd/e101`) read it. Explain answers a namespaced code with the same entry shape as a core one (ADR 0278's
  rule-list reply is gone); a bare code is never resolved into a namespace. A namespaced code links only to the
  docs URL its entry declares.
- **D5. Two loaded extensions with one namespace are E039.** The first in load order owns the namespace's
  descriptions. W023 is retired (replaced by E039).
- **D6. A code's letter is its declared severity.** `Raises` (code text to severity) raises it, never lowers it;
  settings and profiles fill it and the project's report applies it before a surface's strict (K6), so every
  surface, the editor included, shows one severity.

## Rejected

A larger shared range: it postpones the collision. The extension's short name (`ext_short`): it is not unique
and cannot hold a dot. A namespace declared in the handshake: a field and a uniqueness rule that package names
already give. A structured code on the wire: every JSON reader changes for what one string holds. Namespacing
the builtins: 89 codes and every blob churn for no user-facing gain. Lowering a severity per code: ADR 0026 D3.
A raise held by each surface's `DiagnosticPolicy`: the LSP builds none, so the editor and `check` would disagree.
Keeping ADR 0278's rule `description` beside `codes`: two texts for one code, and none for a pass's code.

## Consequences

`Diagnostic.code` holds `ns/E101` for a third-party finding; filters and scripts that matched a third-party
`E9xx` now match `ns/E9xx`. A 1.6 guest loads, and its codes are W150 until it is rebuilt with a catalog; one
that sends a rule `description` gets W138 for the key. `specforge explain E903` becomes `specforge explain
example.marketing/E903`.
Namespace collisions are caught among the extensions one project loads, not across a registry.

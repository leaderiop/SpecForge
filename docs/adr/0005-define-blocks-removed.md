# Define blocks are removed; custom kinds come from extensions

**Status:** accepted (2026-10-01)

The spec promised `define <name> { ... }` blocks in `.spec` files that would register project-local
entity kinds (`custom_entity_types_via_define`, the `define_blocks_registered` event, the
`define_extension_kind_uniqueness` invariant). The grammar parsed them, but nothing ever converted
one into a registered kind: `register_define_blocks` had no production caller. Using one gave
misleading errors instead (E003 for its field names read as references, E013 for `define behavior`).

Wiring them would have meant designing their field vocabulary and adding a pass over parsed
`.spec` files before the registries freeze. That makes a project's registries depend on its
sources, not only on `specforge.json` and the loaded extensions, so the LSP and watch would
have to reload the environment on any edit that touches a `define` (ADR 0004, D5-a).

We removed them instead:

- Every entity kind comes from an extension. A project that needs its own kinds writes one
  (`specforge new <name> --extension`).
- The grammar still parses `define` blocks and `define` stays a reserved word. The compiler
  reports each block with W143 and leaves it out of the graph, on a full build and an
  incremental one.
- `register_define_blocks`, `DefineBlockConfig` and their spec entities are deleted; the
  phases no longer wait on a `define_blocks_registered` barrier.

**What would reopen it:** users who need custom kinds without writing any code. Then define
blocks come back as their own design, one that states how the environment reloads.

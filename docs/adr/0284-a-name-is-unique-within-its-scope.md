# A name is unique within its scope, and an entity's id is its scope's path and its name

**Status:** accepted (2026-10-27, architecture round 30, plan K2). Extends [ADR 0117](0117-a-reference-is-decided-in-the-linker.md),
[ADR 0163](0163-a-written-token-is-found-by-one-finder.md),
[ADR 0176](0176-the-finder-is-the-only-record-of-where-a-name-is-written.md),
[ADR 0233](0233-a-use-binding-is-a-finder-token.md), [ADR 0032](0032-one-graph-build.md),
[ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md) and
[ADR 0103](0103-a-core-mcp-resource-is-one-module.md) and
[ADR 0282](0282-an-unknown-type-name-is-the-hosts-e004-and-a-project-has-one-spec-block.md) (E004 and the type-name
edge read one resolution; a `spec` block is never scoped). Builds on ADR 0283 (a name may hold `-` and `.`).

## Context

- An entity id was one key for the whole project. A second `D-01` was E002 (same kind) or W060 (other kind), and
  its block was dropped. A reference named an entity by that exact key.
- GSD reuses names by design: `D-01` in every phase, `T1` in every plan, `03` in every milestone after a phase
  number reset. Its users name an entity from outside its scope as `03/D-01` or `v1.0/03/D-01`.

```text
BEFORE  decision D-01 { phase 03 } x2       E002, one dropped
        decisions [D-01]                    the one D-01
AFTER   decision D-01 { phase v1.0/03 }     node v1.0/03/D-01
        decision D-01 { phase v1.0/04 }     node v1.0/04/D-01
        decisions [D-01]  (in phase 03)     v1.0/03/D-01
        decisions [04/D-01]                 v1.0/04/D-01
        decisions [/D-09]                   D-09 (rooted: the project)
        decisions [D-01]  (in phase 05)     E003: not in its chain; candidates 03/D-01, 04/D-01
        requirement AUTH-01                 AUTH-01 (unchanged)
```

## Decisions

1. **A kind may be scoped by one of its reference fields.** `scoped_by: "<field>"` on its declaration names a single
   `reference` field with a target kind; that kind opens a scope. The field is optional: an entity without it is in
   the project scope. A field that is not such a reference, a structural target or a scope cycle is W021.
2. **An entity's id is its full path**: its scope's path, `/`, its name. An unscoped entity's path is its name.
   The graph, edges, exports, the Graph Protocol, pass input, MCP and diagnostics carry the path; nothing splits it,
   a node holds its scope.
3. **A name is unique within its scope.** E002 (same kind) and W060 (other kind) apply per scope, unchanged in
   severity. A `spec` block is never scoped: a second one is E002 whatever its name (ADR 0282 D2).
4. **A reference resolves from its holder outward, and nowhere else.** It is tried in the holder itself, then each
   enclosing scope, then the project; a path `03/D-01` resolves at the nearest level where the whole path resolves,
   and a rooted path `/D-01` at the project only. A miss is E003 naming the entities that bear the name elsewhere,
   each a fix. Only what has no chain takes a unique suffix: a scope field, resolved from the project in the order of
   the kinds' scope depth, and a surface's input; several matches are E003 (`ambiguous_entity` on a surface).
5. **A path is a reference value**: names joined by `/`, optionally rooted by a leading `/`, in a value, a list item
   or a `use` binding, never in an entity header. A binding still changes no resolution.
6. **Every surface reports the full path and takes a short form**: a path, or a unique suffix. What is written into
   a file (typed edits, completion, fixes, respellings) is the shortest spelling that resolves by the chain from its
   holder, never by the suffix; a scope field is therefore written as its full path.
7. **An edit through SpecForge never changes what an existing token names.** A rename keeps the scope and edits each
   segment that names the entity. Setting a scope field moves the entity and its subtree; a create may shadow a
   farther target or make a short scope field ambiguous. Each respells every token whose target would change. In a
   rename or a move, a token that named nothing and would name something is a capture, refused.
8. **A project with no scoped kind sees no difference.**

## Rejected

- Scope by file path: moving a file would rename every entity in it.
- Project-wide names with the scope prepended (`03-D-01`): the name users write and the name the tool shows would
  differ.
- A required scope field: GSD has project decisions, quick plans and workstream-less milestones.
- A flat name plus a scope pair as the key: every map, edge and payload gains a second key.
- No root marker: an unscoped entity whose name a nearer scope reuses (a project `D-01` beside `03/D-01`) could not
  be named from inside that scope, and the shortest spelling would not be total.
- A unique suffix for every reference in a file: a bare name would link a distant entity silently, and a file's
  meaning would change when an unrelated scope gains the name.
- One `/`-joined token: the editor could not take one segment, and the lexer and the tree would disagree.
- A new code for an ambiguous reference: it names no one entity, which is E003.
- Showing the shortest spelling in reports: it depends on where the reader stands.
- Leaving references to a moved entity for the user to fix.
- `use` bindings that import names into resolution.

## Trade-offs accepted

- A reference outside its chain must be written as a path (`03/03-02` from phase 04), as GSD's model already writes
  it; E003 offers the spelling.
- A scope field is read from the project: a hand-written `phase 03` is ambiguous once two milestones have a phase
  `03`. SpecForge writes scope fields as full paths, and its create respells a hand-written short one, so only a hand
  edit meets the E003 (or writes the path once, in the file's `defaults`).

## Breaking

- Node ids, edges and every export carry `scope/name` for scoped entities; a scoped entity's diagnostics name its
  path.
- Extension protocol 1.7.0: `EntityKindDescriptor::scoped_by`; `PassEntity.id` and edges may be paths (the shared row
  also records K10's removal of r29's rule `description`).
- A new name holding `/` is refused; a `reference_path` CST node, relative or rooted.
- A bare name that is in no scope of its holder's chain no longer links: E003 with candidates. It could only link
  before when the name was project-unique, which a scoped kind is the first to break.
- An MCP `entity_id` may be a percent-encoded path; a short form that names several entities is refused,
  `ambiguous_entity`.
- `WrittenToken` gains `written` and `Form::Qualifier`; `DiagnosticData::UnresolvedReference` gains `candidates`.

## Would reopen it

- A kind scoped by a union of kinds (K3's `→ A|B`).
- A `use` binding that changes resolution.
- A need to show the shortest spelling in a report.

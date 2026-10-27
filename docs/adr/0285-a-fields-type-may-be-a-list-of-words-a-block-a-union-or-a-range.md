# A field's type may be a list of words, a block, a union or a range, and a file field declares its base

**Status:** accepted (2026-10-27, architecture round 30, plan K3). Amends [ADR 0118](0118-the-paths-a-field-names-are-read-once.md) D1
and [ADR 0242](0242-a-project-file-has-one-spelling-from-the-project-root.md) D8. Extends
[ADR 0215](0215-a-field-written-twice-is-decided-by-the-graph-build.md),
[ADR 0182](0182-the-graph-answers-what-a-blocks-field-holds.md), ADR 0093,
[ADR 0117](0117-a-reference-is-decided-in-the-linker.md) and
[ADR 0034](0034-one-field-type-read-from-the-declaration.md). Applies
[ADR 0279](0279-a-builtin-closes-a-fields-words-only-when-they-fit-any-domain.md) D1 to `enum_list` and keeps
[ADR 0282](0282-an-unknown-type-name-is-the-hosts-e004-and-a-project-has-one-spec-block.md) D1 to top-level type
expressions. Builds on [ADR 0283](0283-a-name-may-hold-a-dash-and-a-dot-where-a-name-is-expected.md) (a bare word is read by its field's declared type, members included) and [ADR 0284](0284-a-name-is-unique-within-its-scope.md) (a member reference resolves from its entity outward).

## Context

A field was a scalar, a list of strings or references, or an open block of named text entries. GSD's model needs a
list of vocabulary words, repeatable groups with typed members that hold references, enums and files, a reference to
one of several kinds, a bounded integer, and paths named from the repository root.

```text
BEFORE  reasons [threshold, jump]          string_list: any text
        requires { phase 02 provides "x" } open map: 02 names nothing
        path "src/a.ts"                    E016 from spec/
AFTER   reasons [threshold, jump]          enum_list: each a declared word, once
        requires [ { phase 02 provides "x" } ]  block_list: edge requires.phase -> v1.0/02
        path "src/a.ts"   (file_base project)   E016 from the project root
```

## Decisions

1. **`enum_list`**: a list of distinct words of the field's `enum_values`, in written order; a repeat is E061. A
   builtin declares one only for words that fit any domain (ADR 0279 D1).
2. **A typed block.** A `block` field may declare `members`, full field descriptors, recursively; each member is
   read, checked and written as a field is. A `block` without members stays an open map of named text entries.
   **`block_list`** is a list of typed blocks, written `[ { … } { … } ]`. A block has no identity.
3. **One walker** (`FieldForms::walk`) yields every typed member with its member path (`field[i].member`); coercion,
   E061, E006, W020, W159, E016, the linker, the finder, the exports and the language server read it.
4. **A reference inside a block is its entity's**: held by the entity, resolved from the entity outward, its edge
   labelled by the member path (`requires.phase`).
5. **A reference union**: `target_kinds` beside `target_kind`, exactly one of them.
6. **Integer bounds**: `minimum` and `maximum`, judged by E061. A string pattern stays the `matches` rule, whose
   `field` may name a member path.
7. **A file field declares its base**: `file_base` `spec` (absent, the spec root) or `project` (the directory of
   `specforge.json`). Every `file_exists` rule over the field reads it.
8. **Passes and validators receive a block structured** (`values`), beside its text.
9. **Typed edits address members** (`field[i].member`, 0-based). A block-list item is appended or removed by its
   own op and replaced by `set`; a word of a list (an `enum_list` too) is linked or unlinked by value. One way per
   change.

## Rejected

- Each group an entity: names nobody references.
- A repeated key for a repeatable block: reopens ADR 0215.
- A second block type beside `block`; a `many` flag instead of `block_list`.
- `target_kind` as a list, or `"A|B"` in it.
- A descriptor pattern beside the `matches` rule; bounds as rules only.
- A member edge labelled by its member name or its block field.
- One base per extension; rewriting paths at import; changing ADR 0118's default.
- Text-only pass input for blocks; whole-field edits only.

## Trade-offs accepted

- A member path in an edge label holds a `.`; field names never do, so it parses back.
- A block list item is addressed by its index: reordering items moves every address.

## Breaking

- Extension protocol minor: `enum_list`, `block_list`, `members`, `target_kinds`, `minimum`, `maximum`, `file_base`,
  `PassEntity.values`, `ValidatorEntity.values`. An older host drops a field of an unknown type (W019).
- `FieldValue::Blocks`, `ValueShape::Blocks`, a `block_list` CST node; `Input` gains `Block`/`Blocks` (an object of
  texts still reads as `Entries`); `append` and `remove` edit ops. E022 names the kinds of a union in its message.
- The `FieldJson` schema gains arrays of objects; `SchemaField` gains five attributes.

## Would reopen it

- A block that must be referenced (it becomes an entity kind).
- A block list whose items must keep their address across reorders (a declared key member, as Kubernetes'
  `list-map-keys`).
- A decimal value (a `number` type).
- A file base other than the spec and the project root.

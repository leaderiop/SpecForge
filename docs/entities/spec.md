# spec

> **Structural, not from an extension.** `spec` is one of the five keywords the core grammar knows (`spec`, `ref`,
> `use`, `define`, `defaults`); it exists in every project, whatever extensions it loads. Every other kind on these
> pages is declared by an extension ([extension model](../extension-model.md)).

## Purpose

The `spec` block names the project in the graph. It answers: **"What project is this?"**

It is a graph entity only: **it configures nothing.** No extension, provider or setting is read from it. The
project's configuration is `specforge.json`.

## Syntax

```spec
spec "clinic-operations" {
  version "0.1.0"
}
```

The name after `spec` is the block's id and title (2 to 60 characters, E014). Write one `spec` block per project,
by convention in `specforge.spec` or `main.spec` under the spec root. Fields beyond `version` are free text that no
check reads.

## Where configuration lives: `specforge.json`

`specforge init` writes it. The keys you meet most:

| Key | What it does |
|-----|--------------|
| `name`, `version` | Project identity |
| `spec_root` | Directory of the `.spec` sources (default: the project root) |
| `exclude` | Files under the spec root that are not sources |
| `extensions` | The extensions to load: a builtin or installed `@scope/name`, or a `.wasm` file. **This list decides which entity kinds exist** |
| `providers` | Ref schemes and their settings (registered only; no provider reads external data yet) |
| `registries` | Package registries for `add`, `update`, `search`, `publish` |
| `inference` | Hints for `specforge infer` |

The editor schema is [schema/specforge.schema.json](../../schema/specforge.schema.json). Change the extension
list with commands rather than by hand:

```bash
specforge init --extensions @specforge/product,@specforge/governance   # a new project
specforge add @specforge/software                                       # load one more
specforge remove @specforge/governance
specforge extensions                                                    # what is enabled and loaded
```

There is no test configuration: each collector extension (`@specforge/cargo-test`, `@specforge/vitest`) knows
where its runner's results are, and `specforge analyze coverage --min N` is the proof gate.

## Writing rules

1. **One `spec` block per project.**
2. **Never configure in it.** `extensions [...]` or `providers {...}` inside `spec` do nothing or fail to parse.
3. **Personas and channels are entities**: with `@specforge/product` loaded, write `persona` and `channel` blocks.
4. **No `define` blocks.** `define` is reserved but registers nothing (W143). A new kind comes from an extension:
   `specforge extension init` scaffolds one ([Extending SpecForge](../guides/extending-specforge.md)).

## Validation rules

| Code | Rule |
|------|------|
| E002 | A second `spec` block, whatever its name |
| E014 | The spec name is shorter than 2 or longer than 60 characters |
| E069 | `specforge.json` cannot be used as written |
| I002 | No extensions configured: structural-only mode |
| W143 | A `define` block |

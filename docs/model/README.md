# `specforge model`: the logical data model

What you get: a picture of your project's **vocabulary**, the entity kinds its extensions declare, their fields
and the relationships between them, drawn like a database schema (kinds as tables, fields as columns, edge types
as foreign keys). It is the quickest way to see what a set of extensions teaches SpecForge, and the Markdown form
is written to be read by an AI agent as well as a person.

The command has no domain knowledge: it draws whatever the loaded extensions declare. Load a different extension
and the model changes, with no change to the command.

## What it shows, and what it does not

It shows the kinds of the loaded extensions, each kind's fields (type, required, description, enum values,
default, and which extension added the field), the edge types between kinds with a cardinality, grouped by
extension.

It does not show your entities (the instances): for those use `specforge export`, `specforge query` or
`specforge trace`. It does not list validation rules: see [diagnostics.md](../diagnostics.md) and the
[entity pages](../entities/).

## Command line

```
specforge model [OPTIONS]

  --format <FORMAT>      markdown (default), mermaid, dot, json, dbml
  --group-by <GROUP_BY>  extension (default): a header per extension; none: one flat list
  --fields <FIELDS>      none, keys (default), all
  --extension <NAME>     only the kinds this extension declares
  --kinds <KINDS>        only these kinds (comma-separated, or the flag repeated)
  --root <ROOT>          only the kinds connected to this kind
  --depth <DEPTH>        how many hops from --root (needs --root)
  --path <PATH>          a path inside the project
```

The filters combine: a kind is drawn when every filter keeps it, and a relationship when both its kinds are drawn.
`--root` follows relationships in both directions; without `--depth` it keeps every kind connected to the root.
An `--extension` the project does not load, and a `--root` no loaded extension declares, are refused, naming the
closest match. A kind in `--kinds` that the project does not know is reported (I020) and selects nothing.

The MCP tool `specforge.model` takes the same parameters and calls the same operation.

## Field levels

`--fields` chooses **which** fields each kind lists; every listed field carries its full metadata.

| Level | Lists |
|-------|-------|
| `none` | No fields: kind names and relationships only, the topology |
| `keys` (default) | The synthetic `id`, every required field, every `reference` and `reference_list` field |
| `all` | Every field the kind declares, including fields other extensions add and shared fields such as `tags` |

The `id` is synthetic: it stands for the block's id (`behavior create_user`), which is not a declared field.

## Cardinality

A relationship's cardinality comes from the field that writes it:

| Source field | Cardinality |
|--------------|-------------|
| `reference_list` | N:M |
| `reference` | N:1 (1:1 when the target kind also has a single `reference` back with the same edge) |
| none on the source, a single `reference` on the target | 1:N |
| no field on either side | N:M |

## More

- [formats.md](formats.md): each output format, with real output
- [architecture.md](architecture.md): where the code is and how the model is built

# Explore the Entity Graph

The graph is SpecForge's memory: one source of truth. Your `.spec` files compile into a **typed graph** where nodes are entities and edges are the links between them. You, the editor and your AI agents all read the same graph.

## Opening the graph

Use the keyboard shortcut to open the graph visualization:

- **Windows/Linux:** `Ctrl+Shift+G`
- **macOS:** `Cmd+Shift+G`

Or run **SpecForge: Show Graph** from the Command Palette.

## What you will see

The graph starts with one group per entity kind (behaviors, features, types, and so on), each with its count:

- **Click** a group to expand its entities, and again to collapse it
- **Filter chips** at the top hide or show a kind
- **Edges** are labeled with the field that made them (`features`, `types`, `produces`, ...)

### Node interactions

- **Click** an entity to expand or collapse its neighbors
- **Double-click** to go to the entity's definition in the editor
- **Drag** to move a node

## Focus on a single entity

Place your cursor on an entity ID in the editor and press `Alt+G` (**SpecForge: Focus Entity in Graph**). The graph centers on that entity and its immediate neighbors.

## Auto-update

When `specforge.graph.autoUpdate` is enabled (the default), the graph refreshes automatically every time you save a spec file. You will see the graph evolve as you add entities and relationships.

## Other visualization formats

For documentation or external tools, use the model command:

- **SpecForge: Show Model** in the Command Palette
- Choose from: Mermaid, Markdown, DOT (Graphviz), JSON, or DBML

These are great for embedding diagrams in README files, architecture docs, or wiki pages.

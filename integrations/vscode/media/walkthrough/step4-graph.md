# Explore the Entity Graph

The graph is the core product of SpecForge. Your `.spec` files are compiled into a **typed entity graph** where nodes are entities and edges are their relationships.

## Opening the graph

Use the keyboard shortcut to open the graph visualization:

- **Windows/Linux:** `Ctrl+Shift+G`
- **macOS:** `Cmd+Shift+G`

Or run **SpecForge: Show Graph** from the Command Palette.

## What you will see

The graph shows your entire specification as an interactive visualization:

- **Nodes** are color-coded by entity kind (behaviors, features, types, etc.)
- **Edges** are labeled relationships (Implements, DependsOn, Emits, etc.)
- **Clusters** group entities by extension or module

### Node interactions

- **Click** a node to select it and see its details
- **Double-click** to navigate to the entity in the editor
- **Right-click** for context menu (inspect, trace, copy JSON)

## Layout algorithms

Choose the layout that best fits your graph:

| Layout | Best For |
|--------|---------|
| **Force** (default) | General exploration -- nodes repel, edges attract |
| **Dagre** | Hierarchical relationships -- top-to-bottom flow |
| **Concentric** | Seeing which entities are most connected |
| **Grid** | Large flat graphs with many entities |

Change the default in settings: search for `specforge.graph.defaultLayout`.

## Focus on a single entity

Place your cursor on an entity ID in the editor and press `Alt+G` to focus that entity in the graph. This highlights the entity and its immediate neighbors, dimming everything else.

## Auto-update

When `specforge.graph.autoUpdate` is enabled (the default), the graph refreshes automatically every time you save a spec file. You will see the graph evolve as you add entities and relationships.

## Other visualization formats

For documentation or external tools, use the model command:

- **SpecForge: Show Model** in the Command Palette
- Choose from: Mermaid, Markdown, DOT (Graphviz), JSON, or DBML

These are great for embedding diagrams in README files, architecture docs, or wiki pages.

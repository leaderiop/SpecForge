---
name: spec-plugins
description: "Author plugin extension point documents in a spec's plugins/ directory. Each file describes a plugin's activation, behavioral additions, and cross-references with PLG-prefixed IDs. Use when documenting extension points, plugin configurations, or behavioral additions from plugins."
---

# Spec Plugins

Rules and conventions for authoring **plugin extension point documents** in a spec's `plugins/` directory. One file per plugin describing its activation, configuration, and behavioral additions.

## When to Use

- Documenting extension points in a package
- Describing what a plugin adds when activated
- Cross-referencing plugin behaviors with core behaviors

## Directory Structure

```
plugins/
  index.yaml                    # Manifest of all plugin files
  PLG-<name>.md                 # One file per plugin
```

### index.yaml Schema

```yaml
kind: plugins
package: "@hex-di/<name>"
entries:
  - id: PLG-tracing
    file: PLG-tracing.md
    title: Tracing Plugin
    status: active              # active | draft | deprecated
    behaviors_added: ["BEH-XX-050", "BEH-XX-051"]
  - id: PLG-metrics
    file: PLG-metrics.md
    title: Metrics Plugin
    status: active
    behaviors_added: ["BEH-XX-060"]
```

**Rules:**
- Every `.md` file in `plugins/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `behaviors_added` lists the BEH IDs that this plugin introduces

## File Template

```markdown
---
id: PLG-<name>
kind: plugin
title: "<Plugin Name> Plugin"
status: active
activation: "<how to enable>"
plugin_type: "<extension|adapter|middleware>"
behaviors_added: [BEH-XX-NNN]
---

# <Plugin Name> Plugin

<Description of what the plugin adds.>

## Activation

<How to enable/configure.>

## Behavioral Additions

<What behaviors, invariants, or constraints the plugin adds when active.>

| BEH ID | Title | Description |
|--------|-------|-------------|
| BEH-XX-NNN | <Title> | <What the plugin adds> |

## Cross-References

<Links to relevant behaviors and architecture docs.>
```

## Content Rules

1. **YAML frontmatter** — Every plugin file MUST start with `---` frontmatter containing `id`, `kind: plugin`, `title`, `status`, `activation`, `plugin_type`, `behaviors_added`.
2. **Activation is clear** — Document exactly how to enable and configure the plugin.
2. **Behavioral additions are formal** — List specific BEH IDs that the plugin introduces.
3. **Cross-references are complete** — Link to architecture docs, type files, and core behaviors affected.

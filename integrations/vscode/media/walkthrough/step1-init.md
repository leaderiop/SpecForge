# Create Your First SpecForge Project

SpecForge lets you write structured `.spec` files that build **typed entity graphs** for AI agents. Instead of feeding prose to Claude or Copilot, you give them a precise graph of your system's behaviors, types, events, and relationships.

## What happens when you initialize

Running **SpecForge: Initialize Project** creates:

```
my-project/
  specforge.json          # Project config (like tsconfig.json)
  spec/
    root.spec             # Spec root block with version
    behaviors.spec        # Starter behavior entities
    types.spec            # Starter type entities
```

## Choose your extensions

SpecForge is modular. Pick the extensions that match your domain:

| Extension | Entity Kinds | Best For |
|-----------|-------------|----------|
| **@specforge/software** | behavior, invariant, event, type, port | Core software specs |
| **@specforge/product** | feature, journey, deliverable, milestone, module, term, persona, channel, release | Product planning |
| **@specforge/governance** | decision, constraint, failure_mode | Architecture decisions |
| **@specforge/formal** | property, axiom, protocol, refinement, process | Formal verification |

Most projects start with **@specforge/software** and add more as needed.

## The specforge.json config

Your project config looks like this:

```json
{
  "name": "my-project",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/software"]
}
```

You can add or remove extensions at any time using the Command Palette:
- **SpecForge: Add Extension** to install a new one
- **SpecForge: Remove Extension** to uninstall

## Three commands to value

1. `specforge init` -- create the project
2. `specforge check` -- validate your specs
3. `specforge export` -- produce structured output for AI agents

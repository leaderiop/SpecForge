# Create Your First SpecForge Project

SpecForge turns what you know (the things you work with, how they connect, and the rules they must obey) into one **checked graph** that you and your AI agents read and change. A change that breaks a rule is refused, whoever made it.

SpecForge itself knows no field. **Extensions** teach it one. Software specs are the first use, so that's where this walkthrough starts.

## What happens when you initialize

Running **SpecForge: Initialize Project** creates:

```
my-project/
  specforge.json          # Project config (like tsconfig.json)
  spec/
    hello.spec            # A first spec that checks clean
```

## Choose your extensions

Each extension adds kinds of things you can write, and the rules that check them. These ship built in:

| Extension | What it declares | Use it for |
|-----------|-------------|----------|
| **@specforge/software** | behavior, invariant, event, type, port | Software specs |
| **@specforge/product** | feature, journey, deliverable, milestone, module, term, persona, channel, release | Product planning |
| **@specforge/governance** | decision, constraint, failure_mode | Decisions and risks |
| **@specforge/formal** | property, axiom, protocol, refinement, process | Rules proved by the z3 solver |

They're a starter set, not the limit: you can write your own extension for any domain (see *Extending SpecForge* in the repository docs).

## The specforge.json config

Your project config looks like this:

```json
{
  "name": "my-project",
  "version": "0.1.0",
  "spec_root": "spec",
  "extensions": ["@specforge/software", "@specforge/testing"]
}
```

`@specforge/testing` comes with `@specforge/software`: it adds the test tracing `specforge collect` uses.

You can add or remove extensions at any time using the Command Palette:
- **SpecForge: Add Extension** to install a new one
- **SpecForge: Remove Extension** to uninstall

## Three commands to value

1. `specforge init` -- create the project
2. `specforge check` -- validate your specs
3. `specforge export` -- produce structured output for AI agents

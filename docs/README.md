# SpecForge Documentation

SpecForge turns what an organisation knows (its things, how they relate, and the rules they must obey) into one typed, validated graph that people and AI agents read and change through standard protocols. The core knows no domain: **extensions** teach it one. Software specification is one of its biggest uses, not its definition.

New here? Start with the [root README](../README.md), then [what SpecForge is for](vision/README.md) and the [use cases](vision/use-cases.md).

## Understand it

| Page | What it explains |
|---|---|
| [Vision](vision/README.md) | Why SpecForge exists: the five parts (extensions, graph, validation, surfaces, AI) and the two ways to use it, as a product or as a harness |
| [Use cases](vision/use-cases.md) | Software methods, product and marketing, regulated industries, any business: what works today and what is planned |
| [Principles](vision/principles.md) | The eight principles a design decision is tested against |
| [North star](vision/north-star.md) | The three horizons: the engine, the ecosystem, the harness |
| [Extension model](extension-model.md) | How an extension teaches SpecForge a domain: kinds, fields, edges, rules, commands, collectors, providers |
| [Entity model](entity-model.md) | The five structural keywords, the anatomy of an entity, the edge types of the builtin extensions |
| [Model and formats](model/README.md) | What `specforge model` and the exports produce ([architecture](model/architecture.md), [formats](model/formats.md)) |

## Use it

| Guide | For |
|---|---|
| [Model your domain](guides/model-your-domain.md) | Turn any domain into a project: builtin extensions or your own, then let an agent work on it |
| [Connect an agent](guides/connect-an-agent.md) | Serve a project to Claude or any MCP client; the validate-every-edit loop; LSP and VS Code |
| [Zero to hero](guides/authoring-spec-files.md) | The `.spec` language end to end, with a tour of the builtin extensions |
| [Spec writing flow](spec-writing-flow.md) | One method for writing a project with the builtin extensions, in order |
| [Cookbook](guides/spec-cookbook.md) | Task recipes: "how do I model X?" |
| [Best practices](guides/spec-best-practices.md) | Rules and named anti-patterns |
| [Troubleshooting](guides/spec-troubleshooting.md) | Diagnostic codes and their fixes |
| [Product getting started](guides/product-getting-started.md) | `@specforge/product`: personas, channels, journeys, features, milestones, releases |
| [Formal verification](guides/formal-verification.md) | Machine-checkable bounds and claims proved with z3 |
| [Rust test tracing](guides/rust-test-tracing.md), [vitest test tracing](guides/vitest-test-tracing.md) | Link tests to the obligations they prove: `specforge collect`, `analyze coverage` |
| [Demo script](demo.md) | A walkthrough on `examples/shop`, then the same engine on a product plan |

## Examples

[examples/](../examples/README.md) holds checked projects in several domains: a todo app and a shop (software), a product launch, a regulated batch-release procedure, a spec-driven development method, and a marketing plan built on a custom `@example/marketing` extension.

## Extend it

| Page | What it covers |
|---|---|
| [Extending SpecForge](guides/extending-specforge.md) | The tutorial: scaffold, declare, build and install a WASM extension |
| [Extension SDK](extension-sdk.md) | The SDK reference |
| [Extension protocol](extension-protocol.md) | The wire between the host and an extension, and how extensions reload |
| [Extension inventory](extension-inventory.md) | The builtin starter set, kind by kind |
| [Registry trust](registry-trust.md) | Publishing and installing signed extensions |

## Reference

- [Quick reference](quick-reference.md): a one-page cheat sheet of commands, the structural keywords, and every builtin kind's fields and relations (generated).
- [Entity pages](entities/): one page per builtin kind, plus the structural `spec` and `ref`.
- [Diagnostic codes](diagnostics.md): every code with its meaning, owner and fix, generated from the catalog `specforge explain <CODE>` prints.

## For contributors

How the code is built: [ARCHITECTURE.md](../ARCHITECTURE.md) and [crates/README.md](../crates/README.md). Why it is built that way: the [decision records](adr/). How to contribute: [CONTRIBUTING.md](../CONTRIBUTING.md).

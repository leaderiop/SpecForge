<p align="center">
  <img src="assets/readme/hero.svg" width="100%" alt="SpecForge. Turn your team's expertise into rules your AI can't skip. AI drafts, SpecForge checks, you sign.">
</p>

<p align="center">
  <a href="#see-it-in-60-seconds">See it in 60 seconds</a> ·
  <a href="#the-five-parts">How it works</a> ·
  <a href="#what-you-can-build-with-it">What you can build</a> ·
  <a href="docs/README.md">Docs</a>
</p>

**SpecForge turns what your organisation knows into rules your AI can't skip.**

You write down your things, how they link, and the rules they must obey. SpecForge compiles that into one typed,
checked graph. People and AI agents read it and change it through standard protocols (CLI, MCP, LSP), and a
change that breaks a rule is refused, whoever made it.

It works for software specs, product plans, quality procedures, or any field with rules worth checking. SpecForge
doesn't know your field until you install an extension that teaches it.

## The problem

AI can now do real work: write code, draft a launch plan, prepare a batch record. What it works from is the weak
part.

The knowledge that should guide it sits in places nothing enforces:

- **Documents and specs in prose** (PRDs, Markdown specs, policies, brand guides). An agent can misread a rule
  or skip it, silently. Nothing checks that the document is complete or that its parts agree.
- **Wikis and decks** drift away from reality, and nothing breaks when they do.
- **Traceability** (which requirement has a test, which procedure has an approval, which campaign has a source
  for its claim) is a spreadsheet someone updates by hand, until they stop.
- **Every session starts from zero.** The agent re-reads dozens of files to rediscover what you meant, and still
  guesses.

The more work you give to AI, the worse this gets. It produces faster than anyone can review.

This isn't a problem with the model. It's a problem with where your rules live.

## The idea: hard skills, not soft skills

There are two ways to teach an AI how your organisation works.

A **soft skill** is advice placed in the AI's context: a prompt, a Markdown spec, a policy PDF, a skill file. It
helps, but it's prose. The AI may follow it, or not.

A **hard skill** is a rule a program enforces. The AI can't skip it, because the check fails and says why.

|  | Soft skill<br>prompt, Markdown spec, policy, skill file | Hard skill<br>SpecForge extension |
|---|---|---|
| Lives in | The AI's context window, as prose | A compiler, as code |
| Can the AI ignore it? | Yes, silently | No: the check fails |
| Same verdict twice? | Not always | Always. There's no AI inside |
| Knows what's missing? | No | Yes: broken links, unenforced rules, unproven promises |
| Shared how? | Copied between files | Versioned and installed, like a library |

**SpecForge is an engine for hard skills.** Think of it as a type checker for what your organisation means.

## The five parts

| Part | What it is | In plain words |
|---|---|---|
| **Extensions** | WASM packages that declare entity kinds, their fields and links, the rules that judge them, and commands | The "apps": they teach SpecForge a domain |
| **The graph** | Every entity and link, compiled from small `.spec` files in your repository | The "memory": one source of truth |
| **Validation** | The extensions' rules plus structural checks, on every compile and every edit | The "referee": a change that breaks a rule is refused, whoever made it |
| **Surfaces** | CLI, MCP, LSP with a VS Code extension, and a package registry | The "doors" people and agents come in through |
| **AI models** | Claude or any other agent, driving SpecForge from outside, mostly over MCP | The "brain": bring your own |

<p align="center">
  <img src="assets/readme/extensions.svg" width="100%" alt="Extensions plug into the SpecForge core, which parses, links and checks and knows no domain: software, product, governance, formal, test runners, and your own, such as pharma quality, medical devices or safety cases.">
</p>

The core grammar knows five structural words: `spec`, `ref`, `use`, `define`, `defaults`. Every other word in a
spec (`behavior`, `persona`, `invariant`, `campaign`, `procedure`) and every rule ("an invariant must be enforced
by a behavior", "a journey's persona must exist") exists only because an extension declares it.

These extensions ship inside the binary, as a starter set:

| Extension | What it declares |
|---|---|
| `@specforge/software` | Behaviors, the rules they must keep (invariants), events, types, and interfaces (ports) |
| `@specforge/product` | Features, personas, channels, journeys, deliverables, milestones, releases, modules, and your team's vocabulary (terms) |
| `@specforge/governance` | Decisions, constraints, and failure modes |
| `@specforge/formal` | Properties and axioms that an SMT solver proves or refutes |
| `@specforge/testing`, `@specforge/cargo-test`, `@specforge/vitest` | Which of your tests prove which entities |
| `@specforge/rust`, `@specforge/typescript` | Source analysis that helps an agent write a spec from existing code |

Turn one on with `specforge add @specforge/product`. The full list is in the
[extension inventory](docs/extension-inventory.md). To teach SpecForge your own field, see
[Extending SpecForge](docs/guides/extending-specforge.md) and the [extension SDK](docs/extension-sdk.md).

## What you can build with it

- **Software spec frameworks.** The builtin software extensions already give agents a checked spec to build
  against, with tests that name the promise they prove. Methods like GSD, BMAD, OpenSpec and SpecKit are a
  vocabulary plus rules, so each can become an extension, with its state in checked entities instead of
  Markdown files. A GSD family (`@gsd/*`) is being designed; the others are not started.
- **Product and marketing.** Personas, channels, journeys, features and releases in one checked map that an
  agent can report from ("which channels does this persona use?", "what is unscheduled?"). Campaigns, audiences
  and claims can be added with your own extension.
- **Regulated industries.** Pharma (GxP), medical devices, safety cases: procedures, deviations and approvals
  with the rules an auditor expects, in version control and checked in CI. The AI drafts, the rules check, a
  qualified person signs. You write this extension yourself today; none ships yet.
- **Your own domain.** Any business with entities and rules worth checking. SpecForge can be the product you use,
  or the harness the business runs on: extensions as its domain, the graph as its memory, AI models as its
  workers.

The [use cases](docs/vision/use-cases.md) say what works today and what is roadmap for each, and
[examples](examples/README.md) has projects you can run.

## See it in 60 seconds

No agent needed. Install, create a product plan, then try to break it:

```bash
curl -fsSL https://raw.githubusercontent.com/leaderiop/SpecForge/main/install.sh | sh
mkdir launch-plan && cd launch-plan
specforge init --extensions @specforge/product     # a persona, a channel, a journey, a milestone
specforge link onboarding channels email           # point the journey at a channel that doesn't exist
```

```text
error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
  error[E003]: unresolved reference 'email' in entity 'onboarding'
```

The edit is refused and nothing is written. Declare the channel first, and the same edit goes through:

```bash
specforge new channel email "Email newsletter" --in spec/hello.spec --description "The monthly newsletter" --status active
specforge link onboarding channels email
specforge product persona-channels developer       # cli, email
```

An agent makes the same edits over MCP, and gets the same refusals. Hand-written files get the same check:

```bash
mkdir ../sf-demo && cd ../sf-demo
specforge init --extensions @specforge/software          # spec/hello.spec, which checks clean
sed -i.bak 's/\[user_logged_in\]/[user_loged_in]/' spec/hello.spec   # drop one letter
specforge check
```

```text
[E003] Error: unresolved reference 'user_loged_in' in entity 'authenticate_user'
 20 │   produces [user_loged_in]
    │ Help: did you mean 'user_logged_in'?

[W007] Warning: event 'user_logged_in' is not produced by any behavior

1 error, 1 warning, 0 infos
```

A reviewer reads. A check resolves. Errors always fail the check; warnings fail it too with
`specforge check --strict`, which is what you run in CI.

## Use it with your agent

**1. Install.** SpecForge is a single binary with no dependencies.

```bash
curl -fsSL https://raw.githubusercontent.com/leaderiop/SpecForge/main/install.sh | sh
```

<details>
<summary>Other ways to install</summary>

```bash
# Homebrew (once the tap is published)
brew install leaderiop/tap/specforge

# Prebuilt binary via cargo-binstall
cargo binstall --git https://github.com/leaderiop/SpecForge specforge-cli

# From source (stable Rust toolchain)
git clone https://github.com/leaderiop/SpecForge && cd SpecForge
cargo install --path crates/surfaces/cli
```

Prebuilt archives for macOS, Linux and Windows are on the [Releases page](https://github.com/leaderiop/SpecForge/releases). Formal proofs (`@specforge/formal`) also need the [`z3`](https://github.com/Z3Prover/z3) solver on your `PATH`; everything else works without it.

</details>

**2. Create a project** with the extensions your work needs:

```bash
specforge init --extensions @specforge/software,@specforge/product
```

This writes a `specforge.json` and a first file in `spec/`. In a Cargo or vitest project, it also turns on the
matching test runner.

**3. Connect your agent.** SpecForge speaks [MCP](https://modelcontextprotocol.io), so any MCP client can read,
query and edit the graph. Use an absolute path:

```bash
claude mcp add specforge -- specforge mcp /path/to/your/project
```

In Cursor, one click adds it for the open project (`${workspaceFolder}`):

[![Add SpecForge to Cursor](https://cursor.com/deeplink/mcp-install-dark.svg)](https://cursor.com/en/install-mcp?name=specforge&config=eyJjb21tYW5kIjoic3BlY2ZvcmdlIiwiYXJncyI6WyJtY3AiLCIke3dvcmtzcGFjZUZvbGRlcn0iXX0%3D)

<details>
<summary>Other MCP clients</summary>

```json
{
  "mcpServers": {
    "specforge": { "command": "specforge", "args": ["mcp", "/path/to/your/project"] }
  }
}
```

</details>

**4. Ask your agent.** For example:

> Write a spec in `spec/` for the feature I described: the behavior, the rules it must never break, the events it
> emits, and what proves each one. Validate it with the SpecForge tools and fix every error before you show it to me.

**5. Read the result yourself.**

```bash
specforge check     # is the graph consistent?
specforge review    # what is still unproven?
```

**6. Link your proof.** For software, each test names the entity and obligation it proves. Follow the
[Rust](docs/guides/rust-test-tracing.md) or [vitest](docs/guides/vitest-test-tracing.md) guide, then run
`specforge collect`. It asks once before running your test command, and records what each test proves.

## Do I have to learn a new language?

You'll want to **read** it, because the graph is what you approve. You rarely need to **write** it: your agent
writes and fixes the files, or edits them through `specforge new`, `set` and `link`, and the check tells it when
it's wrong.

It's built to be easy to read. Each block is one thing, with a kind, a name, a title, and fields, like a
filled-in form:

```spec
channel email "Email newsletter" {
  description "The monthly newsletter"
  status      active
}
```

To write specs by hand, start with the [authoring tutorial](docs/guides/authoring-spec-files.md), the
[worked example](examples/todo-app/) and [product getting started](docs/guides/product-getting-started.md).

## Surfaces

| Surface | Status | What it's for |
|---|---|---|
| CLI (`specforge`) | Shipped | Check, query, export, typed edits, CI |
| MCP server (`specforge mcp`) | Shipped | AI agents read, query and edit the graph |
| LSP + [VS Code extension](integrations/vscode/) | Shipped | Diagnostics, completion, navigation and formatting while you type |
| Package registry (`add`, `search`, `publish`) | Shipped | Install and share extensions ([trust model](docs/registry-trust.md)) |
| AG-UI | Planned | Agent-driven screens for people who don't use an editor |
| HTTP | Planned | Other applications reading and changing the graph |

Not built yet, so not claimed: connections to outside systems (a CRM, an issue tracker, a quality system), and
proof beyond test results. Extensions are pure WASM today: no files, no network.

## How is this different from…

| You already use | It's good at | What SpecForge adds |
|---|---|---|
| Specs, PRDs and policies in Markdown | Describing intent for people and agents | The same intent as `.spec` files whose structure is checked. Broken references, missing pieces and unproven promises fail the check |
| `CLAUDE.md`, `AGENTS.md` | Working rules for the agent: conventions, commands, style | Nothing to replace. Keep those rules there; what must be true lives in SpecForge |
| Tests | Proving the code does something | Saying *which* promise each test proves, and which promises have no test at all |
| Issue trackers, roadmap tools | Tracking work in progress | A checked model of what must exist and how it links, that an agent can query |
| Requirements and quality tools (DOORS, Polarion, eQMS) | Traceability for regulated teams | Plain text in your repository, checked in CI, readable by agents, extensible by you |
| Generators | Producing code or documents from a model | Nothing. SpecForge never writes your code or your documents. Your agent does, from knowledge SpecForge has checked |

## What SpecForge is not

- **Not an AI.** It makes no model calls. It's a deterministic compiler, written in Rust. The AI is yours.
- **Not a generator.** It checks knowledge. Your agent produces the work.
- **Not a test framework.** Your tests stay in `cargo test` or vitest. SpecForge runs them only after you approve
  the command, and records what they prove.
- **Not only for software.** Software is one of its biggest uses, not its definition.

## Learn more

- **[Vision](docs/vision/README.md)**: why SpecForge exists, its [principles](docs/vision/principles.md), the
  [north star](docs/vision/north-star.md) and the [use cases](docs/vision/use-cases.md).
- **[Authoring tutorial](docs/guides/authoring-spec-files.md)**: writing `.spec` files from zero, with a
  [cookbook](docs/guides/spec-cookbook.md), [best practices](docs/guides/spec-best-practices.md) and
  [troubleshooting](docs/guides/spec-troubleshooting.md).
- **[Extending SpecForge](docs/guides/extending-specforge.md)**: teach it your field, with the
  [extension SDK](docs/extension-sdk.md).
- **[Test tracing for Rust](docs/guides/rust-test-tracing.md)** and **[for vitest](docs/guides/vitest-test-tracing.md)**:
  link tests to the entities they prove.
- **[Formal verification](docs/guides/formal-verification.md)**: specs an SMT solver checks, with counterexamples
  at exact locations.
- **[Quick reference](docs/quick-reference.md)**: every command, entity kind and diagnostic code on one page.
- **[Architecture](ARCHITECTURE.md)**, **[the crate map](crates/README.md)** and **[Contributing](CONTRIBUTING.md)**:
  how SpecForge is built, what each crate owns, and how to work on it.
- **[Documentation hub](docs/README.md)**: everything else.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

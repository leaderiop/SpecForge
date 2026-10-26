# Examples

SpecForge turns what an organisation knows (its things, how they relate, the rules they must obey)
into one typed, validated graph. Extensions teach it a domain. These projects show the same engine
on different domains. Each one compiles with zero errors.

| Example | Domain | Extensions | What it shows |
|---|---|---|---|
| [todo-app](todo-app/) | software product | `software`, `testing`, `cargo-test`, `product`, `governance`, `formal` | a full software spec, from personas to behaviors and invariants, with test results collected back into the graph |
| [shop](shop/) | software (minimal) | `software`, `governance`, `testing`, `cargo-test` | the smallest loop: one type, one behavior, one invariant, and the Rust tests that prove them |
| [product-launch](product-launch/) | product management | `product`, `governance` | a team launching a desk lamp: personas, channels, journeys, features, milestones, a release, a glossary, decisions and risks; no code |
| [regulated-process](regulated-process/) | pharma / GxP (illustration) | `software`, `governance`, `formal` | releasing a batch of medicine: rules that must hold, limits checked by an SMT solver, the batch lifecycle as a state machine, failure modes; no code, no compliance claim |
| [spec-driven-method](spec-driven-method/) | software method (GSD / SpecKit style) | `software`, `testing`, `product` | milestones, phases, requirements and must-hold truths with verify obligations, applied to a link shortener |
| [marketing](marketing/) | marketing | a custom `@example/marketing` extension | a domain the builtins do not cover, taught by a project's own extension |

Extension names above drop the `@specforge/` prefix. Every builtin is a starter extension, not a part
of the core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`.

## Run one

Build the CLI from this repository, then run it from the example's folder:

```bash
cargo build --release -p specforge-cli
cd examples/product-launch
../../target/release/specforge check      # 0 errors
../../target/release/specforge stats      # entity and edge counts
../../target/release/specforge trace <entity-id>
```

Every project has the same layout: a `specforge.json` naming its extensions, a `spec/` folder of
`.spec` files and a `.gitignore` for the files SpecForge writes. The READMEs of product-launch,
regulated-process and spec-driven-method each show a small edit that breaks a rule, so you can see
the check catch it.

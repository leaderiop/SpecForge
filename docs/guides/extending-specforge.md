# Extending SpecForge — Zero to Production

A guided, end-to-end tutorial that takes you from an empty directory to a shipped,
installable extension: domain vocabulary, validation rules, and compiler passes —
all authored in Rust, compiled to Wasm, and loaded by every SpecForge surface.

This is the extension **tutorial**. Companion reading:

- **[Authoring `.spec` Files](authoring-spec-files.md)** — the spec-authoring tutorial (start there if you have never written a `.spec` file).
- **[SDK Reference](../extension-sdk.md)** — the full attribute and builder API.
- **[Extension Protocol](../extension-protocol.md)** — the wire protocol underneath: `__handshake` / `__describe`.

> 💡 **Every code block in this guide is real.** The snippets mirror
> [`fixtures/greet-extension/`](../../fixtures/greet-extension/) and
> [`extensions/formal/`](../../extensions/formal/) — extensions that ship in
> this repository and are exercised by its test suite.

---

## Table of contents

- [Act I — What an extension is](#act-i--what-an-extension-is)
  - [The bet: vocabulary over forks](#the-bet-vocabulary-over-forks)
  - [What an extension can contribute](#what-an-extension-can-contribute)
- [Act II — Scaffold](#act-ii--scaffold)
- [Act III — Vocabulary: kinds, fields, edges, rules](#act-iii--vocabulary-kinds-fields-edges-rules)
  - [A kind](#a-kind)
  - [A validation rule](#a-validation-rule)
- [Act IV — Contracts and compiler passes](#act-iv--contracts-and-compiler-passes)
  - [Structured conditions](#structured-conditions)
  - [Your first compiler pass](#your-first-compiler-pass)
  - [Ordering passes](#ordering-passes)
- [Act V — Build, install, test](#act-v--build-install-test)
  - [Build to Wasm](#build-to-wasm)
  - [Install into a project](#install-into-a-project)
  - [Test without a runtime](#test-without-a-runtime)
- [Act VI — Where to go next](#act-vi--where-to-go-next)

---

# Act I — What an extension is

## The bet: vocabulary over forks

SpecForge's built-in entity kinds — `type`, `behavior`, `feature`, `invariant`,
`event`, `port` — cover general software specification. The moment your domain
talks about something else (hardware registers, compliance controls, financial
instruments), you have three options:

1. **Shoehorn** your domain into existing kinds. Loses validation and tooling.
2. **Fork the compiler.** Loses updates, splits the ecosystem.
3. **Extend it.** Declare your vocabulary in an extension; the compiler,
   LSP, and MCP server all learn it at load time.

SpecForge is built on option 3 — the protocol is the *only* mechanism for adding
domain vocabulary. There is no bypass. The four built-in extensions
(`@specforge/product`, `@specforge/software`, `@specforge/governance`,
`@specforge/formal`) are not privileged; they ride the same protocol you are
about to use.

## What an extension can contribute

| Contribution | What it gives the graph |
|---|---|
| **Entity kinds** | New first-class nouns (`greeting`, `compliance_control`) with fields, constraints, and verify support |
| **Edges** | Typed relationships between kinds, with visual styling for diagrams |
| **Validation rules** | Structural checks the compiler runs on every build |
| **Compiler passes** | Analysis that runs during `specforge analyze`, receiving the resolved graph and returning diagnostics |
| **Feature flags** | User-configurable behavior, read from `specforge.json` |

---

# Act II — Scaffold

```console
$ specforge new --extension @you/my-ext
```

That scaffolds a ready-to-build project:

```text
my-ext/
├── Cargo.toml              # cdylib; depends on specforge-extension-sdk
├── .cargo/config.toml      # pins wasm32-unknown-unknown
└── src/lib.rs              # a working extension: builds and describes as-is
```

The scaffold compiles and installs *before you change a line* — the generated
`src/lib.rs` already declares a working kind. Treat it as a smoke test: build it
(Act V) before you start editing.

---

# Act III — Vocabulary: kinds, fields, edges, rules

An extension is one struct implementing `Contributions`. Everything is declared
through the builder passed to `contribute` — the SDK generates the protocol
exports (`__handshake`, `__describe`) from what you declare. You never write
protocol JSON by hand.

## A kind

This is the greet extension, complete:

```rust
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@you/greet",
    version = "0.1.0",
    short = "Friendly greetings"
)]
struct Greet;

impl Contributions for Greet {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("greeting", |k| {
            k.description("A friendly greeting").testable(false);
            k.field("style", |f| {
                f.field_type(FieldType::Enum);
                f.enum_values(&["warm", "formal"]);
                f.required();
            });
        });
    }
}
```

Three things happened:

1. **`greeting` is now a keyword.** Any `.spec` file in a project with this
   extension installed can write `greeting hello { style "warm" }`, and the
   parser, resolver, and validators all understand it.
2. **`style` is a constrained enum field.** The compiler rejects values outside
   `warm | formal` at compile time — before your rule engine ever runs.
3. **`.testable(false)`** tells the tooling that greetings carry no verify
   obligations, so `specforge stats` and `specforge analyze coverage` exclude
   them from coverage accounting.

Fields can also be **references** (`field_type(Reference)` / reference lists)
that create typed edges between entities, and they can carry constraints
(`required`, enums, file references).

## A validation rule

Rules are structural checks the compiler runs on every graph build:

```rust
c.rule("G101", |r| {
    r.check(CheckKind::FieldConstraint);
    r.target_kind("greeting");
    r.field("style");
    r.constraint(|fc| {
        fc.kind("field-constraint");
        fc.pattern("^(warm|formal)$");
    });
    r.severity(ValidationSeverity::Error);
    r.message_template("greeting '{id}' has unknown style");
});
```

Rules are declarative patterns — they run in-process, cost nothing at Wasm
boundaries, and are the right tool for per-field shape checks. When a check
needs the *relationships between* entities rather than the shape of one entity,
that is a compiler pass (Act IV).

---

# Act IV — Contracts and compiler passes

## Structured conditions

`@specforge/formal` extends `behavior` with contract clauses — `requires`,
`ensures`, `maintains` — that reference invariants:

```spec
behavior publish "Publish" {
  title "Publish"

  requires {
    graph_valid "the graph compiles without errors"
  }
  ensures {
    artifacts_written "output files exist on disk"
  }

  verify contract "requires/ensures consistency for publish"
}
```

These are not comments. Each clause item becomes a typed edge
(`BehaviorRequiresInvariant`, `BehaviorEnsuresInvariant`) in the graph, which
is what makes contract analysis possible.

## Your first compiler pass

A compiler pass runs during `specforge analyze`, receives a snapshot of the
resolved project, and returns diagnostics. The `#[compiler_pass]` attribute
generates the Wasm export; you write a plain function:

```rust
use specforge_extension_sdk::{
    compiler_pass, PassDiagnostic, PassEntity, PassInput,
};

#[compiler_pass(name = "condition_check", after = "resolve")]
fn pass_condition_check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut findings = Vec::new();
    for entity in &input.entities {
        if entity.kind != "behavior" {
            continue;
        }
        let requires = non_empty(entity, "requires");
        let ensures = non_empty(entity, "ensures");
        if requires && !ensures {
            findings.push(
                PassDiagnostic::warning(
                    "W096",
                    format!(
                        "behavior '{}' declares requires but no ensures",
                        entity.id
                    ),
                )
                .with_suggestion(
                    "add an ensures clause: a caller's obligation must buy a guarantee",
                ),
            );
        }
    }
    findings
}

fn non_empty(entity: &PassEntity, field: &str) -> bool {
    entity
        .fields
        .get(field)
        .is_some_and(|v| !v.trim().is_empty())
}
```

Declare the pass so the host knows it exists:

```rust
c.pass("condition_check", |p| {
    p.after("resolve");
});
```

Then run it:

```console
$ specforge analyze --json
```

Design notes, so your passes age well:

- **Passes are pure.** Same snapshot in, same diagnostics out. No IO, no
  clocks. This is what makes them testable without a Wasm runtime and safe to
  reorder.
- **Prefer structural checks.** The v1 pass ABI hands you an entity snapshot
  (id, kind, stringified fields, edge counts) plus the resolved edge list —
  checks that reason about *structure* (symmetry, presence, cycles, coverage)
  are its sweet spot. Semantic verification (SMT-backed `analyze --prove`) is
  a separate, future rung of the formality ladder.
- **Codes are yours.** Pick a prefix your extension owns and stay consistent;
  document your codes the way `@specforge/formal` does
  ([Entity Model → diagnostics](../entity-model.md)).

## Ordering passes

`after` / `before` name other passes. The host topologically sorts each
extension's passes by these constraints before running them — declaration
order in the source file does not matter. Constraints naming unknown passes
(host phases like `"resolve"`, or other extensions' passes) are ignored.
Constraint cycles fall back to declaration order with a warning; don't rely
on that — cycles are bugs.

---

# Act V — Build, install, test

## Build to Wasm

```console
$ cargo build --release --target wasm32-unknown-unknown
```

The scaffold's `.cargo/config.toml` already pins the target, so plain
`cargo build --release` works too. The artifact lands in
`target/wasm32-unknown-unknown/release/<your_ext>.wasm`.

## Install into a project

```console
$ specforge add ./target/wasm32-unknown-unknown/release/my-ext.wasm
```

`specforge add` copies the module into the project and registers it in
`specforge.json`. From that moment, `check`, `analyze`, `watch`, the LSP, and
the MCP server all load your vocabulary. Verify with a round trip:

```console
$ specforge check          # your validation rules run
$ specforge analyze        # your compiler passes run
$ specforge outline        # your kinds render
```

## Test without a runtime

The SDK's describe output is plain data — build your contributions in a unit
test and assert on them. No Wasm runtime, no SpecForge binary:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_greeting_kind() {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(
            "@you/greet",
            "0.1.0",
        ));
        Greet::contribute(&mut c);

        let describe = c.describe_response_json("entities").unwrap();
        assert!(describe.contains("\"greeting\""));
    }
}
```

The shipped extensions test exactly this way — see the `raw_category_flag_tests`
module in the SDK and the pass tests in `extensions/formal/src/lib.rs`.

---

# Act VI — Where to go next

- **Host API (v2 pass ABI).** Passes currently receive an entity snapshot and
  return diagnostics. The next ABI revision adds host-query functions —
  `query`, `read_file`, `emit_diagnostic` — so passes can pull data on demand
  instead of receiving it all up front.
- **`specforge analyze --prove`.** The formal ladder's top rung: contracts
  become verification conditions discharged by an SMT backend. Design
  groundwork is in [RES-25](../../spec/research/RES-25-sources/).
- **Publish.** `cargo publish` your SDK extension crate; the SDK itself
  (`specforge-extension-sdk`) is on crates.io.

The source of truth for everything in this guide:
[`docs/extension-sdk.md`](../extension-sdk.md) and
[`docs/extension-protocol.md`](../extension-protocol.md).

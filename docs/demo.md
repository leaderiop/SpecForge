# Demo script

A 12-minute walkthrough of what SpecForge does: knowledge written as a spec,
checked like code, handed to an agent as a small graph, refused when an edit
breaks a rule, and proven by the project's own tests. Steps 1 to 5 run on
`examples/shop`, a shopping cart with one spec file and one Rust file: the
software use case, with the builtin `@specforge/software` extension. Step 6
shows the same engine on a product plan, to make the point that SpecForge is
not tied to software. Every output below was recorded from a real run.

## Before the demo

- Install the binary (about 10 minutes): `cargo install --path crates/surfaces/cli`.
- Warm the demo crate so `cargo test` doesn't compile on stage:
  `cd examples/shop && cargo test -q`.
- To show the approval prompt, make sure the project isn't approved yet:
  remove its entry from `~/.specforge/collector-consent.json`, or run the demo
  with `SPECFORGE_CONSENT_FILE=/tmp/demo-consent.json`.
- Delete any old `examples/shop/specforge-report.json` (it is git-ignored);
  a report written by an older version is refused with E045 until
  `specforge collect` rewrites it.
- Keep `examples/shop/src/lib.rs` as committed: its last obligation is
  deliberately untested.

## 1. The spec (1 minute)

`cd examples/shop` and open `spec/cart.spec`. Point at:

- entities are typed: a `type`, a `behavior`, an `invariant`;
- `add_item` references `cart` and `no_duplicate_items`: the spec is a graph;
- each `verify` line is an obligation a test has to prove.

## 2. Checked like code (1 minute)

```console
$ specforge check
0 errors, 0 warnings, 0 infos
```

Break a reference (`invariants [no_duplicate_item]`) and check again:

```console
$ specforge check
[E003] Error: unresolved reference 'no_duplicate_item' in entity 'add_item'
    │ Help: did you mean 'no_duplicate_items'?
[W003] Warning: invariant 'no_duplicate_items' is not enforced by any behavior
```

Undo the typo. `specforge format` keeps the files in one layout.

## 3. What an agent gets (2 minutes)

```console
$ specforge export --format context
{"schema_version":"2.0.0","nodes":[{"id":"add_item","kind":"behavior","title":"Add an item to the cart","contract":"Adding a named item puts it in the cart once","verify":[...]}, ...],"edges":[...]}
```

About 1,000 bytes for the whole project: every entity's statements, members,
obligations and links, instead of an agent reading files to reconstruct them.

A person who signs the specification reads a document, not JSON.
`specforge export --format markdown` writes one section per entity, with its
statements quoted, its links named by kind and title, what references it and
its numbered obligations:

```console
$ specforge export --format markdown
# Specification

4 entities.

## behavior `add_item` — Add an item to the cart

`spec/cart.spec:12`

**contract**

> Adding a named item puts it in the cart once
```

`specforge trace add_item` shows the neighbourhood of one entity:

```console
$ specforge trace add_item
  "downstream": [
    { "entity_id": "cart", "entity_kind": "type", "edge_label": "types", "depth": 1 },
    { "entity_id": "no_duplicate_items", "entity_kind": "invariant", "edge_label": "invariants", "depth": 1 }
  ]
```

With Claude Code: `claude mcp add specforge -- specforge mcp $PWD`, then ask
"what does add_item promise, and which obligations are untested?". The agent
answers from the graph through SpecForge's MCP tools.

## 4. Proven by the tests (3 minutes)

Open `src/lib.rs`: each test names the obligation it proves.

```rust
#[specforge_test(behavior = "add_item", verify = "rejects an empty name")]
fn rejects_an_empty_name() { ... }
```

```console
$ specforge collect
@specforge/cargo-test wants to run this command in .../examples/shop:
  cargo test --workspace --no-fail-fast
Allow it for this project? [y/N] y
...
cargo-test: 3 entities, 4 passed, 0 failed, 0 skipped (1 report file(s))
report written: .../examples/shop/specforge-report.json
```

SpecForge never runs anything by itself: the runner extension declares the
command, and you approve it once per project.

```console
$ specforge analyze coverage
[A015] Warning: behavior 'add_item' has 1 obligation(s) no passing test proves: "rejects a duplicate item"
  summary:
    discharge_funnel.entities_proven: 2
    test_results.obligations_proven: 4
    ...
```

The spec promised "rejects a duplicate item"; no test proves it.

## 5. Close the gap (2 minutes)

Add the test:

```rust
#[specforge_test(behavior = "add_item", verify = "rejects a duplicate item")]
fn rejects_a_duplicate_item() {
    let mut cart = vec!["apple".to_string()];
    assert!(add(&mut cart, "apple").is_err());
}
```

```console
$ specforge collect
cargo-test: 3 entities, 5 passed, 0 failed, 0 skipped (1 report file(s))
$ specforge analyze coverage
    discharge_funnel.entities_proven: 3
    test_results.obligations_proven: 5
0 errors, 0 warnings, 0 infos
```

Every obligation is backed by a passing test. A failing test shows up as
A014, and a test whose `verify` text doesn't match the spec as A016.

## 6. Same engine, another field (2 minutes)

Nothing above is built into SpecForge: `behavior`, `invariant` and `verify`
come from the `@specforge/software` extension. Load a different one and the
same checks apply to a different field. In an empty directory:

```console
$ specforge init --extensions @specforge/product
$ specforge link onboarding channels email
error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
  error[E003]: unresolved reference 'email' in entity 'onboarding'
```

The journey can't name a channel that doesn't exist, and the edit is refused
before anything is written. An agent editing over MCP gets the same refusal.
Declare the channel and the edit goes through:

```console
$ specforge new channel email "Email newsletter" --in spec/hello.spec --description "The monthly newsletter" --status active
$ specforge link onboarding channels email
$ specforge product persona-channels developer
Channels of persona 'developer':
  cli
  email
```

## Afterwards

`git checkout examples/shop/src/lib.rs` restores the untested obligation for
the next run.

## Keep out of a demo (for now)

- Windows: not verified yet.
- jest, pytest and Playwright projects: no runner extension yet (vitest and
  cargo are supported).
- A browser screen for business users, or a live connection to a CRM or a
  quality system: neither exists yet (AG-UI and HTTP surfaces are planned).
- Proof other than test results (a signed review, an executed procedure):
  future work.
- `specforge analyze` on SpecForge's own spec: it honestly lists over a
  thousand obligations without a test, which reads badly out of context.

# specforge-extension-sdk

Author [SpecForge](https://github.com/leaderiop/SpecForge) extensions in Rust.
SpecForge is a domain-free engine: it compiles `.spec` files into one typed,
validated graph, and every kind of entity in that graph comes from an
extension. An extension teaches it a domain — software, product planning, a
regulated procedure, anything with things, links and rules worth checking.

Declare **what your extension contributes**: entity kinds, fields, edges,
validation rules, compiler passes, collectors, analyzers, feature flags and
commands, each operation together with the function that answers it. The SDK
serves the protocol the SpecForge host loads (`__handshake`, `__describe`) and
routes every export it calls to the handler you declared.

An extension is a `wasm32-wasip2` component, loaded by the SpecForge CLI, the
LSP and the MCP server.

## Quick start

Scaffold a crate with `specforge extension init --name @you/my-ext`, or write
it yourself:

```toml
# Cargo.toml
[package]
name = "my-ext"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
specforge-extension-sdk = "0.3.0"
serde_json = "1.0"
```

```toml
# .cargo/config.toml
[build]
target = "wasm32-wasip2"
```

```rust
// src/lib.rs
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@you/my-ext",
    version = "0.1.0",
    short = "my-ext",
    description = "One line saying what the extension is for"
)]
struct Extension;

/// The greeting styles.
const STYLES: Vocabulary = Vocabulary::new(&["warm", "formal"]);

impl Contributions for Extension {
    fn contribute(c: &mut ContributionsBuilder) {
        c.kind("greeting", |k| {
            k.description("A friendly greeting").testable(false);
            k.field("style", |f| {
                f.vocabulary(&STYLES).required();
            });
        });
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);
```

```console
$ cargo build --release
$ specforge extension validate
$ specforge add ./path/to/my-ext     # the crate directory, from your project
```

`short` names the extension's commands (`specforge my-ext <command>`) and MCP
tools (`specforge.my-ext.<command>`): lowercase kebab case, checked when the
crate compiles.

## What the SDK guarantees

- **Wire types are shared with the host** (`specforge-protocol-types`), so the
  protocol cannot drift between your extension and SpecForge.
- **The declaration is the package**: what the builder builds is what the host
  loads and what `specforge publish` uploads; there is no manifest to keep in
  step.
- **An operation is declared with its handler** (`command`, `mcp_tool`,
  `mcp_resource`, `pass`, `collector`, `rule` with `validate`, `analyzer` with
  `scan`, `migration_hook_handler`): the declaration and the export routing come
  from that one call, and declaring one without its handler panics when the
  extension is built.
- **Runtime-free testing**: build your contributions in a unit test and assert
  on the describe output (the `testing` module, feature `testing`; enable it in
  `[dev-dependencies]` only).

## Not modeled yet?

Categories the builders do not cover can be served with
`ContributionsBuilder::raw_category(DeclaredCategory::..., items)`; exports no builder
declares go to `component_guest!`'s `handler`, which can decode and encode with
`answer_export`.

## Host functions

None: a guest is pure compute. Everything it needs comes in its call's input,
and its answer is the only channel back. It has no file system, environment or
network, so an extension cannot reach a live business system; a typed
host-import surface is planned.

## License

MIT OR Apache-2.0

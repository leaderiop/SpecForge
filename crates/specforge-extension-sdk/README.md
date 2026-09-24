# specforge-extension-sdk

Author [SpecForge](https://github.com/leaderiop/SpecForge) extensions in Rust.
Declare **what your extension contributes** — entity kinds, fields, edges,
validation rules, compiler passes, feature flags — and the SDK generates every
protocol export the SpecForge host loads: `__handshake` and `__describe`
(handshake/describe protocol v1.0.0).

Your extension compiles to a Wasm module (`wasm32-unknown-unknown`) and is
loaded by the SpecForge CLI, LSP, and MCP server.

## Quick start

```toml
# Cargo.toml
[package]
name = "my-ext"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
specforge-extension-sdk = "0.1"
extism-pdk = "1.4.1"
```

```rust
// src/lib.rs
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@you/my-ext",
    version = "0.1.0",
    short = "One-line description"
)]
struct Extension;

impl Contributions for Extension {
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

```console
$ cargo build --release --target wasm32-unknown-unknown
$ specforge add ./target/wasm32-unknown-unknown/release/
```

Scaffold the whole project with:

```console
$ specforge new --extension @you/my-ext
```

## What the SDK guarantees

- **Wire types are shared with the host** (`specforge-protocol-types`), so the
  protocol cannot drift between your extension and SpecForge.
- **Contribution flags are derived** from what you actually contribute — they
  cannot contradict your content.
- **Runtime-free testing**: build your contributions in a unit test and assert
  on the describe output (see the `testing` module).

## Not modeled yet?

Categories the typed builders do not cover can still be served through
`ContributionsBuilder::raw_category(category, items)` — the SDK raises the
right handshake flags for them.

## Host API

Extensions call back into the host through three functions
(`specforge_extension_sdk::host`): `host_query_graph`, `host_read_file`, and
`host_emit_diagnostic`.

## License

MIT OR Apache-2.0

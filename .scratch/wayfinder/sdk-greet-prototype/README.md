# PROTOTYPE: the SDK developer experience (wayfinder map #1, ticket #6)

A cheap, concrete mock of what authoring a third-party extension would feel
like with `specforge-extension-sdk` (decided in ticket #2). Nothing here runs.

## The imagined flow

    # 1. scaffold (later: `specforge new --extension greet`)
    cargo new greet --lib && cd greet
    # deps: specforge-extension-sdk (path/registry); target pinned in .cargo/config.toml

    # 2. write the extension (see src/lib.rs — 16 lines, zero JSON, zero boilerplate)

    # 3. build (target is pinned: no --target flag to remember; ticket #3 decision)
    cargo build --release

    # 4. install locally (ticket #5 decision: local paths only in v1)
    specforge add ./target/wasm32-unknown-unknown/release/greet.wasm

    # 5. use it — spec files can now declare `greeting` blocks:
    #    greeting hello_warm "A warm hello" { style warm }

## React to

- The `#[specforge::extension]` + `Contributions` shape (sdk/src/lib.rs)
- Builders vs a JSON DSL vs proc-macro-everything
- `FieldType::Enum` inline values vs separate enum descriptors
- The `testing::MockHost` idea (runtime-free describe tests)

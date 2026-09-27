# W0 spike — Component Model viability (hardening-plan Phase W0)

Bench: typed WIT component guest (wasm32-wasip2, wit-bindgen 0.30) hosted
by wasmtime 49 `component::bindgen!` vs the current extism 1.30 byte-array
path (wasmtime 43).

Results (M3 Pro, 2026-09-27):

    COMPONENT tiny_call_us=0.31 batch_1_5mb_call_us=20.39
    EXTISM     tiny_call_us=1056.51

- Component boundary: 0.31 us/call floor; a 1.5 MB batch call costs 20 us
  (single linear-memory copy) — batch-shaped world functions scale.
- extism leg cost is dominated by guest-side JSON serialization through the
  PDK (~1 ms for a handshake response) — exactly the cost a typed WIT
  boundary removes.
- Registry compatibility: wasip2 component blobs keep the `\0asm` magic
  (layer byte = 1) and sha256 addressing, so the existing publish/download/
  integrity gates are unaffected.

## Decision (gate met)

GO — Component Model / WIT for C7-03:
- per-call overhead well under the 2x gate (it is lower than extism's)
- registry gates unaffected
- SDK delta confined to the bindings layer (wit-bindgen generate + export
  macro replace the extism-pdk plugin_fn glue inside specforge-extension-sdk;
  ContributionsBuilder business logic is unchanged)

Run it yourself:

    cd spike/w0-component/guest && cargo build --release --target wasm32-wasip2
    cd ../host && cargo build --release
    ./target/release/w0-host ../../extensions/product/wasm/specforge_ext_product.wasm

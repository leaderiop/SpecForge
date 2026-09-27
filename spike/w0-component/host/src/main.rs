use anyhow::{Context, Result};
use std::time::Instant;

mod component_host {
    use anyhow::{Context, Result};
    use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
    use wasmtime::component::{Component, Linker};
    use wasmtime::{Config, Engine, Store};

    wasmtime::component::bindgen!({
        world: "extension",
        path: "../guest/wit",
    });

    pub struct HostState {
        wasi: WasiCtx,
        table: wasmtime_wasi::ResourceTable,
    }

    impl WasiView for HostState {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }

    pub struct Bench {
        store: Store<HostState>,
        bindings: Extension,
    }

    impl Bench {
        pub fn new(path: &str) -> Result<Self> {
            let mut config = Config::new();
            config.wasm_component_model(true);
            let engine = Engine::new(&config)?;
            let component = Component::from_file(&engine, path)?;
            let mut linker: Linker<HostState> = Linker::new(&engine);
            wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
            let table = wasmtime_wasi::ResourceTable::new();
            let wasi = wasmtime_wasi::WasiCtx::builder().build();
            let mut store = Store::new(&engine, HostState { wasi, table });
            let bindings = Extension::instantiate(&mut store, &component, &linker)?;
            Ok(Self { store, bindings })
        }

        pub fn ping(&mut self) -> Result<String> {
            Ok(self.bindings.call_ping(&mut self.store)?)
        }

        pub fn pass(&mut self, payload: &str) -> Result<String> {
            // wit result<string,string> flattens to a Result in Rust
            let out = self.bindings.call_pass(&mut self.store, payload)?;
            out.map_err(|e| anyhow::anyhow!("guest trap/error: {e}"))
        }
    }
}

fn main() -> Result<()> {
    let component_path = std::fs::canonicalize("../guest/target/wasm32-wasip2/release/w0_guest.wasm")
        .unwrap_or_else(|e| panic!("component path missing: {e}"))
        .to_str()
        .unwrap()
        .to_string();
    let extism_blob = std::env::args()
        .nth(1)
        .expect("pass vendored extism blob path (for the extism leg)");

    let big = "x".repeat(1_500_000);

    // ── component leg (wasmtime 49) ──
    let mut bench = component_host::Bench::new(&component_path).context("component leg")?;
    assert_eq!(bench.ping()?, "pong");
    assert!(bench.pass(&big)?.contains("len=1500000"));

    let n = 10_000;
    let start = Instant::now();
    for _ in 0..n {
        let _ = bench.ping()?;
    }
    let comp_tiny_us = start.elapsed().as_nanos() as f64 / n as f64 / 1_000.0;

    let m = 500;
    let start = Instant::now();
    for _ in 0..m {
        let _ = bench.pass(&big)?;
    }
    let comp_big_us = start.elapsed().as_nanos() as f64 / m as f64 / 1_000.0;

    // ── extism leg (wasmtime 43, the current production path) ──
    let wasm_bytes = std::fs::read(&extism_blob).context("read extism blob")?;
    let manifest = extism::Manifest::new([extism::Wasm::data(wasm_bytes)]);
    let mut plugin = extism::PluginBuilder::new(manifest)
        .with_wasi(true)
        .build()
        .context("extism plugin build")?;

    let hs = plugin.call::<&[u8], Vec<u8>>("__handshake", b"")?;
    assert!(hs.starts_with(b"{"));

    let start = Instant::now();
    for _ in 0..n {
        let _ = plugin.call::<&[u8], Vec<u8>>("__handshake", b"")?;
    }
    let ext_tiny_us = start.elapsed().as_nanos() as f64 / n as f64 / 1_000.0;

    println!(
        "COMPONENT tiny_call_us={:.2} batch_1_5mb_call_us={:.2}",
        comp_tiny_us, comp_big_us
    );
    println!("EXTISM     tiny_call_us={:.2}", ext_tiny_us);
    Ok(())
}

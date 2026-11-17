//! Wasmtime on-disk compile cache wiring (C7-02 closeout).
//!
//! Proves the cache is configured at engine construction and actually
//! populates on first compile — the previous `.aot` side cache was a byte
//! copy no runtime ever consumed.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::ComponentRuntime;
use specforge_wasm::runtime::WasmRuntime;

fn component_path() -> std::path::PathBuf {
    specforge_component::committed::BRIDGE.path()
}

#[test]
fn compile_cache_dir_is_populated_on_first_load() {
    let dir = tempfile::TempDir::new().unwrap();
    let cache_dir = dir.path().join("cache");

    let runtime = ComponentRuntime::new_with_compile_cache(cache_dir.clone());
    let bytes = std::fs::read(component_path()).expect("fixture component bytes");
    runtime
        .load(
            "@test/component",
            &bytes,
            specforge_wasm::Provenance::third_party(&bytes),
        )
        .expect("fixture component compiles");
    let entries: Vec<_> = std::fs::read_dir(&cache_dir)
        .expect("cache dir exists")
        .collect();
    assert!(
        !entries.is_empty(),
        "compile cache must be populated by the first component compile"
    );
}

#[test]
fn second_runtime_reuses_cache_and_still_loads() {
    let dir = tempfile::TempDir::new().unwrap();
    let cache_dir = dir.path().join("cache");

    let first = ComponentRuntime::new_with_compile_cache(cache_dir.clone());
    let bytes = std::fs::read(component_path()).expect("fixture component bytes");
    first
        .load(
            "@test/component",
            &bytes,
            specforge_wasm::Provenance::third_party(&bytes),
        )
        .expect("first compile");
    // A fresh engine over the same cache dir exercises the deserialize path.
    let second = ComponentRuntime::new_with_compile_cache(cache_dir);
    second
        .load(
            "@test/component",
            &bytes,
            specforge_wasm::Provenance::third_party(&bytes),
        )
        .expect("cache-hit load");
    let result = second.call_export("@test/component", "__handshake", b"");
    match result {
        specforge_wasm::runtime::WasmCallResult::Ok(bytes) => {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("\"name\":\"@test/component\""), "got: {text}");
        }
        specforge_wasm::runtime::WasmCallResult::Trap(t) => panic!("handshake trapped: {t:?}"),
    }
}

#[test]
fn unwritable_cache_dir_degrades_to_no_cache_with_warning() {
    // A path under a *file* cannot be created — the runtime must still work.
    let file = tempfile::TempDir::new().unwrap();
    let blocker = file.path().join("blocker");
    std::fs::write(&blocker, b"not a dir").unwrap();

    let runtime = ComponentRuntime::new_with_compile_cache(blocker.join("cache"));
    let bytes = std::fs::read(component_path()).expect("fixture component bytes");
    runtime
        .load(
            "@test/component",
            &bytes,
            specforge_wasm::Provenance::third_party(&bytes),
        )
        .expect("component compiles even when cache setup fails");
}

/// Milliseconds, for the printed runs.
fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// The median of an even number of runs: the mean of the two middle ones.
fn median(mut runs: Vec<std::time::Duration>) -> std::time::Duration {
    runs.sort();
    let mid = runs.len() / 2;
    (runs[mid - 1] + runs[mid]) / 2
}

/// `@gsd/core`'s cached load, what `wasm_compile_cache_budget`'s `cached_extension_load` measures (ADR 0373 D3): the
/// in-process load of a compile-cache hit plus the handshake and describe calls that declare the extension, which is
/// what the host pays for it. One cold compile into an empty cache directory, then eight fresh runtimes over that
/// directory, each timed for its `load` and its declaration apart; every run and the medians are printed. No gate runs
/// it: a wall-clock bound flakes on a shared runner.
#[test]
#[ignore = "a wall-clock measure of the cached load, ADR 0373 D3: run it alone on a quiet machine"]
#[expect(
    clippy::print_stdout,
    reason = "the measure prints every run, read with --no-capture"
)]
fn gsd_core_loads_from_the_compile_cache_in_under_fifty_ms() {
    use specforge_component::builtins::BUILTINS;
    use std::time::{Duration, Instant};
    const RUNS: usize = 8;
    const BOUND: Duration = Duration::from_millis(50);
    let gsd = BUILTINS
        .iter()
        .find(|builtin| builtin.name == "@gsd/core")
        .expect("the @gsd/core row");
    let dir = tempfile::TempDir::new().unwrap();
    let cache_dir = dir.path().join("cache");
    let load = |runtime: &ComponentRuntime| {
        runtime
            .load(
                gsd.name,
                gsd.bytes,
                specforge_wasm::Provenance::builtin(gsd.bytes),
            )
            .expect("@gsd/core loads");
    };
    let cold = Instant::now();
    load(&ComponentRuntime::new_with_compile_cache(cache_dir.clone()));
    println!("cold compile {:.2} ms", ms(cold.elapsed()));
    let (mut loads, mut declarations, mut totals) = (Vec::new(), Vec::new(), Vec::new());
    for run in 1..=RUNS {
        let runtime = ComponentRuntime::new_with_compile_cache(cache_dir.clone());
        let start = Instant::now();
        load(&runtime);
        let loaded = start.elapsed();
        let start = Instant::now();
        specforge_wasm::protocol::load_declaration(&runtime, gsd.name).expect("@gsd/core declares");
        let declared = start.elapsed();
        println!(
            "run {run}: load {:.2} ms, declaration {:.2} ms, total {:.2} ms",
            ms(loaded),
            ms(declared),
            ms(loaded + declared)
        );
        loads.push(loaded);
        declarations.push(declared);
        totals.push(loaded + declared);
    }
    let total = median(totals);
    println!(
        "median of {RUNS}: load {:.2} ms, declaration {:.2} ms, total {:.2} ms",
        ms(median(loads)),
        ms(median(declarations)),
        ms(total)
    );
    assert!(
        total < BOUND,
        "@gsd/core's cached load is {:.2} ms, over the {} ms of wasm_compile_cache_budget",
        ms(total),
        BOUND.as_millis()
    );
}

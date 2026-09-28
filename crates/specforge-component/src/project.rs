//! Project-scoped runtime construction — the single constructor every
//! surface (CLI, LSP, MCP) uses to load a project's extensions.
//!
//! Consolidating construction here is the Phase 3 step of the WASM-only
//! migration (`.plugin/migration-plan.md`): one engine per process/session,
//! embedded builtin blobs for `@specforge/*` names, and third-party `.wasm`
//! paths from `specforge.json`, with identical semantics everywhere.

use crate::{ComponentRuntime, builtins};
use std::path::{Path, PathBuf};

/// Build the Wasm runtime for a project.
///
/// Only extensions listed in `specforge.json` are loaded — no implicit
/// builtins. Builtin extensions are loaded from embedded Wasm binaries when
/// their name matches a `@specforge/*` builtin. Custom `.wasm` paths are
/// loaded from disk (`"name=path.wasm"` or bare `"path.wasm"`).
pub fn project_runtime(path: &Path) -> ComponentRuntime {
    let config = specforge_common::load_project_config(path);

    // The compile cache must be selected at construction — wasmtime reads it
    // when the Engine is built. Builtin loads below populate it on first use.
    let runtime = match user_compile_cache_dir() {
        Some(dir) => ComponentRuntime::new_with_compile_cache(dir),
        None => ComponentRuntime::new(),
    };

    builtins::load_builtins_for(&runtime, &config.extensions)
        .expect("failed to load builtin extensions");

    // Load any additional Wasm extensions from project config
    for ext in &config.extensions {
        if ext.ends_with(".wasm") {
            let (name, wasm_path) = if let Some((n, p)) = ext.split_once('=') {
                (n, p)
            } else {
                let stem = Path::new(ext.as_str())
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(ext.as_str());
                (stem, ext.as_str())
            };
            let resolved = if Path::new(wasm_path).is_relative() {
                path.join(wasm_path)
            } else {
                Path::new(wasm_path).to_path_buf()
            };
            if let Err(e) = runtime.load_module_as(name, &resolved) {
                eprintln!("warning: failed to load Wasm extension '{name}': {e}");
            }
        }
    }

    runtime
}

/// Per-user Wasmtime compilation cache directory.
///
/// `$SPECFORGE_WASMTIME_CACHE` overrides; setting it to `off` disables the
/// cache. Default: `$HOME/.cache/specforge/wasmtime` (falls back to the
/// system temp dir when `HOME` is unset).
fn user_compile_cache_dir() -> Option<PathBuf> {
    match std::env::var_os("SPECFORGE_WASMTIME_CACHE") {
        Some(v) if v == "off" => None,
        Some(v) => Some(PathBuf::from(v)),
        None => std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".cache")
                .join("specforge")
                .join("wasmtime")
        }),
    }
}

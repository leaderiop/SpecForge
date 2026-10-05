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

    // Installed extensions load from the lock file (ADR 0004 D3-b).
    let lock = specforge_wasm::read_lock_file(&path.join("specforge.lock")).ok();
    for ext in &config.extensions {
        let name = specforge_common::extension_entry_name(ext);
        if ext.ends_with(".wasm") || builtins::is_builtin(name) {
            continue;
        }
        if let Err(diagnostic) = load_installed(&runtime, path, name, lock.as_ref()) {
            runtime.record_load_failure(name, diagnostic);
        }
    }

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

/// Load the installed extension `name` from
/// `.specforge/extensions/<name>/extension.wasm` under the name itself,
/// refusing a binary whose hash is not the one its lock entry records
/// (E033). Not in the lock, or no binary: E028, with the command that
/// installs it.
fn load_installed(
    runtime: &ComponentRuntime,
    root: &Path,
    name: &str,
    lock: Option<&specforge_wasm::LockFile>,
) -> Result<(), specforge_common::Diagnostic> {
    let Some(entry) = lock.and_then(|lock| lock.entries.iter().find(|e| e.name == name)) else {
        return Err(specforge_common::Diagnostic {
            code: "E028".to_string(),
            severity: specforge_common::Severity::Error,
            message: format!(
                "extension '{name}' is enabled in specforge.json but not installed (no specforge.lock entry)"
            ),
            span: None,
            suggestion: Some(format!("install it with: specforge add {name}")),
            data: None,
        });
    };
    let wasm = specforge_wasm::installed_wasm_path(&root.join(".specforge/extensions"), name);
    specforge_wasm::load_wasm_module(name, &wasm, runtime, Some(&entry.wasm_hash)).map(|_| ())
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

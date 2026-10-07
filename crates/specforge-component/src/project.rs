//! Project-scoped runtime construction — the single constructor every
//! surface (CLI, LSP, MCP) uses to load a project's extensions.
//!
//! Consolidating construction here is the Phase 3 step of the WASM-only
//! migration (`.plugin/migration-plan.md`): one engine per process/session,
//! embedded builtin blobs for `@specforge/*` names, and third-party `.wasm`
//! paths from `specforge.json`, with identical semantics everywhere.

use crate::{ComponentRuntime, builtins};
use specforge_common::{ExtensionEntry, codes};
use std::path::{Path, PathBuf};

/// Build the Wasm runtime for a project.
///
/// Only extensions listed in `specforge.json` are loaded — no implicit
/// builtins. Each entry is read by [`ExtensionEntry`], the rule the
/// environment reads it by too: a builtin loads from its embedded binary,
/// an installed extension from `.specforge/extensions` (pinned by
/// `specforge.lock`), and a `.wasm` file entry (`path.wasm` or
/// `name=path.wasm`) from that file, under the name its component
/// declares. What does not load is recorded as a load failure (E028/E033)
/// the environment reports.
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
    let lock = specforge_wasm::LockState::at(path);
    for ext in &config.extensions {
        let ExtensionEntry::Named(name) = ExtensionEntry::parse(ext) else {
            continue;
        };
        if builtins::is_builtin(name) {
            continue;
        }
        if let Err(diagnostic) = load_installed(&runtime, path, name, lock.file()) {
            runtime.record_load_failure(name, diagnostic);
        }
    }

    // `.wasm` files, once every named extension is loaded: one that
    // declares a name already loaded is refused, not swapped in.
    for ext in &config.extensions {
        let entry = ExtensionEntry::parse(ext);
        if !matches!(entry, ExtensionEntry::File { .. }) {
            continue;
        }
        let key = ext.trim();
        match load_file(&runtime, path, key, entry) {
            Ok(extension) => runtime.record_file_entry(key, &extension),
            Err(diagnostic) => runtime.record_load_failure(key, diagnostic),
        }
    }

    runtime
}

/// Load the component the `.wasm` file entry `key` names (`entry`, its
/// reading) under the name it declares, which is returned. Refused with
/// E028, naming the entry: a file that does not exist, that does not load
/// as a component or answer its handshake, that declares another name than
/// the one written before `=`, or that declares an extension already
/// loaded.
fn load_file(
    runtime: &ComponentRuntime,
    root: &Path,
    key: &str,
    entry: ExtensionEntry<'_>,
) -> Result<String, specforge_common::Diagnostic> {
    let ExtensionEntry::File { name, path } = entry else {
        unreachable!("load_file is given file entries");
    };
    let file = entry.file(root).expect("a file entry names a file");
    let refused = |message: String, suggestion: String| {
        specforge_common::Diagnostic::new(
            codes::E028,
            format!("extension entry '{key}' in specforge.json: {message}"),
        )
        .with_suggestion(suggestion)
    };
    if !file.is_file() {
        return Err(refused(
            format!("the file {} does not exist", file.display()),
            "build the extension, or correct the path (a relative path is relative to the project root)"
                .to_string(),
        ));
    }
    // Loaded under the entry until its handshake says what it is.
    runtime.load_module_as(key, &file).map_err(|e| {
        refused(
            format!(
                "{} does not load as an extension component: {e}",
                file.display()
            ),
            "build it with specforge-extension-sdk for wasm32-wasip2".to_string(),
        )
    })?;
    let declared = match specforge_wasm::ExtensionCalls::new(runtime).handshake(key) {
        Ok(handshake) => handshake.response.name,
        Err(error) => {
            runtime.unload(key);
            return Err(refused(
                format!("{} answers no handshake: {error}", file.display()),
                "build it with specforge-extension-sdk for wasm32-wasip2".to_string(),
            ));
        }
    };
    if let Some(name) = name.filter(|name| *name != declared) {
        runtime.unload(key);
        return Err(refused(
            format!("{} declares '{declared}', not '{name}'", file.display()),
            format!("write \"{declared}={path}\", or just \"{path}\""),
        ));
    }
    if declared != key && runtime.loaded_names().contains(&declared) {
        runtime.unload(key);
        return Err(refused(
            format!(
                "{} declares '{declared}', and '{declared}' is already loaded by another entry",
                file.display()
            ),
            "remove one of the two entries".to_string(),
        ));
    }
    runtime.rename(key, &declared);
    Ok(declared)
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
        return Err(specforge_common::Diagnostic::new(
            codes::E028,
            format!("extension '{name}' is enabled in specforge.json but not installed (no specforge.lock entry)"),
        )
        .with_suggestion(format!("install it with: specforge add {name}")));
    };
    let wasm = specforge_wasm::installed_wasm_path(&root.join(".specforge/extensions"), name);
    specforge_wasm::load_wasm_module(name, &wasm, runtime, Some(&entry.wasm_hash))
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

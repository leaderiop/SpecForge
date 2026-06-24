use std::path::Path;
use std::sync::{Arc, Mutex};

use specforge_common::load_project_config;
use specforge_extism::{builtins, ExtismRuntime, HostContext};

pub use specforge_emitter::compile::CompilationContext;

/// Compile a project using the Extism Wasm runtime.
///
/// Only extensions listed in `specforge.json` are loaded — no implicit builtins.
/// Builtin extensions are loaded from embedded Wasm binaries when their name
/// matches a `@specforge/*` builtin. Custom `.wasm` paths are loaded from disk.
pub fn compile(path: &Path) -> CompilationContext {
    let config = load_project_config(path);

    let ctx = HostContext::new(Arc::new(Mutex::new(Vec::new())))
        .with_spec_root(path.to_path_buf());
    let runtime = ExtismRuntime::with_host_context(ctx);

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
            if let Err(e) = runtime.load_module_as(name, &resolved, None) {
                eprintln!("warning: failed to load Wasm extension '{}': {}", name, e);
            }
        }
    }

    specforge_emitter::compile::compile_with_runtime(path, Some(&runtime))
}

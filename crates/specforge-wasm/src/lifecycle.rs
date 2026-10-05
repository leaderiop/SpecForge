use crate::integrity::hex_sha256;
use crate::runtime::{ExtensionLifecycleState, LoadedModule, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use std::path::Path;

/// Load a Wasm component from `wasm_path` under `extension_name`.
///
/// `expected_hash` enforces the lockfile pin (spec #21, T4): when it carries
/// a hash (from `specforge.lock`), the on-disk binary must match it — a
/// mismatch refuses the load with a remediation hint. `None` or an empty
/// string (legacy lockfile entries from before hash pinning) load unchanged.
pub fn load_wasm_module(
    extension_name: &str,
    wasm_path: &Path,
    runtime: &dyn WasmRuntime,
    expected_hash: Option<&str>,
) -> Result<LoadedModule, Diagnostic> {
    // Check if the .wasm binary exists
    if !wasm_path.exists() {
        return Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': .wasm binary not found at '{}'",
                extension_name,
                wasm_path.display()
            ),
            span: None,
            suggestion: Some(format!(
                "install the extension with: specforge add {}",
                extension_name
            )),
            data: None,
        });
    }

    // Content hash recorded on the loaded module (lockfile verification)
    let bytes = std::fs::read(wasm_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!(
            "extension '{}': cannot read .wasm binary at '{}': {}",
            extension_name,
            wasm_path.display(),
            e
        ),
        span: None,
        suggestion: None,
        data: None,
    })?;
    let wasm_hash = hex_sha256(&bytes);

    // Lockfile pin enforcement (spec #21, T4): a recorded hash that no
    // longer matches the on-disk binary means the installed extension was
    // tampered with or corrupted after install. Refuse before touching the
    // runtime (this also denies a tampered binary a cache-hit load path).
    if let Some(expected) = expected_hash.filter(|h| !h.is_empty() && h != &wasm_hash) {
        return Err(Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!(
                "integrity mismatch for '{}': lockfile records hash {} but the installed binary is {}",
                extension_name, expected, wasm_hash
            ),
            span: None,
            suggestion: Some(format!(
                "the installed binary changed after install — re-install it: specforge remove \"{0}\" && specforge add \"{0}\"",
                extension_name
            )),
            data: None,
        });
    }

    runtime
        .load_module_named(extension_name, wasm_path)
        .map_err(|e| Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': failed to load Wasm module: {}",
                extension_name, e
            ),
            span: None,
            suggestion: None,
            data: None,
        })?;

    Ok(LoadedModule {
        extension_name: extension_name.to_string(),
        wasm_hash,
        state: ExtensionLifecycleState::Loading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InProcessRuntime;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
    use std::io::Write;
    use tempfile::TempDir;

    /// A runtime that serves `test-ext`, so loading its binary succeeds.
    fn runtime() -> InProcessRuntime {
        InProcessRuntime::new()
            .with(|| ContributionsBuilder::new(ExtensionMeta::new("test-ext", "1.0.0")))
    }

    fn create_fake_wasm(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"\x00asm\x01\x00\x00\x00fake").unwrap();
        path
    }

    // -- load_wasm_module --

    // B:load_wasm_module — verify unit "loads .wasm binary from manifest path"
    #[test]
    fn test_loads_wasm_binary_from_manifest_path() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = runtime();

        let module = load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();
        assert_eq!(module.extension_name, "test-ext");
        assert_eq!(module.state, ExtensionLifecycleState::Loading);
        assert!(!module.wasm_hash.is_empty());
    }

    // B:load_wasm_module — verify unit "missing .wasm produces ExtensionError"
    #[test]
    fn test_missing_wasm_produces_extension_error() {
        let runtime = runtime();
        let missing = Path::new("/nonexistent/ext.wasm");

        let err = load_wasm_module("test-ext", missing, &runtime, None).unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("not found"));
    }

    // B:load_wasm_module — verify contract "requires/ensures consistency for Wasm module loading"
    #[test]
    fn test_load_wasm_module_contract() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = runtime();

        // ensures: extension_loaded on success
        let module = load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();
        assert_eq!(module.state, ExtensionLifecycleState::Loading);

        // ensures: missing_binary_diagnosed
        let missing = Path::new("/nonexistent.wasm");
        let err = load_wasm_module("missing", missing, &runtime, None).unwrap_err();
        assert_eq!(err.code, "E028");
        assert_eq!(err.severity, Severity::Error);
    }
}

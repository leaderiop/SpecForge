//! Extension mode detection: manifest.json vs Wasm protocol.
//!
//! Given an extension directory, determines whether it uses the legacy
//! manifest.json format or the new Wasm protocol (`__handshake` / `__describe`).

use std::path::{Path, PathBuf};

/// How an extension directory declares its capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionMode {
    /// Legacy: capabilities declared in manifest.json.
    Manifest,
    /// Protocol: capabilities discovered via __handshake / __describe Wasm exports.
    Protocol,
}

/// Detect whether an extension directory uses manifest.json or Wasm protocol.
///
/// Returns `None` if the directory has neither `manifest.json` nor `.wasm` files.
/// When both exist, `Manifest` wins (backward compatibility during migration).
pub fn detect_extension_mode(dir: &Path) -> Option<ExtensionMode> {
    if dir.join("manifest.json").exists() {
        return Some(ExtensionMode::Manifest);
    }

    if find_wasm_binary(dir).is_some() {
        return Some(ExtensionMode::Protocol);
    }

    None
}

/// Find the first `.wasm` binary in a directory (non-recursive).
pub fn find_wasm_binary(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "wasm") && path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_manifest_when_manifest_json_exists() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("manifest.json"), "{}").unwrap();
        assert_eq!(detect_extension_mode(dir.path()), Some(ExtensionMode::Manifest));
    }

    #[test]
    fn detect_protocol_when_wasm_file_exists() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("extension.wasm"), [0x00, 0x61, 0x73, 0x6d]).unwrap();
        assert_eq!(detect_extension_mode(dir.path()), Some(ExtensionMode::Protocol));
    }

    #[test]
    fn detect_none_for_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_extension_mode(dir.path()), None);
    }

    #[test]
    fn manifest_wins_when_both_exist() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("manifest.json"), "{}").unwrap();
        std::fs::write(dir.path().join("extension.wasm"), [0x00]).unwrap();
        assert_eq!(detect_extension_mode(dir.path()), Some(ExtensionMode::Manifest));
    }

    #[test]
    fn find_wasm_binary_returns_path() {
        let dir = TempDir::new().unwrap();
        let wasm_path = dir.path().join("my_ext.wasm");
        std::fs::write(&wasm_path, [0x00, 0x61, 0x73, 0x6d]).unwrap();
        assert_eq!(find_wasm_binary(dir.path()), Some(wasm_path));
    }

    #[test]
    fn find_wasm_binary_returns_none_for_no_wasm() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("readme.md"), "hello").unwrap();
        assert_eq!(find_wasm_binary(dir.path()), None);
    }
}

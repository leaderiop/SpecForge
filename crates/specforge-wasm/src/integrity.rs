use sha2::{Digest, Sha256};
use specforge_common::{Diagnostic, Severity};
use std::path::Path;

/// Verify the SHA256 integrity of a .wasm binary against an expected hash.
/// Returns Ok(()) if the hash matches, or an error diagnostic on mismatch or missing file.
pub fn verify_wasm_integrity(wasm_path: &Path, expected_hash: &str) -> Result<(), Diagnostic> {
    let bytes = std::fs::read(wasm_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!("cannot read .wasm binary at '{}': {}", wasm_path.display(), e),
        span: None,
        suggestion: None,
    })?;

    let actual_hash = hex_sha256(&bytes);

    if actual_hash != expected_hash {
        return Err(Diagnostic {
            code: "E032".to_string(),
            severity: Severity::Error,
            message: format!(
                "integrity check failed for '{}': expected {}, got {}. Possible tampering.",
                wasm_path.display(), expected_hash, actual_hash
            ),
            span: None,
            suggestion: Some("re-install the extension or verify the source".to_string()),
        });
    }

    Ok(())
}

/// Compute the SHA256 hex digest of a byte slice.
pub fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Verify integrity but skip the check if `skip_verify` is true, emitting a warning.
pub fn verify_wasm_integrity_or_skip(
    wasm_path: &Path,
    expected_hash: &str,
    skip_verify: bool,
) -> (bool, Vec<Diagnostic>) {
    if skip_verify {
        return (true, vec![Diagnostic {
            code: "W027".to_string(),
            severity: Severity::Warning,
            message: format!(
                "integrity check skipped for '{}' due to --skip-verify flag",
                wasm_path.display()
            ),
            span: None,
            suggestion: None,
        }]);
    }

    match verify_wasm_integrity(wasm_path, expected_hash) {
        Ok(()) => (true, vec![]),
        Err(diag) => (false, vec![diag]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn write_temp_wasm(content: &[u8]) -> (NamedTempFile, String) {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();
        let hash = hex_sha256(content);
        (f, hash)
    }

    // B:verify_wasm_integrity — verify unit "matching hash passes verification"
    #[test]
    fn test_matching_hash_passes_verification() {
        let (f, hash) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00fake wasm");
        let result = verify_wasm_integrity(f.path(), &hash);
        assert!(result.is_ok());
    }

    // B:verify_wasm_integrity — verify unit "mismatched hash produces hard error"
    #[test]
    fn test_mismatched_hash_produces_hard_error() {
        let (f, _) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00fake wasm");
        let err = verify_wasm_integrity(f.path(), "deadbeef").unwrap_err();
        assert_eq!(err.code, "E032");
        assert_eq!(err.severity, Severity::Error);
        assert!(err.message.contains("tampering"));
    }

    // B:verify_wasm_integrity — verify unit "--skip-verify bypasses check with warning"
    #[test]
    fn test_skip_verify_bypasses_with_warning() {
        let (f, _) = write_temp_wasm(b"data");
        let (ok, diags) = verify_wasm_integrity_or_skip(f.path(), "wrong-hash", true);
        assert!(ok);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "W027");
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // B:verify_wasm_integrity — verify contract "requires/ensures consistency for Wasm integrity verification"
    #[test]
    fn test_verify_integrity_contract() {
        let content = b"test module binary";
        let (f, hash) = write_temp_wasm(content);

        // ensures: wasm_integrity_verified — correct hash passes
        assert!(verify_wasm_integrity(f.path(), &hash).is_ok());

        // ensures: tampering_diagnosed — wrong hash is hard error
        let err = verify_wasm_integrity(f.path(), "badhash").unwrap_err();
        assert_eq!(err.code, "E032");

        // ensures: missing binary diagnosed
        let missing = Path::new("/nonexistent/path.wasm");
        let err = verify_wasm_integrity(missing, &hash).unwrap_err();
        assert_eq!(err.code, "E028");
    }
}

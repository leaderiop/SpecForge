// Slice 12: Wasm integrity integration tests
//
// Tests behaviors through the public API:
// - B:verify_wasm_integrity

use specforge_common::Severity;
use specforge_wasm::{hex_sha256, verify_wasm_integrity};
use std::path::Path;
use tempfile::NamedTempFile;

fn write_temp_wasm(content: &[u8]) -> (NamedTempFile, String) {
    use std::io::Write;
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content).unwrap();
    f.flush().unwrap();
    let hash = hex_sha256(content);
    (f, hash)
}

// ============================================================================
// B:verify_wasm_integrity — integration tests
// ============================================================================

// B:verify_wasm_integrity — verify integration "matching SHA256 → Ok"
#[test]
fn test_verify_integrity_matching_hash_passes() {
    let (f, hash) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00test_binary");
    let result = verify_wasm_integrity(f.path(), &hash);
    assert!(result.is_ok());
}

// B:verify_wasm_integrity — verify integration "mismatched SHA256 → E032 (tampering)"
#[test]
fn test_verify_integrity_mismatch_produces_e032() {
    let (f, _) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00test_binary");
    let err = verify_wasm_integrity(f.path(), "deadbeefdeadbeef").unwrap_err();
    assert_eq!(err.code, "E032");
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("tampering"));
}

// B:verify_wasm_integrity — verify contract "requires hash + bytes, ensures integrity check"
#[test]
fn test_verify_integrity_contract() {
    let content = b"module_binary_content";
    let (f, hash) = write_temp_wasm(content);

    // ensures: correct hash passes
    assert!(verify_wasm_integrity(f.path(), &hash).is_ok());

    // ensures: wrong hash fails with E032
    let err = verify_wasm_integrity(f.path(), "badhash").unwrap_err();
    assert_eq!(err.code, "E032");

    // ensures: missing file fails with E028
    let err = verify_wasm_integrity(Path::new("/no/such/file.wasm"), &hash).unwrap_err();
    assert_eq!(err.code, "E028");
}

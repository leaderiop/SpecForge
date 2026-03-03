use sha2::{Digest, Sha256};

const CHECKSUM_PREFIX: &str = "// @specforge-checksum:sha256:";

/// Compute a SHA-256 checksum of the content and prepend a checksum header line.
pub fn prepend_checksum(content: &str) -> String {
    let hash = compute_sha256(content);
    format!("{CHECKSUM_PREFIX}{hash}\n{content}")
}

/// Extract the checksum from a file's first line, if present.
/// Returns `Some((hex_checksum, rest_of_content))` or `None`.
pub fn extract_checksum(file_content: &str) -> Option<(&str, &str)> {
    let first_line = file_content.lines().next()?;
    let hex = first_line.strip_prefix(CHECKSUM_PREFIX)?;
    // rest is everything after the first line (skip the newline)
    let rest = &file_content[first_line.len()..];
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    Some((hex, rest))
}

/// Verify that a file's checksum header matches its content.
/// Returns `true` if the checksum is valid, `false` if tampered or missing.
pub fn verify_checksum(file_content: &str) -> bool {
    match extract_checksum(file_content) {
        Some((expected_hex, body)) => {
            let actual_hex = compute_sha256(body);
            expected_hex == actual_hex
        }
        None => false,
    }
}

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepend_and_verify_roundtrip() {
        let content = "export interface User {\n  name: string;\n}\n";
        let with_checksum = prepend_checksum(content);

        assert!(with_checksum.starts_with(CHECKSUM_PREFIX));
        assert!(verify_checksum(&with_checksum));
    }

    #[test]
    fn tampered_content_fails_verify() {
        let content = "export interface User {\n  name: string;\n}\n";
        let with_checksum = prepend_checksum(content);

        let tampered = with_checksum.replace("name: string", "name: number");
        assert!(!verify_checksum(&tampered));
    }

    #[test]
    fn extract_from_valid() {
        let content = "export interface Foo {}\n";
        let with_checksum = prepend_checksum(content);

        let (hex, body) = extract_checksum(&with_checksum).unwrap();
        assert_eq!(body, content);
        assert_eq!(hex, compute_sha256(content));
    }

    #[test]
    fn extract_from_no_header() {
        assert!(extract_checksum("no header here").is_none());
    }

    #[test]
    fn verify_no_header_returns_false() {
        assert!(!verify_checksum("plain content"));
    }
}

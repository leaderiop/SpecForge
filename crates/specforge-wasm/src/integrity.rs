use sha2::{Digest, Sha256};

/// The SHA256 hex digest of `data`: what the lock file pins an installed
/// binary to, and what loading it checks (`load_wasm_module`, E033).
pub fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_is_sha256_in_lowercase_hex() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hex_sha256(b"").len(), 64);
    }
}

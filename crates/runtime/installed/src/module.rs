use specforge_package::Sha256;

/// The SHA-256 digest of `data`: what `specforge.lock` pins an installed binary to.
pub fn hex_sha256(data: &[u8]) -> Sha256 {
    Sha256::of(data)
}

/// An extension binary and its SHA-256, computed once: the bytes that are hashed are the bytes that
/// are loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    bytes: Vec<u8>,
    digest: Sha256,
}

impl Module {
    pub fn new(bytes: Vec<u8>) -> Module {
        let digest = hex_sha256(&bytes);
        Module { bytes, digest }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn digest(&self) -> &Sha256 {
        &self.digest
    }
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
        assert_eq!(hex_sha256(b"").as_str().len(), 64);
    }

    #[test]
    fn a_module_digests_the_bytes_it_holds() {
        let module = Module::new(b"abc".to_vec());
        assert_eq!(module.digest(), &hex_sha256(b"abc"));
        assert_eq!(module.bytes(), b"abc");
    }
}

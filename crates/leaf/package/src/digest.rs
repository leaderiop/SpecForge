//! A SHA-256 digest: the one spelling a module's digest, a registry's reply and `specforge.lock` share
//! (ADR 0243, 0265).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::Digest;

/// A SHA-256 digest: 64 lowercase hex characters, the one spelling a module's digest, a registry's reply
/// and `specforge.lock` share (ADR 0243, 0265).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sha256(String);

impl Sha256 {
    /// The digest of `bytes`.
    pub fn of(bytes: &[u8]) -> Sha256 {
        Sha256(format!("{:x}", sha2::Sha256::digest(bytes)))
    }

    /// `text` as a digest; the reason when it is not 64 lowercase hex characters.
    pub fn parse(text: &str) -> Result<Sha256, String> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if text.len() == 64 && text.chars().all(hex) {
            Ok(Sha256(text.to_string()))
        } else {
            Err(format!(
                "{text:?} is not a SHA-256 (64 lowercase hex characters)"
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Sha256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Sha256 {
    type Err = String;

    fn from_str(text: &str) -> Result<Sha256, String> {
        Sha256::parse(text)
    }
}

impl PartialEq<str> for Sha256 {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Sha256 {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Serialize for Sha256 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Sha256::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sha256_is_64_lowercase_hex_characters() {
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(Sha256::parse(digest).unwrap(), Sha256::of(b"abc"));
        for text in [
            "",
            "00",
            "aaaaaaaé",
            &digest.to_uppercase(),
            &digest[1..],
            &format!("{digest}0"),
            "g".repeat(64).as_str(),
        ] {
            let why = Sha256::parse(text).unwrap_err();
            assert!(why.contains("not a SHA-256"), "{why}");
        }
    }

    #[test]
    fn a_sha256_reads_and_writes_as_its_text() {
        let digest = Sha256::of(b"abc");
        let text = serde_json::to_string(&digest).unwrap();
        assert_eq!(text, format!("\"{digest}\""));
        assert_eq!(serde_json::from_str::<Sha256>(&text).unwrap(), digest);
        let why = serde_json::from_str::<Sha256>("\"nope\"")
            .unwrap_err()
            .to_string();
        assert!(why.contains("is not a SHA-256"), "{why}");
    }
}

//! How the lock reads the values of an entry: each field is parsed where the lock is read, so a
//! refused one makes the whole lock unreadable (E033) and nothing downstream re-judges its text
//! (ADR 0243).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specforge_package::Version;

/// `version` as the lock reads it: a SemVer version, the error naming the field.
pub(super) fn version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
    let text = String::deserialize(deserializer)?;
    Version::parse(&text).map_err(|why| {
        serde::de::Error::custom(format!("version {text:?} is not a SemVer version: {why}"))
    })
}

/// A publisher key id as `specforge.lock` records it: 16 lowercase hex characters, the first 16 of
/// the SHA-256 of an Ed25519 public key (`specforge_registry_wire::signature::key_id`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId(String);

impl KeyId {
    /// `text` as a key id; the reason when it is not 16 lowercase hex characters.
    pub fn parse(text: &str) -> Result<KeyId, String> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if text.len() == 16 && text.chars().all(hex) {
            Ok(KeyId(text.to_string()))
        } else {
            Err(format!(
                "{text:?} is not a publisher key id (16 lowercase hex characters)"
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for KeyId {
    type Err = String;

    fn from_str(text: &str) -> Result<KeyId, String> {
        KeyId::parse(text)
    }
}

impl Serialize for KeyId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for KeyId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        KeyId::parse(&text).map_err(|why| serde::de::Error::custom(format!("key_id {why}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Holder {
        #[serde(deserialize_with = "version")]
        version: Version,
    }

    #[test]
    fn a_lock_version_is_read_as_semver() {
        let read =
            |text: &str| serde_json::from_value::<Holder>(serde_json::json!({"version": text}));

        assert_eq!(
            read("2.0.0-beta.1").unwrap().version,
            Version::parse("2.0.0-beta.1").unwrap()
        );
        for text in ["", "1.0", "latest", "v1.0.0", "0.1.x", "0.1.0\u{1b}[2J"] {
            let why = read(text).err().unwrap().to_string();
            assert!(why.starts_with("version "), "{text:?}: {why}");
            assert!(why.contains("is not a SemVer version"), "{text:?}: {why}");
        }
    }

    #[test]
    fn a_key_id_is_16_lowercase_hex_characters() {
        assert_eq!(
            KeyId::parse("6d656d6f72792d6b").unwrap().as_str(),
            "6d656d6f72792d6b"
        );
        for text in [
            "",
            "k",
            "in-memory",
            "ABCDEF0123456789",
            "abcdef012345678",
            "abcdef01234567890",
        ] {
            let why = KeyId::parse(text).unwrap_err();
            assert!(why.contains("not a publisher key id"), "{text:?}: {why}");
        }
    }

    #[test]
    fn a_key_id_reads_and_writes_as_its_text() {
        let key = KeyId::parse("6d656d6f72792d6b").unwrap();
        let text = serde_json::to_string(&key).unwrap();
        assert_eq!(text, "\"6d656d6f72792d6b\"");
        assert_eq!(serde_json::from_str::<KeyId>(&text).unwrap(), key);
        let why = serde_json::from_str::<KeyId>("\"nope\"")
            .unwrap_err()
            .to_string();
        assert!(why.starts_with("key_id "), "{why}");
    }
}

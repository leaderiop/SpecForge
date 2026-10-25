//! How the lock reads the values of an entry: each field is parsed where the lock is read, so a
//! refused one makes the whole lock unreadable (E033) and nothing downstream re-judges its text
//! (ADR 0243).

use serde::{Deserialize, Deserializer};
use specforge_package::Version;

/// `version` as the lock reads it: a SemVer version, the error naming the field.
pub(super) fn version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
    let text = String::deserialize(deserializer)?;
    Version::parse(&text).map_err(|why| {
        serde::de::Error::custom(format!("version {text:?} is not a SemVer version: {why}"))
    })
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
}

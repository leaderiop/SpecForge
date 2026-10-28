//! Reading a `package.json`: whether it is a JSON object, and the keys of its `scripts`. `serde_json` is the SDK's own
//! JSON crate, so the guest links it already. Like `JSON.parse`, a duplicate key keeps its last value; unlike it, a
//! lone surrogate escape and nesting past 128 levels are refused (`manifest_unreadable`, a warning).

/// The script names of a `package.json` text; `None` when the text is not a JSON object.
pub fn scripts(text: &str) -> Option<Vec<String>> {
    let serde_json::Value::Object(manifest) = serde_json::from_str(text).ok()? else {
        return None;
    };
    Some(
        manifest
            .get("scripts")
            .and_then(|scripts| scripts.as_object())
            .map(|scripts| scripts.keys().cloned().collect())
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::scripts;

    #[test]
    fn the_scripts_of_a_manifest_are_listed() {
        let text = r#"{ "name": "w", "n": [1, -2.5e3, true, null, {"a": []}], "scripts": { "test": "x \"y\"", "lint": "z" } }"#;
        let mut names = scripts(text).expect("an object");
        names.sort();
        assert_eq!(names, ["lint", "test"]);
        assert_eq!(scripts("{}"), Some(vec![]));
        assert_eq!(scripts(r#"{"scripts": "no"}"#), Some(vec![]));
    }

    #[test]
    fn a_duplicate_key_keeps_the_last_value_as_json_parse_does() {
        let text = r#"{"scripts": {"old": "a"}, "scripts": {"new": "b"}}"#;
        assert_eq!(scripts(text), Some(vec!["new".into()]));
    }

    #[test]
    fn escapes_are_decoded_in_script_names() {
        assert_eq!(
            scripts(r#"{"scripts": {"test": "a", "q\"": "b"}}"#).map(|mut n| {
                n.sort();
                n
            }),
            Some(vec!["q\"".into(), "test".into()])
        );
        assert_eq!(scripts(r#"{"scripts": {"\u+123": "a"}}"#), None);
        assert_eq!(scripts(r#"{"scripts": {"\x": "a"}}"#), None);
    }

    #[test]
    fn a_broken_or_non_object_manifest_is_none() {
        for text in [
            "",
            "{ not json",
            "[]",
            "{\"a\": 1,}",
            "{\"a\": 1} x",
            "{\"a\": tru}",
            "{\"a\": e}",
            "{\"a\": +1}",
            "{\"a\": 1.2.3}",
            "{\"a\": 1}\u{c}",
        ] {
            assert_eq!(scripts(text), None, "{text:?}");
        }
    }
}

//! A JSON key's span, found without a JSON parser that keeps spans (ADR 0288).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::Value;
use specforge_common::json_span::{index, locate, path_at, span};

/// A small deterministic generator, so a failure names its seed.
struct Gen(u64);

impl Gen {
    fn next(&mut self, below: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % below
    }

    fn space(&mut self) -> &'static str {
        ["", " ", "\n  ", "\t"][self.next(4) as usize]
    }

    /// A key as written: plain, with escapes, with a multi-byte letter, or the same letter escaped.
    fn key(&mut self) -> String {
        [
            "\"a\"",
            "\"b\"",
            "\"c\\\"d\"",
            "\"é\"",
            "\"\\u00e9\"",
            "\"k\\\\n\"",
            "\"x.y\"",
            "\"\\ud83d\\ude00\"",
            "\"😀\"",
        ][self.next(9) as usize]
            .to_string()
    }

    fn value(&mut self, depth: usize) -> String {
        match if depth > 3 {
            self.next(4)
        } else {
            self.next(7)
        } {
            0 => "true".into(),
            1 => "-12.5e3".into(),
            2 => "\"s,}]\\\"x\"".into(),
            3 => "null".into(),
            4 | 5 => {
                let n = self.next(5);
                let mut out = String::from("{");
                for i in 0..n {
                    if i > 0 {
                        out.push(',');
                    }
                    let (s1, s2, s3) = (self.space(), self.space(), self.space());
                    out.push_str(&format!(
                        "{s1}{}{s2}:{s3}{}",
                        self.key(),
                        self.value(depth + 1)
                    ));
                }
                out.push_str(self.space());
                out.push('}');
                out
            }
            _ => {
                let n = self.next(4);
                let items: Vec<String> = (0..n).map(|_| self.value(depth + 1)).collect();
                format!(
                    "[{}{}]",
                    self.space(),
                    items.join(&format!(",{}", self.space()))
                )
            }
        }
    }
}

/// Every member path of `value`, with the member's own value.
fn members<'v>(value: &'v Value, path: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'v Value)>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                path.push(key.clone());
                out.push((path.clone(), inner));
                members(inner, path, out);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (i, inner) in items.iter().enumerate() {
                path.push(i.to_string());
                out.push((path.clone(), inner));
                members(inner, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a JSON key is located at its own text and the path at an offset names the key or value there"
)]
fn a_json_key_is_located_at_its_text() {
    let mut checked = 0;
    for seed in 0..3000u64 {
        let text = Gen(seed).value(0);
        let parsed: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
        let mut all = Vec::new();
        members(&parsed, &mut Vec::new(), &mut all);
        let indexed = index(&text);
        for (path, value) in all {
            let refs: Vec<&str> = path.iter().map(String::as_str).collect();
            let found = indexed
                .locate(&refs)
                .unwrap_or_else(|| panic!("{path:?} not located in {text}"));
            // The value range is the value's own text.
            let written: Value = serde_json::from_str(&text[found.value.clone()])
                .unwrap_or_else(|e| panic!("{path:?}: {e} in {text}"));
            assert_eq!(&written, value, "{path:?} in {text}");
            // The key range is the key's own text, decoded as serde_json decodes it.
            if let Some(key) = &found.key {
                let decoded: String = serde_json::from_str(&text[key.clone()]).unwrap();
                assert_eq!(Some(&decoded), path.last(), "{text}");
                let under = path_at(&text, key.start + 1).unwrap();
                assert_eq!(under.path, path, "{text}");
            }
            checked += 1;
        }
    }
    assert!(checked > 3000, "{checked} members: the generator is thin");

    // A repeated key: the last is the live one, as in serde_json.
    let text = r#"{"a": {"x": 1}, "a": {"y": 2}}"#;
    assert!(locate(text, &["a", "x"]).is_none());
    let y = locate(text, &["a", "y"]).unwrap();
    assert_eq!(&text[y.value], "2");

    // Lines and columns: 1-based, columns in characters.
    let text = "{\n  \"é\": {\"k\": 1}\n}";
    let k = locate(text, &["é", "k"]).unwrap();
    let at = span(text, "specforge.json", k.key.unwrap());
    assert_eq!((at.start_line, at.start_col, at.end_col), (2, 9, 12));
    assert_eq!(at.file.as_str(), "specforge.json");

    // Not JSON: indexed as far as it is well formed.
    assert!(locate("{\"a\": 1, \"b\": ", &["a"]).is_some());
}

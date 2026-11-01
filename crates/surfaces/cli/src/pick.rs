//! `--pick <path>`: the values at a path of a JSON answer, one per line (ADR 0325), so a shell reads one value of
//! `specforge query` or of an extension command without jq.
//!
//! The path is steps: `.key`, `[n]`, `[]` (every item) and `[kind=<k>]` (the items whose `kind` is `<k>`). A
//! string prints bare, a number or boolean as JSON, an object or array as compact JSON; `null` and a missing value
//! print nothing, and are not an error.

use serde_json::Value;

/// The help text of `--pick`, which `query` and every extension command share.
pub(crate) const HELP: &str = "Print only the values at this path of the JSON answer, one per line: `.key`, `[n]`, \
     `[]` (every item), `[kind=<k>]` (the items of that kind), e.g. '.nodes[kind=plan].id'. A string prints bare (a \
     newline in it is printed as is), a number, boolean, object or array as compact JSON, null or a missing value as \
     nothing. It reads the JSON answer: on query, the graph format, with neither --inputs nor --include-coverage";

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
    Each,
    Kind(String),
}

/// A parsed `--pick` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pick(Vec<Step>);

impl Pick {
    /// Parse `text`; a refusal names the byte where the path goes wrong.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut steps = Vec::new();
        let mut at = 0;
        while at < text.len() {
            let rest = &text[at..];
            if let Some(key) = rest.strip_prefix('.') {
                let len = key.find(['.', '[']).unwrap_or(key.len());
                if len == 0 {
                    return Err(format!("at byte {at}: '.' needs a key after it"));
                }
                steps.push(Step::Key(key[..len].to_string()));
                at += 1 + len;
            } else if let Some(inner) = rest.strip_prefix('[') {
                let Some(len) = inner.find(']') else {
                    return Err(format!("at byte {at}: '[' is not closed"));
                };
                let inner = &inner[..len];
                steps.push(match inner.strip_prefix("kind=") {
                    Some("") => return Err(format!("at byte {at}: '[kind=]' needs a kind")),
                    Some(kind) => Step::Kind(kind.to_string()),
                    None if inner.is_empty() => Step::Each,
                    None => Step::Index(inner.parse().map_err(|_| {
                        format!("at byte {at}: expected '[<index>]', '[]' or '[kind=<k>]', found '[{inner}]'")
                    })?),
                });
                at += 2 + len;
            } else {
                return Err(format!(
                    "at byte {at}: expected '.<key>' or '[', found '{rest}'"
                ));
            }
        }
        if steps.is_empty() {
            return Err("at byte 0: the path is empty".to_string());
        }
        Ok(Self(steps))
    }

    /// The values `document` holds at the path, in document order.
    fn values<'v>(&self, document: &'v Value) -> Vec<&'v Value> {
        let mut reached = vec![document];
        for step in &self.0 {
            reached = reached
                .into_iter()
                .flat_map(|value| -> Vec<&Value> {
                    match (step, value) {
                        (Step::Key(key), Value::Object(object)) => {
                            object.get(key).into_iter().collect()
                        }
                        (Step::Index(n), Value::Array(items)) => {
                            items.get(*n).into_iter().collect()
                        }
                        (Step::Each, Value::Array(items)) => items.iter().collect(),
                        (Step::Kind(kind), Value::Array(items)) => items
                            .iter()
                            .filter(|item| item.get("kind").and_then(Value::as_str) == Some(kind))
                            .collect(),
                        _ => Vec::new(),
                    }
                })
                .collect();
        }
        reached
    }

    /// What `--pick` prints for `document`: one line per value reached.
    pub(crate) fn lines(&self, document: &Value) -> String {
        let mut text = String::new();
        for value in self.values(document) {
            match value {
                Value::Null => continue,
                Value::String(s) => text.push_str(s),
                other => text.push_str(&other.to_string()),
            }
            text.push('\n');
        }
        text
    }

    /// [`Self::lines`] for an answer printed as text: nothing for an empty answer, else `Err` with the answer when it
    /// is not JSON.
    pub(crate) fn printed(&self, answer: &str) -> Result<String, String> {
        if answer.trim().is_empty() {
            return Ok(String::new());
        }
        serde_json::from_str(answer)
            .map(|document| self.lines(&document))
            .map_err(|e| format!("error: --pick: the answer is not JSON ({e})\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn picked(path: &str, document: &Value) -> String {
        Pick::parse(path).unwrap().lines(document)
    }

    #[test]
    fn the_grammar_has_four_steps() {
        assert_eq!(
            Pick::parse(".nodes[kind=plan].fields.goal[0][]").unwrap().0,
            [
                Step::Key("nodes".into()),
                Step::Kind("plan".into()),
                Step::Key("fields".into()),
                Step::Key("goal".into()),
                Step::Index(0),
                Step::Each,
            ]
        );
    }

    #[test]
    fn a_malformed_path_names_its_byte() {
        for (path, at) in [
            ("", "byte 0"),
            ("nodes", "byte 0"),
            (".", "byte 0"),
            (".a..b", "byte 2"),
            (".a[", "byte 2"),
            (".a[x]", "byte 2"),
            (".a[-1]", "byte 2"),
            (".a[kind=]", "byte 2"),
            (".a[0]b", "byte 5"),
        ] {
            let error = Pick::parse(path).unwrap_err();
            assert!(error.starts_with(&format!("at {at}:")), "{path}: {error}");
        }
    }

    #[test]
    fn each_value_prints_by_its_type() {
        let document = json!({"s": "bare", "n": 1.5, "b": false, "o": {"a": [1, null]}, "z": null, "l": "a\nb"});
        assert_eq!(picked(".s", &document), "bare\n");
        assert_eq!(picked(".n", &document), "1.5\n");
        assert_eq!(picked(".b", &document), "false\n");
        assert_eq!(picked(".o", &document), "{\"a\":[1,null]}\n");
        assert_eq!(picked(".o.a", &document), "[1,null]\n");
        assert_eq!(picked(".z", &document), "");
        assert_eq!(picked(".l", &document), "a\nb\n");
    }

    #[test]
    fn a_missing_value_prints_nothing() {
        let document = json!({"a": [], "s": "x"});
        for path in [
            ".b",
            ".a[0]",
            ".a[]",
            ".s.k",
            ".s[0]",
            ".s[]",
            "[0]",
            ".a[kind=plan]",
        ] {
            assert_eq!(picked(path, &document), "", "{path}");
        }
    }

    #[test]
    fn every_item_and_the_items_of_a_kind() {
        let document = json!({"nodes": [
            {"id": "p", "kind": "phase"}, {"id": "a", "kind": "plan"}, {"id": "b", "kind": "plan"}, 7,
        ]});
        assert_eq!(picked(".nodes[].id", &document), "p\na\nb\n");
        assert_eq!(picked(".nodes[kind=plan].id", &document), "a\nb\n");
        assert_eq!(picked(".nodes[kind=task]", &document), "");
        assert_eq!(picked(".nodes[3]", &document), "7\n");
        assert_eq!(picked(".nodes[]", &document).lines().count(), 4);
    }

    #[test]
    fn an_answer_that_is_not_json_is_refused_and_an_empty_one_prints_nothing() {
        let pick = Pick::parse(".a").unwrap();
        assert_eq!(pick.printed("{\"a\": 1}\n"), Ok("1\n".into()));
        assert_eq!(pick.printed(""), Ok(String::new()));
        assert!(pick.printed("done").unwrap_err().contains("not JSON"));
    }
}

//! `--pick <path>`: the values at a path of a JSON answer, one per line (ADR 0325), so a shell reads one value of
//! `specforge query` or of an extension command without jq.
//!
//! The path is steps: `.key`, `[n]`, `[]` (every item) and `[<key>=<value>]` (the items whose field `<key>`, a dotted
//! path such as `fields.result`, is `<value>`: a string by its text, another scalar as JSON). A string prints
//! bare, a number or boolean as JSON, an object or array as compact JSON; `null` and a missing value
//! print nothing, and are not an error.

use crate::outcome::{Exit, OutputFormat};
use crate::terminal::{Terminal, errln, out};
use serde_json::Value;

/// The rules every surface's long help states.
macro_rules! rules {
    () => {
        "The path is steps: `.key`, `[n]` (digits, no leading zero), `[]` (every item) and `[<key>=<value>]` (the \
         items whose field <key> is <value>, e.g. `[kind=plan]`: <key> may be dotted, `[fields.result=issue]`, and a \
         number, boolean or null is compared as JSON, `[fields.backlog=true]`, `[fields.wave=2]`; an object \
         or array is never selected); a `.` before `[` may be left out, and `.` \
         alone is the whole answer. One line per value reached: a string bare (a newline in it is printed as is), an \
         integer as is and any other number as an f64 (`1.0`, `-0.0`, `1e3` as `1000.0`), a boolean, object or array \
         as compact JSON; null or a missing value prints nothing, and is not an error."
    };
}

/// The short help of `--pick`, on every command that takes it.
pub(crate) const HELP: &str = "Print only the values at this path of the JSON answer, one per line";

/// `query --pick`'s long help.
pub(crate) const QUERY_LONG_HELP: &str = concat!(
    "Print only the values at this path of the graph format's JSON answer, one per line, e.g. \
     '.nodes[kind=plan].id'. ",
    rules!(),
    " It takes --format graph or json, and neither --inputs nor --include-coverage."
);

/// An extension command's `--pick` long help.
pub(crate) const COMMAND_LONG_HELP: &str = concat!(
    "Print only the values at this path of the command's own JSON answer, one per line, e.g. '.next_command'. ",
    rules!(),
    " It implies --format json (another --format is refused): errors print as they do under json, and the exit code \
     is the command's, or 1 when it passed but its answer is not JSON. A writing command is picked from its own \
     answer, never from the plan report --format json wraps it in."
);

/// `export --pick`'s long help.
pub(crate) const EXPORT_LONG_HELP: &str = concat!(
    "Print only the values at this path of the graph format's JSON answer, one per line, e.g. \
     '.nodes[kind=phase].id'. ",
    rules!(),
    " It takes --format graph or json."
);

/// The long help of `--pick` on a command whose JSON is its answer (`check`, `settings`).
pub(crate) const ANSWER_LONG_HELP: &str = concat!(
    "Print only the values at this path of the command's JSON answer, one per line, e.g. '.[code=E003].message' \
     on check or '.extensions[].settings[name=<setting>].value' on settings. ",
    rules!(),
    " It implies --format json (--format human is refused): errors print as they do under json, and the exit code \
     is the command's."
);

/// `--format human|json` beside `--pick`, for a core command whose JSON form is its answer (`check`, `settings`):
/// with `--pick` the format is `json` unless one is given, so an explicit `--format human` can be refused.
#[derive(clap::Args, Clone, Debug)]
pub(crate) struct PickedOutputArg {
    /// Output format: human or json (json with --pick)
    #[arg(
        long,
        default_value = "human",
        default_value_if("pick", clap::builder::ArgPredicate::IsPresent, "json")
    )]
    format: OutputFormat,

    #[arg(long, value_name = "PATH", value_parser = Pick::parse, help = HELP, long_help = ANSWER_LONG_HELP)]
    pick: Option<Pick>,
}

impl PickedOutputArg {
    /// The format, or the usage error (exit 2) of `--pick` beside `--format human`.
    pub(crate) fn format(&self, terminal: &Terminal) -> Result<OutputFormat, Exit> {
        // a usage error, not a note: written under either format
        match (&self.pick, self.format) {
            (Some(_), OutputFormat::Human) => {
                errln!(
                    terminal,
                    "error: --pick implies --format json: it takes no --format human"
                );
                Err(Exit::Unjudged)
            }
            (_, format) => Ok(format),
        }
    }

    pub(crate) fn pick(&self) -> Option<&Pick> {
        self.pick.as_ref()
    }
}

/// `document` as the JSON form prints it, or the values `pick` reaches in it.
pub(crate) fn print_json(
    terminal: &Terminal,
    pick: Option<&Pick>,
    document: &impl serde::Serialize,
) {
    match pick {
        Some(pick) => {
            let value = serde_json::to_value(document).expect("serialize JSON output");
            out!(terminal, "{}", pick.lines(&value));
        }
        None => terminal.json(document),
    }
}

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
    Each,
    /// `[<key>=<value>]`: the items whose field at the dotted `key` (its segments) is `value`: a string by its text,
    /// another scalar as JSON.
    Field(Vec<String>, String),
}

/// Whether `item`'s field at the dotted `key` is `wanted`: a string by its text, another scalar as JSON.
fn field_is(item: &Value, key: &[String], wanted: &str) -> bool {
    match key
        .iter()
        .try_fold(item, |value, segment| value.get(segment))
    {
        Some(Value::String(text)) => text == wanted,
        // the wanted text read as JSON: `2` is 2 and `true` is true; the integer 2 is not the float 2.0
        Some(scalar @ (Value::Bool(_) | Value::Number(_) | Value::Null)) => {
            serde_json::from_str::<Value>(wanted).is_ok_and(|w| w == *scalar)
        }
        _ => false,
    }
}

/// A parsed `--pick` path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pick(Vec<Step>);

impl Pick {
    /// Parse `text`; a refusal names the byte where the path goes wrong.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        // `.` alone is the whole answer, printed compact
        if text == "." {
            return Ok(Self(Vec::new()));
        }
        let mut steps = Vec::new();
        let mut at = 0;
        while at < text.len() {
            let rest = &text[at..];
            if let Some(key) = rest.strip_prefix('.') {
                // `.[`, as jq writes it, is `[`
                if key.starts_with('[') {
                    at += 1;
                    continue;
                }
                let len = key.find(['.', '[']).unwrap_or(key.len());
                if len == 0 {
                    return Err(format!("at byte {at}: '.' needs a key after it"));
                }
                if let Some(bad) =
                    key[..len].find(|c: char| c.is_whitespace() || c == ']' || c == '=')
                {
                    return Err(format!(
                        "at byte {}: a key holds no whitespace, ']' or '='",
                        at + 1 + bad
                    ));
                }
                steps.push(Step::Key(key[..len].to_string()));
                at += 1 + len;
            } else if let Some(inner) = rest.strip_prefix('[') {
                let Some(len) = inner.find(']') else {
                    return Err(format!("at byte {at}: '[' is not closed"));
                };
                let inner = &inner[..len];
                steps.push(match inner.split_once('=') {
                    Some(("", _)) => return Err(format!("at byte {at}: '[=…]' needs a key")),
                    Some((_, "")) => return Err(format!("at byte {at}: '[{inner}]' needs a value")),
                    Some((key, _)) if key.contains(char::is_whitespace) => {
                        return Err(format!("at byte {at}: a key holds no whitespace, ']' or '='"));
                    }
                    Some((key, _)) if key.split('.').any(str::is_empty) => {
                        return Err(format!("at byte {at}: a dotted key has a name between its dots"));
                    }
                    Some((key, value)) => Step::Field(
                        key.split('.').map(str::to_string).collect(),
                        value.to_string(),
                    ),
                    None if inner.is_empty() => Step::Each,
                    // digits only, no sign and no leading zero: `usize::from_str` takes `+1` and `01`
                    None if inner.bytes().all(|b| b.is_ascii_digit())
                        && (inner == "0" || !inner.starts_with('0')) =>
                    {
                        Step::Index(inner.parse().map_err(|_| {
                            format!("at byte {at}: the index '{inner}' is too large")
                        })?)
                    }
                    None => {
                        return Err(format!(
                            "at byte {at}: expected '[<index>]', '[]' or '[<key>=<value>]', found '[{inner}]'"
                        ));
                    }
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
                        (Step::Field(key, wanted), Value::Array(items)) => items
                            .iter()
                            .filter(|item| field_is(item, key, wanted))
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
                Step::Field(vec!["kind".into()], "plan".into()),
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
            (".a..b", "byte 2"),
            (".a[", "byte 2"),
            (".a[x]", "byte 2"),
            (".a[-1]", "byte 2"),
            (".a[kind=]", "byte 2"),
            (".a[=plan]", "byte 2"),
            (".a[a b=x]", "byte 2"),
            (".a.", "byte 2"),
            (".a[0]b", "byte 5"),
            (".a[+1]", "byte 2"),
            (".a[01]", "byte 2"),
            (".a[00]", "byte 2"),
            (".a b", "byte 2"),
            (".a\tb", "byte 2"),
            (".a]", "byte 2"),
            (".kind=plan", "byte 5"),
        ] {
            let error = Pick::parse(path).unwrap_err();
            assert!(error.starts_with(&format!("at {at}:")), "{path}: {error}");
        }
    }

    #[test]
    fn a_dot_alone_is_the_whole_answer_on_one_line() {
        let document = json!({"activeHooks": [{"capId": "security"}], "skippedHooks": []});
        assert_eq!(
            picked(".", &document),
            "{\"activeHooks\":[{\"capId\":\"security\"}],\"skippedHooks\":[]}\n"
        );
        assert_eq!(picked(".", &json!("s")), "s\n");
    }

    #[test]
    fn an_index_is_digits_without_a_leading_zero() {
        assert_eq!(Pick::parse(".a[0]").unwrap().0[1], Step::Index(0));
        assert_eq!(Pick::parse(".a[10]").unwrap().0[1], Step::Index(10));
    }

    #[test]
    fn a_number_prints_as_serde_json_writes_an_f64() {
        // not `arbitrary_precision`: a number is an i64, a u64 or an f64, and an f64 keeps its `.0`
        let document: Value =
            serde_json::from_str(r#"{"a": 1.0, "b": -0, "c": 1e3, "d": 7}"#).unwrap();
        assert_eq!(picked(".a", &document), "1.0\n");
        assert_eq!(picked(".b", &document), "-0.0\n");
        assert_eq!(picked(".c", &document), "1000.0\n");
        assert_eq!(picked(".d", &document), "7\n");
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
    fn an_item_is_selected_by_any_string_field() {
        let settings = json!({"extensions": [{"settings": [
            {"name": "workflow.auto_advance", "value": true}, {"name": "workflow.other", "value": "x"},
            {"name": "workflow.unset"},
        ]}]});
        let path = ".extensions[].settings[name=workflow.auto_advance].value";
        assert_eq!(picked(path, &settings), "true\n");
        assert_eq!(
            picked(
                ".extensions[].settings[name=workflow.unset].value",
                &settings
            ),
            ""
        );
        let diagnostics = json!([{"code": "gsd/W205"}, {"code": "E003"}, {"code": 205}]);
        assert_eq!(picked(".[code=gsd/W205].code", &diagnostics), "gsd/W205\n");
        assert_eq!(picked("[code=E003].code", &diagnostics), "E003\n");
        // a number is compared as JSON, and the value may hold '=' and '.'
        assert_eq!(picked(".[code=205].code", &diagnostics), "205\n");
        assert_eq!(
            picked(".[a=b=c.d]", &json!([{"a": "b=c.d"}])),
            "{\"a\":\"b=c.d\"}\n"
        );
    }

    #[test]
    fn a_dotted_key_selects_by_a_nested_field() {
        let nodes = json!({"nodes": [
            {"id": "t1", "fields": {"result": "issue"}}, {"id": "t2", "fields": {"result": "pass"}},
            {"id": "t3", "fields": {}}, {"id": "t4"}, {"id": "t5", "fields": "issue"},
        ]});
        assert_eq!(picked(".nodes[fields.result=issue].id", &nodes), "t1\n");
        assert_eq!(picked(".nodes[fields.result=pass].id", &nodes), "t2\n");
        assert_eq!(picked(".nodes[fields.missing=x].id", &nodes), "");
        assert_eq!(
            Pick::parse(".nodes[fields.result=issue]").unwrap().0[1],
            Step::Field(vec!["fields".into(), "result".into()], "issue".into())
        );
        for path in [".a[.x=1]", ".a[x.=1]", ".a[x..y=1]"] {
            assert!(
                Pick::parse(path).unwrap_err().starts_with("at byte 2:"),
                "{path}"
            );
        }
    }

    #[test]
    fn a_scalar_that_is_not_a_string_is_compared_as_json() {
        let nodes = json!({"nodes": [
            {"id": "a", "fields": {"backlog": true, "wave": 2}},
            {"id": "b", "fields": {"backlog": false, "wave": 1}},
            {"id": "c", "fields": {"backlog": "true", "wave": "2"}},
            {"id": "d", "fields": {"backlog": [true], "wave": {"n": 2}}},
            {"id": "e", "fields": {"backlog": null}},
        ]});
        assert_eq!(picked(".nodes[fields.backlog=true].id", &nodes), "a\nc\n");
        assert_eq!(picked(".nodes[fields.backlog=false].id", &nodes), "b\n");
        assert_eq!(picked(".nodes[fields.wave=2].id", &nodes), "a\nc\n");
        assert_eq!(picked(".nodes[fields.backlog=null].id", &nodes), "e\n");
        // an object or an array is never selected
        assert_eq!(picked(".nodes[fields.wave={\"n\":2}].id", &nodes), "");
    }

    #[test]
    fn an_answer_that_is_not_json_is_refused_and_an_empty_one_prints_nothing() {
        let pick = Pick::parse(".a").unwrap();
        assert_eq!(pick.printed("{\"a\": 1}\n"), Ok("1\n".into()));
        assert_eq!(pick.printed(""), Ok(String::new()));
        assert!(pick.printed("done").unwrap_err().contains("not JSON"));
    }
}

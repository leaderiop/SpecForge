//! Settings read at render time. Upstream's workflows read a setting with `gsd-tools config-get <key>`
//! and fall back to a default. A ported line cannot run a tool at that point, and a literal default would make the
//! prompt route on the default whatever the project sets, so the line carries `{{setting:<key>|<fallback>}}` and the
//! render replaces it with the value of the settings block (`settings::block()`), or the fallback for a key nothing sets.

use super::shell::in_double_quotes;
use super::token;

/// What a `{{setting:<key>|<fallback>}}` token takes: the setting's value in `block` (a JSON object), escaped for double
/// quotes, else the fallback; the fallback `@host` is `runtime`, the runtime the text is rendered for (`claude`,
/// `codex`, `cursor`).
pub(crate) fn value_of(block: &str, runtime: &str) -> impl Fn(&str, &str) -> Option<String> {
    let values: serde_json::Value = serde_json::from_str(block).unwrap_or_default();
    let runtime = runtime.to_string();
    move |key, fallback| {
        Some(match values.get(key) {
            // a string as it is, a list or number as JSON
            Some(serde_json::Value::String(s)) => in_double_quotes(s),
            Some(other) => in_double_quotes(&other.to_string()),
            None if fallback == "@host" => runtime.clone(),
            None => fallback.to_string(),
        })
    }
}

/// `text` with each `{{setting:<key>|<fallback>}}` replaced as [`value_of`] says.
pub fn fill(text: &str, block: &str, runtime: &str) -> String {
    token::fill(text, &[("setting", &value_of(block, runtime))], None)
}

#[cfg(test)]
mod tests {
    use super::fill;

    const BLOCK: &str = r#"{"workflow.discuss_mode": "assumptions", "context_window": 1000000, "workflow.use_x": true, "review.default_reviewers": ["a", "b"], "workflow.test_command": "echo \"$HOME\" `id`"}"#;

    #[test]
    fn a_set_key_is_replaced_by_its_value() {
        let text = "M=\"{{setting:workflow.discuss_mode|discuss}}\" W=\"{{setting:context_window|200000}}\" B=\"{{setting:workflow.use_x|false}}\"";
        assert_eq!(
            fill(text, BLOCK, "claude"),
            "M=\"assumptions\" W=\"1000000\" B=\"true\""
        );
    }

    #[test]
    fn an_unset_key_takes_its_fallback() {
        assert_eq!(
            fill("{{setting:response_language|}}.", BLOCK, "claude"),
            "."
        );
        assert_eq!(fill("{{setting:runtime|@host}}", "{}", "codex"), "codex");
        assert_eq!(
            fill("{{setting:runtime|@host}}", "not json", "cursor"),
            "cursor"
        );
    }

    #[test]
    fn a_list_is_json_and_a_string_cannot_leave_its_quotes() {
        assert_eq!(
            fill("\"{{setting:review.default_reviewers|}}\"", BLOCK, "claude"),
            "\"[\\\"a\\\",\\\"b\\\"]\""
        );
        assert_eq!(
            fill("\"{{setting:workflow.test_command|}}\"", BLOCK, "claude"),
            "\"echo \\\"\\$HOME\\\" \\`id\\`\""
        );
    }

    #[test]
    fn an_open_token_is_left_alone() {
        assert_eq!(fill("a {{setting:x|y", BLOCK, "claude"), "a {{setting:x|y");
    }
}

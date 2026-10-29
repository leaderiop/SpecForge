//! Settings read at render time (REVIEW-r42 M1). Upstream's workflows read a setting with `gsd-tools config-get <key>`
//! and fall back to a default. A ported line cannot run a tool at that point, and a literal default would make the
//! prompt route on the default whatever the project sets, so the line carries `{{setting:<key>|<fallback>}}` and the
//! render replaces it with the value of the settings block (`settings::block()`), or the fallback for a key nothing sets.

const OPEN: &str = "{{setting:";
const CLOSE: &str = "}}";

/// A value as shell text inside double quotes: a string as it is, a list or number as JSON; `"`, `\`, `$` and a
/// backtick are escaped so a setting can never end the quote or run a command.
fn shell_text(value: &serde_json::Value) -> String {
    let raw = match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `text` with each `{{setting:<key>|<fallback>}}` replaced by the setting's value in `block` (a JSON object), else
/// the fallback; the fallback `@host` is `runtime`, the runtime the text is rendered for (`claude`, `codex`, `cursor`).
pub fn fill(text: &str, block: &str, runtime: &str) -> String {
    let values: serde_json::Value = serde_json::from_str(block).unwrap_or_default();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let after = &rest[at + OPEN.len()..];
        let Some(end) = after.find(CLOSE) else {
            out.push_str(&rest[at..]);
            return out;
        };
        let (key, fallback) = after[..end].split_once('|').unwrap_or((&after[..end], ""));
        match values.get(key.trim()) {
            Some(value) => out.push_str(&shell_text(value)),
            None => out.push_str(if fallback == "@host" {
                runtime
            } else {
                fallback
            }),
        }
        rest = &after[end + CLOSE.len()..];
    }
    out.push_str(rest);
    out
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

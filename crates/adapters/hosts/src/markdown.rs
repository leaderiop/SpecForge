//! Markdown files with YAML frontmatter, as hosts read agents, skills and commands.

use specforge_protocol_types::Attribution;

/// `value` as a YAML scalar: plain when it is safe, else a double-quoted string.
pub fn yaml(value: &str) -> String {
    let plain = !value.is_empty()
        && value.trim() == value
        // Any control character or non-ASCII one may be a YAML line break (CR, U+0085, U+2028): quoted and escaped.
        && value.chars().all(|c| (' '..='~').contains(&c))
        && !value.contains(['\n', '"', '\'', '#', '{', '}', '[', ']', '`'])
        && !value.contains(": ")
        && !value.ends_with(':')
        && !value.starts_with(['-', '?', ':', ',', '&', '*', '!', '|', '>', '%', '@'])
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
        )
        && value.parse::<f64>().is_err();
    if plain {
        value.to_string()
    } else {
        serde_json::Value::String(value.to_string())
            .to_string()
            .replace('\u{85}', "\\u0085")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029")
    }
}

/// How long a short description may be.
const SHORT_DESCRIPTION: usize = 180;

pub fn short(description: &str) -> String {
    if description.chars().count() <= SHORT_DESCRIPTION {
        return description.to_string();
    }
    let cut: String = description.chars().take(SHORT_DESCRIPTION - 3).collect();
    format!("{}...", cut.trim_end())
}

/// A frontmatter line `key: value`.
pub fn field(key: &str, value: &str) -> String {
    format!("{key}: {}\n", yaml(value))
}

/// A frontmatter block list.
pub fn list(key: &str, items: &[String]) -> String {
    let mut out = format!("{key}:\n");
    for item in items {
        out.push_str(&format!("  - {}\n", yaml(item)));
    }
    out
}

/// The notice a file derived from ported text ends with.
pub fn notice(a: &Attribution) -> String {
    format!(
        "<!-- Ported from {} {} at {}. {}. Licensed under {}. -->\n",
        a.upstream,
        a.path,
        a.version,
        a.copyright.trim_end_matches('.'),
        a.license
    )
}

/// A document: the frontmatter between its fences, a blank line, the body, and the notice.
pub fn document(frontmatter: &str, body: &str, attribution: Option<&Attribution>) -> String {
    let mut out = format!("---\n{frontmatter}---\n\n{}\n", body.trim_end());
    if let Some(a) = attribution {
        out.push('\n');
        out.push_str(&notice(a));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_quotes_what_yaml_would_misread() {
        assert_eq!(yaml("Plans a phase"), "Plans a phase");
        assert_eq!(yaml("a: b"), "\"a: b\"");
        assert_eq!(yaml("yes"), "\"yes\"");
        assert_eq!(yaml("3"), "\"3\"");
        assert_eq!(yaml("<phase> [--gaps]"), "\"<phase> [--gaps]\"");
        assert_eq!(yaml("line\nbreak"), "\"line\\nbreak\"");
        // Every YAML line break is quoted, so a description cannot add a frontmatter key.
        for text in [
            "x\rpermissionMode:bypass",
            "x\u{85}y",
            "x\u{2028}y",
            "x\u{2029}y",
            "tab\there",
        ] {
            let out = yaml(text);
            assert!(out.starts_with('"'), "{out:?}");
            assert!(
                !out.contains(['\r', '\u{85}', '\u{2028}', '\u{2029}']),
                "{out:?}"
            );
        }
    }
}

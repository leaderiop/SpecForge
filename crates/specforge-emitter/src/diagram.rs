//! What the diagram renderers share: the graph's DOT export (`dot`), the
//! model's (`model::dot`) and the outline's (`outline::dot`, and the outline's
//! Mermaid flowchart). They draw different things (entities and edges, a
//! kind's fields as an HTML table, one record per extension), so each keeps
//! its own layout; the syntax they all need is here, once.

/// Escape a string for safe inclusion inside a DOT quoted string.
/// Titles and descriptions are user-controlled: unescaped quotes
/// terminate the label early, backslashes form invalid escapes, and
/// raw newlines split the statement (C13-04).
pub(crate) fn escape_dot(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a string for one field of a DOT record label (`shape=record`),
/// inside its quoted string: what [`escape_dot`] escapes, and the record
/// syntax (`{`, `}`, `|`, `<`, `>`) a name or version must not open, split
/// or port. The template's own separators are written unescaped around it.
pub(crate) fn escape_record_field(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in escape_dot(text).chars() {
        if matches!(ch, '{' | '}' | '|' | '<' | '>') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Escape a string for the text or an attribute value of a DOT HTML-like
/// label (`label=<...>`). Graphviz finds the label's end by balancing
/// `<` and `>`, so a bare `>` (a `-> target` marker, a `list<T>` type)
/// ends it early; DOT string escapes do not apply there.
pub(crate) fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// An extension name as a bare DOT or Mermaid identifier: `@scope/name-x`
/// becomes `scope_name_x`.
pub(crate) fn extension_id(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '@')
        .map(|c| if c == '/' || c == '-' { '_' } else { c })
        .collect()
}

/// The colour an extension's nodes and clusters are drawn in: the
/// `theme_color` its manifest declares, when it is a hex colour (`#rgb`,
/// `#rrggbb` or `#rrggbbaa`), or a neutral grey. The value is written into
/// DOT attributes and Mermaid styles as is, so nothing else is accepted.
pub(crate) fn theme_color(declared: Option<&str>) -> &str {
    declared
        .filter(|color| is_hex_color(color))
        .unwrap_or(FALLBACK_COLOR)
}

fn is_hex_color(color: &str) -> bool {
    color.strip_prefix('#').is_some_and(|digits| {
        matches!(digits.len(), 3 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
    })
}

/// The colour of an extension that declares none.
const FALLBACK_COLOR: &str = "#95a5a6";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(
            escape_dot("say \"hi\"\\done\nnext"),
            "say \\\"hi\\\"\\\\done\\nnext"
        );
    }

    #[test]
    fn record_fields_cannot_open_split_or_port_the_record() {
        assert_eq!(
            escape_record_field("a|b {c} <p> \"q\""),
            "a\\|b \\{c\\} \\<p\\> \\\"q\\\""
        );
    }

    #[test]
    fn html_label_text_cannot_close_the_label() {
        assert_eq!(
            escape_html("-> list<T> & \"x\""),
            "-&gt; list&lt;T&gt; &amp; &quot;x&quot;"
        );
    }

    #[test]
    fn only_a_hex_theme_color_is_drawn() {
        assert_eq!(theme_color(Some("#4a90d9")), "#4a90d9");
        assert_eq!(theme_color(Some("#abc")), "#abc");
        assert_eq!(theme_color(Some("red\" penwidth=\"9")), FALLBACK_COLOR);
        assert_eq!(theme_color(Some("#12345")), FALLBACK_COLOR);
        assert_eq!(theme_color(None), FALLBACK_COLOR);
    }

    #[test]
    fn extension_ids_are_bare_identifiers() {
        assert_eq!(
            extension_id("@specforge/cargo-test"),
            "specforge_cargo_test"
        );
    }
}

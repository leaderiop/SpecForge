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

/// An extension name as a bare DOT or Mermaid identifier: `@scope/name-x`
/// becomes `scope_name_x`.
pub(crate) fn extension_id(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '@')
        .map(|c| if c == '/' || c == '-' { '_' } else { c })
        .collect()
}

/// The colour an extension's nodes and clusters are drawn in.
pub(crate) fn extension_color(name: &str) -> &'static str {
    // Exact slug match (text after the final '/'), never substring: an
    // extension named "governance-tools-plus" must not inherit the
    // governance palette (C13-03).
    let slug = name.rsplit('/').next().unwrap_or(name);
    EXTENSION_COLORS
        .iter()
        .find(|(key, _)| slug == *key)
        .map_or(FALLBACK_COLOR, |(_, color)| color)
}

const EXTENSION_COLORS: &[(&str, &str)] = &[
    ("product", "#2ecc71"),
    ("software", "#4a90d9"),
    ("governance", "#e74c3c"),
    ("formal", "#9b59b6"),
];

/// The colour of an extension that declares none.
pub(crate) const FALLBACK_COLOR: &str = "#95a5a6";

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
    fn extension_ids_are_bare_identifiers() {
        assert_eq!(
            extension_id("@specforge/cargo-test"),
            "specforge_cargo_test"
        );
    }
}

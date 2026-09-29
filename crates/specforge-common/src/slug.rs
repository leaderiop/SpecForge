//! The one slug algorithm for obligation texts.
//!
//! A `verify` text becomes a slug when a test is linked to it by name
//! (`add_item__rejects_a_duplicate_item`) and when the graph export lists
//! the slugs a test may use. `specforge-test` carries a copy of this
//! algorithm; both are held to `tests/fixtures/slug-cases.json`, so they
//! can't drift.

/// The slug of an obligation text: comparison operators become words
/// (`<=` → `lte`, `>=` → `gte`, `<` → `lt`, `>` → `gt`), spaces become
/// underscores, ASCII letters are lowercased, everything else outside
/// `[a-z0-9_]` is dropped, runs of underscores collapse to one and
/// leading or trailing underscores are trimmed.
pub fn slug(text: &str) -> String {
    let text = text
        .replace("<=", " lte ")
        .replace(">=", " gte ")
        .replace('<', " lt ")
        .replace('>', " gt ");
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let ch = match ch {
            ' ' | '_' => '_',
            'a'..='z' | '0'..='9' => ch,
            'A'..='Z' => ch.to_ascii_lowercase(),
            _ => continue,
        };
        if ch == '_' && out.ends_with('_') {
            continue;
        }
        out.push(ch);
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    #[derive(serde::Deserialize)]
    struct Case {
        text: String,
        slug: String,
    }

    #[specforge_test(
        behavior = "slug_obligation_text",
        verify = "slug matches the shared test vectors"
    )]
    fn slug_matches_the_shared_test_vectors() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../tests/fixtures/slug-cases.json")).unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            assert_eq!(slug(&case.text), case.slug, "slug of {:?}", case.text);
            assert_eq!(slug(&case.slug), case.slug, "slug is idempotent");
        }
    }
}

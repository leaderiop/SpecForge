//! Loading the old builtin `@specforge/product` beside `@gsd/core`: the kinds both declare are E026, as
//! `gsd/README.md` says.

use crate::harness::project;

/// The kinds `gsd/README.md` says `@specforge/product` declares beside `@gsd/core`: the code spans between
/// `@specforge/product` and `@specforge/governance` in the paragraph that names E026, sorted.
fn kinds_the_readme_names() -> Vec<String> {
    let readme = include_str!("../../../README.md");
    let block = gsd_kit::markdown::inline_blocks(readme)
        .into_iter()
        .find(|b| b.text.contains("E026"))
        .expect("the README's E026 paragraph");
    let spans: Vec<String> = gsd_kit::markdown::inline_code(&block.text)
        .into_iter()
        .map(|(_, span)| span)
        .collect();
    let from = spans
        .iter()
        .rposition(|s| s == "@specforge/product")
        .expect("the paragraph names @specforge/product");
    let mut kinds: Vec<String> = spans[from + 1..]
        .iter()
        .take_while(|s| !s.starts_with('@'))
        .cloned()
        .collect();
    kinds.sort();
    kinds
}

/// Loading `@specforge/product` beside `@gsd/core` registers two kinds twice: E026 on each, the first registration
/// (`@gsd/core`'s) winning, exactly as `gsd/README.md` says.
#[test]
fn specforge_product_beside_gsd_core_is_e026_on_two_kinds() {
    let p = project("empty").with_builtins(&["@specforge/product"]);
    let quoted = regex::Regex::new(r"^entity kind '([a-z_]+)' registered by").expect("a pattern");
    let mut kinds: Vec<String> = p
        .checked()
        .into_iter()
        .filter(|(code, _)| code == "E026")
        .map(|(_, message)| {
            quoted
                .captures(&message)
                .unwrap_or_else(|| panic!("an E026 message names its kind: {message}"))[1]
                .to_string()
        })
        .collect();
    kinds.sort();
    assert_eq!(kinds, ["deliverable", "milestone"]);
    assert_eq!(kinds, kinds_the_readme_names());
}

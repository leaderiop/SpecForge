//! `docs/extension-sdk.md` names only what the SDK has.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

fn doc() -> String {
    std::fs::read_to_string(specforge_repo_root::repo_root().join("docs/extension-sdk.md"))
        .expect("the SDK doc exists")
}

/// The text of every fenced ```rust block of `doc`, in order, with the line each starts after.
fn rust_blocks(doc: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (number, line) in doc.lines().enumerate() {
        match (&mut current, line.trim_end()) {
            (None, "```rust") => current = Some((number, String::new())),
            (Some(_), "```") => blocks.extend(current.take()),
            (Some((_, text)), line) => {
                text.push_str(line);
                text.push('\n');
            }
            _ => {}
        }
    }
    blocks
}

#[test]
fn the_docs_complete_example_is_the_greet_fixture() {
    let doc = doc();
    let heading = doc
        .lines()
        .position(|l| l == "### Complete Example")
        .expect("the doc has a Complete Example");
    let (_, example) = rust_blocks(&doc)
        .into_iter()
        .find(|(start, _)| *start > heading)
        .expect("a rust block follows the heading");
    let fixture = std::fs::read_to_string(
        specforge_repo_root::repo_root().join("fixtures/greet-extension/src/contributions.rs"),
    )
    .expect("the greet fixture exists");
    assert_eq!(
        example.trim_end(),
        fixture.trim_end(),
        "the doc's example is fixtures/greet-extension/src/contributions.rs"
    );
}

#[test]
fn the_docs_name_no_attribute_the_sdk_lacks() {
    const ALLOWED: &[&str] = &[
        "extension",
        "specforge_extension_sdk::extension",
        "cfg",
        "test",
        "specforge_test",
        "derive",
        "allow",
    ];
    for (_, block) in rust_blocks(&doc()) {
        for line in block.lines() {
            let Some(rest) = line.trim_start().strip_prefix("#[") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            assert!(
                ALLOWED.contains(&name.as_str()),
                "the doc uses #[{name}], which the SDK does not have: {line}"
            );
        }
    }
}

/// Every `specforge-*` dependency line an author is shown names a crate on crates.io (the SDK, its macros, the
/// guest wire) at the SDK's own version, or a path inside this repository (ADR 0238 D4).
#[test]
#[ignore = "flipped by T7"]
fn the_docs_give_an_author_only_the_published_sdk_at_its_version() {
    const PUBLISHED: &[&str] = &[
        "specforge-extension-sdk",
        "specforge-extension-sdk-macros",
        "specforge-protocol-types",
    ];
    let version = env!("CARGO_PKG_VERSION");
    for file in [
        "docs/extension-sdk.md",
        "docs/guides/extending-specforge.md",
        "crates/runtime/extension-sdk/README.md",
    ] {
        let text = std::fs::read_to_string(specforge_repo_root::repo_root().join(file)).unwrap();
        for line in text.lines() {
            let Some((name, requirement)) = line.trim_start().split_once(" = ") else {
                continue;
            };
            if !name.starts_with("specforge-") || requirement.contains("path = ") {
                continue;
            }
            assert!(
                PUBLISHED.contains(&name),
                "{file}: {name} is not on crates.io: {line}"
            );
            assert!(
                requirement.starts_with(&format!("\"{version}\""))
                    || requirement.contains(&format!("version = \"{version}\"")),
                "{file}: {line} does not name the SDK's version {version}"
            );
        }
    }
}

//! The spec-link attributes of Rust tests (`#[spec(..)]`, `#[specforge_test(..)]`,
//! `#[specforge_test_macros::test(..)]`), read with `syn` without building or running a test. One reader for
//! check-links, which compares each `verify` text with what its entity declares, and absorb, which turns an
//! invariant's obligations into proofs of the tests that state them.

use std::path::Path;
use syn::ext::IdentExt;
use syn::visit::Visit;
use syn::{Attribute, Ident, LitStr, Token};

/// Directories a walk never enters.
const SKIPPED_DIRS: [&str; 3] = ["target", ".git", "node_modules"];

/// One test attribute: `<kind> = "<entity id>"` and its `verify` text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The file, relative to the root read.
    pub file: String,
    /// The line of the attribute's `#`.
    pub line: usize,
    /// The line of the entity id's string: where a source reader places the name the attribute gives.
    pub id_line: usize,
    pub id: String,
    pub verify: Option<String>,
}

/// Every link in the `.rs` files under each of `scan` (directories relative to `root`), in file order; a file
/// whose path from `root` starts with one of `skip` is not read.
pub fn read(root: &Path, scan: &[String], skip: &[&str]) -> Vec<Link> {
    let mut links = Vec::new();
    for dir in scan {
        let walk = walkdir::WalkDir::new(root.join(dir))
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !e.file_type().is_dir()
                    || !SKIPPED_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
            });
        for entry in walk.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(path);
            if skip.iter().any(|prefix| relative.starts_with(prefix)) {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            let Ok(file) = syn::parse_file(&source) else {
                eprintln!("links: skipped {}: does not parse", path.display());
                continue;
            };
            let mut visitor = Links {
                file: relative.display().to_string(),
                links: &mut links,
            };
            visitor.visit_file(&file);
        }
    }
    links
}

struct Links<'a> {
    file: String,
    links: &'a mut Vec<Link>,
}

impl<'ast> Visit<'ast> for Links<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if !is_test_attribute(attr) {
            return;
        }
        let Ok((id, verify)) = attr.parse_args_with(parse_args) else {
            return;
        };
        self.links.push(Link {
            file: self.file.clone(),
            line: attr.pound_token.spans[0].start().line,
            id_line: id.span().start().line,
            id: id.value(),
            verify,
        });
    }
}

/// The names the test attribute goes by (the macro's own `registration`).
fn is_test_attribute(attr: &Attribute) -> bool {
    let names: Vec<String> = attr
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    matches!(
        names.as_slice(),
        ["spec"]
            | ["specforge_test"]
            | [
                ..,
                "specforge_test_macros" | "specforge_test" | "specforge",
                "test"
            ]
    )
}

/// `<kind> = "<id>" [, verify = "<text>"] [, __guard_only]`.
fn parse_args(input: syn::parse::ParseStream) -> syn::Result<(LitStr, Option<String>)> {
    let _kind = Ident::parse_any(input)?;
    let _eq: Token![=] = input.parse()?;
    let id: LitStr = input.parse()?;
    let mut verify = None;
    while !input.is_empty() {
        let _comma: Token![,] = input.parse()?;
        if input.is_empty() {
            break;
        }
        let key: Ident = input.parse()?;
        if key == "verify" {
            let _eq: Token![=] = input.parse()?;
            let text: LitStr = input.parse()?;
            verify = Some(text.value());
        }
    }
    Ok((id, verify))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test's unwrap is its assertion: a None or an Err fails the test"
    )]

    use super::*;

    #[test]
    fn a_link_records_the_line_of_its_entity_id() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("skipped")).unwrap();
        let source = "#[test]\n\
                      #[specforge_test_macros::test(\n    \
                          invariant = \"writes_are_durable\",\n    \
                          verify = \"a killed writer leaves the old entry\"\n\
                      )]\n\
                      fn survives() {}\n\
                      #[spec(behavior = \"write_entry\")]\n\
                      fn writes() {}\n";
        std::fs::write(dir.path().join("src/lib.rs"), source).unwrap();
        std::fs::write(dir.path().join("skipped/lib.rs"), source).unwrap();
        let links = read(
            dir.path(),
            &["src".to_string(), "skipped".to_string()],
            &["skipped"],
        );
        assert_eq!(
            links,
            [
                Link {
                    file: "src/lib.rs".to_string(),
                    line: 2,
                    id_line: 3,
                    id: "writes_are_durable".to_string(),
                    verify: Some("a killed writer leaves the old entry".to_string()),
                },
                Link {
                    file: "src/lib.rs".to_string(),
                    line: 7,
                    id_line: 7,
                    id: "write_entry".to_string(),
                    verify: None,
                },
            ]
        );
    }
}

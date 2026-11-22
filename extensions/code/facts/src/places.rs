//! Where a report's location lies among the served facts: the innermost `item` holding a file's line, and the
//! `package` holding a file or a directory (ADR 0388).

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::{Fact, FactLocation};

/// The served `item` facts by the files their places are on, and the served `package` facts by their directories.
#[derive(Debug, Default)]
pub struct Places<'a> {
    /// File → each place of an item on it, with the item's id.
    items: BTreeMap<&'a str, Vec<(&'a FactLocation, &'a str)>>,
    /// Each package's `(path, id)`; `path` is project-relative, `""` at the root (and when the field is absent).
    packages: Vec<(&'a str, &'a str)>,
}

impl<'a> Places<'a> {
    /// The places of `served`'s `item` facts, by file, and the directories of its `package` facts.
    pub fn new(served: &'a [Fact]) -> Places<'a> {
        let mut places = Places::default();
        for fact in served {
            match fact.kind.as_str() {
                "item" => {
                    for place in &fact.at {
                        places
                            .items
                            .entry(place.file.as_str())
                            .or_default()
                            .push((place, fact.id.as_str()));
                    }
                }
                "package" => {
                    let path = fact.fields.get("path").and_then(|p| p.as_str());
                    places.packages.push((path.unwrap_or(""), fact.id.as_str()));
                }
                _ => {}
            }
        }
        places
    }

    /// The id of the item whose place on `file` holds the 1-based `line`; among several, the one of fewest lines, then
    /// the smaller id. `None` when no served item is there.
    pub fn innermost(&self, file: &str, line: usize) -> Option<&'a str> {
        self.items
            .get(file)?
            .iter()
            .filter(|(place, _)| holds(place, line))
            .min_by_key(|(place, id)| (place.end_line.saturating_sub(place.start_line), *id))
            .map(|(_, id)| *id)
    }

    /// The id of the package whose directory is the longest one holding `file` (`""` holds every file, `crates/a` holds
    /// `crates/a/x.rs` but not `crates/ab/x.rs`); the smaller id of two at one directory.
    pub fn package_of(&self, file: &str) -> Option<&'a str> {
        self.packages
            .iter()
            .filter(|(path, _)| path.is_empty() || below(file, path))
            .min_by_key(|(path, id)| (std::cmp::Reverse(path.len()), *id))
            .map(|(_, id)| *id)
    }

    /// The ids of the packages of purl type `purl_type` (`pkg:<purl_type>/…`) at the directory `dir`, else of those
    /// under it (`""` covers all), sorted. The type is filtered first, so a root `Cargo.lock` beside a root npm package
    /// still finds the cargo members below it.
    pub fn packages_of_dir(&self, dir: &str, purl_type: &str) -> Vec<&'a str> {
        let prefix = format!("pkg:{purl_type}/");
        let typed: Vec<(&str, &'a str)> = self
            .packages
            .iter()
            .filter(|(_, id)| id.starts_with(&prefix))
            .copied()
            .collect();
        let mut found: Vec<&'a str> = typed
            .iter()
            .filter(|(path, _)| *path == dir)
            .map(|(_, id)| *id)
            .collect();
        if found.is_empty() {
            found = typed
                .iter()
                .filter(|(path, _)| dir.is_empty() || below(path, dir))
                .map(|(_, id)| *id)
                .collect();
        }
        found.sort_unstable();
        found
    }

    /// The served package ids equal to `base` or starting with `base#` (a second package of one name), sorted.
    pub fn packages_named(&self, base: &str) -> Vec<&'a str> {
        let mut found: Vec<&'a str> = self
            .packages
            .iter()
            .map(|(_, id)| *id)
            .filter(|id| {
                id.strip_prefix(base)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('#'))
            })
            .collect();
        found.sort_unstable();
        found
    }
}

/// Whether `place` holds the 1-based `line`: from its start line to its end, which is exclusive, so a place ending at
/// column 1 of a line does not hold that line.
fn holds(place: &FactLocation, line: usize) -> bool {
    place.start_line <= line
        && (line < place.end_line || (line == place.end_line && place.end_col > 1))
}

/// Whether `path` lies below the directory `dir`: it starts with `dir/`.
fn below(path: &str, dir: &str) -> bool {
    path.strip_prefix(dir)
        .is_some_and(|rest| rest.starts_with('/'))
}

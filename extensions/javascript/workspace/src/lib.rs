//! `specforge-javascript-workspace`: the package of a JavaScript project that owns a file, named once for every pack.
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. A guest has no file
//! system (ADR 0293), so a pack declares [`packages_input`], the host serves every `package.json` with its text, and
//! [`Workspace::of`] reads them with `serde_json`. A runner pack (`@javascript/node-test`) names a test, and a source
//! pack (`@javascript/source`) an item, after the [`Workspace::owner`] of its file: the nearest named package at or
//! above it, so the tests and the items of one file name one package.
//!
//! A `package.json` that could not be read, whose text is not JSON, or whose `name` is missing or not a string is no
//! package: the file falls to the named package above it.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

#[cfg(test)]
mod tests;

/// The key of the read holding every `package.json` with its text.
pub const PACKAGES: &str = "packages";

/// The read a pack declares so [`Workspace::of`] can name packages: every `package.json` of the project, with text.
pub fn packages_input() -> Input {
    Input::files(PACKAGES, FileReadSpec::glob("**/package.json").text())
}

/// A named package of the project, as its `package.json` gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Its directory, project-relative, `""` at the root.
    pub dir: String,
    pub name: String,
    /// Its `version`, when it is a string.
    pub version: Option<String>,
    /// `<engine> <range>` for each string of its `engines`, by engine.
    pub engines: Vec<String>,
    /// The names its `dependencies`, `devDependencies`, `peerDependencies` and `optionalDependencies` list, sorted and
    /// deduplicated: the project's own packages and the registry's alike.
    pub depends_on: Vec<String>,
}

impl Package {
    /// The path of the project-relative `file` under this package's directory, when the file is in it.
    pub fn holds<'f>(&self, file: &'f str) -> Option<&'f str> {
        if self.dir.is_empty() {
            Some(file)
        } else {
            file.strip_prefix(self.dir.as_str())?.strip_prefix('/')
        }
    }

    /// The package a `package.json` at the project-relative `path` declares with `text`; `None` when it names none.
    fn read(path: &str, text: &str) -> Option<Package> {
        let json: Value = serde_json::from_str(text).ok()?;
        let name = json.get("name")?.as_str()?.to_string();
        let dir = path.strip_suffix("package.json")?.trim_end_matches('/');
        let engines = json
            .get("engines")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(engine, range)| Some(format!("{engine} {}", range.as_str()?)))
            .collect();
        let mut depends_on: Vec<String> = [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ]
        .iter()
        .filter_map(|table| json.get(table)?.as_object())
        .flat_map(|table| table.keys().cloned())
        .collect();
        depends_on.sort();
        depends_on.dedup();
        Some(Package {
            dir: dir.to_string(),
            name,
            version: json
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
            engines,
            depends_on,
        })
    }
}

/// The named packages of a project, from the `package.json` files the host read for [`packages_input`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    packages: Vec<Package>,
}

impl Workspace {
    /// The packages the host read for `input`, in the order it read them (by path).
    pub fn of(input: &CollectInput) -> Workspace {
        let packages = input
            .files
            .get(PACKAGES)
            .into_iter()
            .flat_map(|read| &read.facts)
            .filter(|file| file.error.is_none())
            .filter_map(|file| Package::read(&file.path, file.text.as_deref()?))
            .collect();
        Workspace { packages }
    }

    /// Every named package, by the path of its `package.json`.
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    /// The package that owns the project-relative `file`, the deepest whose directory holds it, and the path under
    /// that directory; `None` when no named package is at or above it.
    pub fn owner<'f>(&self, file: &'f str) -> Option<(&Package, &'f str)> {
        self.packages
            .iter()
            .filter_map(|package| Some((package, package.holds(file)?)))
            .max_by_key(|(package, _)| package.dir.len())
    }
}

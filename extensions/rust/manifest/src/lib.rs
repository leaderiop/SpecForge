//! `specforge-rust-manifest`: every `Cargo.toml` of a project read as cargo reads it, into `@code/core`'s
//! [`Package`]s (ADR 0348 D5).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. `@rust/cargo` hands it
//! the manifests' texts and the listing of the files cargo's autodiscovery looks at, both as the host's `files` reads
//! serve them; [`read_packages`] completes each manifest with `cargo_toml` over that listing ([`Listing`]):
//!
//! - workspace inheritance (`version.workspace = true`, `[workspace.dependencies]`) from the nearest manifest above
//!   that declares `[workspace]`, or the one `package.workspace` names;
//! - target autodiscovery (`src/lib.rs`, `src/main.rs`, `src/bin/*.rs`, `src/bin/*/main.rs`, `tests`, `examples`,
//!   `benches`, `build.rs`) and the explicit `[[bin]]`, `[[test]]`, … tables, `autotests = false` included.
//!
//! A virtual manifest is no package. Two packages of one name keep [`PackageId::cargo`] for the one whose directory
//! is shortest (then lexically first) and [`PackageId::cargo_in`] for every other. A package's `depends_on` names the
//! project's packages its path dependencies (normal, dev, build, per target platform) reach.

mod listing;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;

use cargo_toml::{Dependency, Manifest, OptionalFile, Product, Value};
use specforge_code_facts::{Package, PackageId, Target, TargetKind};
use specforge_extension_sdk::prelude::FactLocation;

pub use listing::Listing;

/// A manifest cargo's reader could not read or complete: its project-relative path and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub path: String,
    pub why: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.why)
    }
}

impl std::error::Error for ManifestError {}

/// A package as its manifest gives it, before its id is chosen.
struct Read {
    name: String,
    package: Package,
    /// The project-relative directories its path dependencies name.
    path_deps: Vec<String>,
}

/// Every package the `manifests` (each a project-relative `Cargo.toml` path and its text) declare, completed over
/// `listing` (the project-relative files autodiscovery may find; the manifests' paths count too), by directory. A
/// manifest that does not parse or complete is the error, never a dropped package.
pub fn read_packages(
    manifests: &[(String, String)],
    listing: &[String],
) -> Result<Vec<Package>, ManifestError> {
    let texts: BTreeMap<&str, &str> = manifests
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let mut read = Vec::new();
    for (path, text) in &texts {
        let error = |why: String| ManifestError {
            path: path.to_string(),
            why,
        };
        let mut manifest = Manifest::<Value>::from_slice_with_metadata(text.as_bytes())
            .map_err(|e| error(e.to_string()))?;
        if manifest.package.is_none() {
            continue;
        }
        let dir = dir_of(path);
        let fs = Listing::new(&dir, listing, &texts);
        manifest
            .complete_from_abstract_filesystem::<Value, _>(&fs, None)
            .map_err(|e| error(e.to_string()))?;
        read.push(package(path, &dir, text, &manifest));
    }
    let ids = ids(&read);
    let by_dir: BTreeMap<String, PackageId> = read
        .iter()
        .zip(&ids)
        .map(|(r, id)| (r.package.path.clone(), id.clone()))
        .collect();
    Ok(read
        .into_iter()
        .zip(&ids)
        .map(|(mut r, id)| {
            r.package.id = id.clone();
            r.package.depends_on = r
                .path_deps
                .iter()
                .filter_map(|dir| by_dir.get(dir).cloned())
                .filter(|dep| dep != id)
                .collect();
            r.package.depends_on.sort();
            r.package.depends_on.dedup();
            r.package
        })
        .collect())
}

/// The id each package takes: the plain purl for the package of its name whose directory is shortest (then
/// lexically first), the purl subpath for every other (ADR 0348 D5).
fn ids(read: &[Read]) -> Vec<PackageId> {
    let mut first: BTreeMap<&str, &str> = BTreeMap::new();
    for r in read {
        let dir = r.package.path.as_str();
        first
            .entry(r.name.as_str())
            .and_modify(|kept| {
                if (dir.len(), dir) < (kept.len(), *kept) {
                    *kept = dir;
                }
            })
            .or_insert(dir);
    }
    read.iter()
        .map(|r| match first.get(r.name.as_str()) {
            Some(kept) if *kept == r.package.path => PackageId::cargo(&r.name),
            _ => PackageId::cargo_in(&r.name, &r.package.path),
        })
        .collect()
}

/// The package `manifest` (completed, at `path` in `dir`, written as `text`) declares.
fn package(path: &str, dir: &str, text: &str, manifest: &Manifest<Value>) -> Read {
    let declared = manifest
        .package
        .as_ref()
        .expect("read_packages skips a virtual manifest");
    let name = declared.name.clone();
    let mut targets = Vec::new();
    let mut add = |kind: TargetKind, product: &Product| {
        if let (Some(name), Some(root)) = (&product.name, &product.path) {
            if let Some(root) = joined(dir, root) {
                targets.push(Target {
                    kind,
                    name: name.clone(),
                    root,
                });
            }
        }
    };
    if let Some(lib) = &manifest.lib {
        add(TargetKind::Lib, lib);
    }
    for (kind, products) in [
        (TargetKind::Bin, &manifest.bin),
        (TargetKind::Test, &manifest.test),
        (TargetKind::Example, &manifest.example),
        (TargetKind::Bench, &manifest.bench),
    ] {
        for product in products {
            add(kind, product);
        }
    }
    if let Some(OptionalFile::Path(build)) = &declared.build {
        if let Some(root) = joined(dir, &build.to_string_lossy()) {
            targets.push(Target {
                kind: TargetKind::CustomBuild,
                name: "build-script-build".to_string(),
                root,
            });
        }
    }
    let path_deps = manifest
        .dependencies
        .values()
        .chain(manifest.dev_dependencies.values())
        .chain(manifest.build_dependencies.values())
        .chain(manifest.target.values().flat_map(|platform| {
            platform
                .dependencies
                .values()
                .chain(platform.dev_dependencies.values())
                .chain(platform.build_dependencies.values())
        }))
        .filter_map(|dependency| dependency_dir(dir, dependency))
        .collect();
    Read {
        package: Package {
            id: PackageId::cargo(&name),
            version: declared.version.get().ok().cloned(),
            edition: Some(declared.edition().to_string()),
            rust_version: declared.rust_version().map(str::to_string),
            engines: Vec::new(),
            features: manifest.features.keys().cloned().collect(),
            targets,
            path: dir.to_string(),
            depends_on: Vec::new(),
            at: package_table(path, text),
        },
        name,
        path_deps,
    }
}

/// The project-relative directory a path dependency of the package in `dir` names. A path inherited from
/// `[workspace.dependencies]` is rooted at [`listing::ROOT`] (the listing's stand-in for the project root).
fn dependency_dir(dir: &str, dependency: &Dependency) -> Option<String> {
    let path = dependency.detail()?.path.as_deref()?;
    match path.strip_prefix(listing::ROOT) {
        Some(rooted) => joined("", rooted),
        None => joined(dir, path),
    }
}

/// Where `text`'s `[package]` table is named: the key's span (1-based lines and byte columns).
fn package_table(path: &str, text: &str) -> Option<FactLocation> {
    let table = toml::de::DeTable::parse(text).ok()?;
    let span = table
        .get_ref()
        .iter()
        .find(|(key, _)| key.get_ref().as_ref() == "package")?
        .0
        .span();
    let (start_line, start_col) = point(text, span.start);
    let (end_line, end_col) = point(text, span.end);
    Some(FactLocation {
        file: path.to_string(),
        start_line,
        start_col,
        end_line,
        end_col,
    })
}

/// The 1-based line and byte column of byte `offset` of `text`.
fn point(text: &str, offset: usize) -> (usize, usize) {
    let before = &text.as_bytes()[..offset.min(text.len())];
    let line = before.iter().filter(|b| **b == b'\n').count() + 1;
    let col = before
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(before.len(), |at| before.len() - at - 1)
        + 1;
    (line, col)
}

/// The directory of the project-relative manifest `path`: `""` at the root.
fn dir_of(path: &str) -> String {
    path.strip_suffix("Cargo.toml")
        .unwrap_or(path)
        .trim_end_matches('/')
        .to_string()
}

/// `path` (relative, `/` or `\` separated) under the project-relative `dir`, its `.` and `..` resolved; `None` when it
/// leaves the project.
pub(crate) fn joined(dir: &str, path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in dir.split('/').chain(path.split(['/', '\\'])) {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            segment => segments.push(segment),
        }
    }
    Some(segments.join("/"))
}

//! The `dependency` facts of a CycloneDX bill of materials (ADR 0388): what a reader (`specforge-code-cyclonedx`) read
//! of a BOM, and the facts minted from it, each citing the served project `package` the BOM describes. Whether a
//! dependency is direct is what the BOM's own dependency graph states, and stays unknown (absent) when it states none.

use std::collections::BTreeMap;
use std::str::FromStr;

use specforge_extension_sdk::prelude::Fact;

use crate::ids::PackageId;
use crate::places::Places;
use crate::report::{id_part, shown, unescape, DEPENDENCY};

/// A component's scope: CycloneDX's three, widest first. A BOM stating none means `required`, the spec's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    Required,
    Optional,
    Excluded,
}

/// Each scope and its name: the one table [`Scope::NAMES`], [`Scope::as_str`] and [`Scope::parse`] read.
const SCOPES: [(Scope, &str); 3] = [
    (Scope::Required, "required"),
    (Scope::Optional, "optional"),
    (Scope::Excluded, "excluded"),
];

impl Scope {
    /// Every scope's name, widest first: `@code/core`'s vocabulary of `dependency.scope`.
    pub const NAMES: [&'static str; 3] = {
        let mut names = [""; 3];
        let mut at = 0;
        while at < names.len() {
            names[at] = SCOPES[at].1;
            at += 1;
        }
        names
    };

    pub fn as_str(self) -> &'static str {
        SCOPES
            .iter()
            .find(|(scope, _)| *scope == self)
            .map_or("required", |(_, name)| name)
    }

    /// One of CycloneDX's three scope values; `None` for any other text.
    pub fn parse(text: &str) -> Option<Scope> {
        SCOPES
            .iter()
            .find(|(_, name)| *name == text)
            .map(|(scope, _)| *scope)
    }
}

/// What a BOM says of one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Its `bom-ref`, the name the BOM's dependency graph knows it by.
    pub bom_ref: Option<String>,
    /// Its package URL as written, qualifiers and subpath included.
    pub purl: Option<String>,
    pub group: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub scope: Scope,
}

/// What a reader read of one BOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bom {
    /// `metadata.component`: the component the BOM describes, a project package.
    pub root: Component,
    /// Every component, nested ones flattened depth first (nesting is no dependency tree).
    pub components: Vec<Component>,
    /// The `dependsOn` refs of the dependency graph's entry for the root; `None` when the BOM has no `dependencies`
    /// array, no entry for the root, or a root with no `bom-ref`: unknown, not "none".
    pub direct: Option<Vec<String>>,
}

/// The `dependency` facts of `boms`, sorted by id, each citing the served `package` its BOM describes.
///
/// A BOM's project package is a served package [`Places::packages_named`] finds by the root's purl with its
/// qualifiers (`?…`), subpath (`#…`) and version (after the last `@` of the last segment) removed, a namespace
/// starting with a literal `@` written `%40`: the one whose directory is the purl's subpath (decoded, no leading or
/// trailing `/`), else the one whose id is exactly that, else the only one. No such package, several with none of
/// them selected, or a root with no purl is an error naming the root's purl or name: a BOM about no collected package
/// is no report about this project.
///
/// The id is `dep:<package id>/<id_part(purl without qualifiers and subpath)>`, or
/// `dep:<package id>/@<id_part([group/]name[@version])>` for a component with no purl ([`id_part`] encodes `@`, so the
/// two forms never meet). Fields: `package`, `purl` as written, `name` (`group/name` with a group), `version`,
/// `scope`, and `direct` only when the BOM states a graph entry for its root and the component has a `bom-ref`.
/// Components of one id merge across `boms` (F4): the widest scope, `direct` true when any is, absent only when every
/// one is absent; the first one's other fields.
pub fn dependency_facts(boms: &[Bom], served: &[Fact]) -> Result<Vec<Fact>, String> {
    let places = Places::new(served);
    let mut merged: BTreeMap<String, Merged> = BTreeMap::new();
    for bom in boms {
        let package = project_package(&bom.root, &places)?;
        for component in &bom.components {
            let direct = match (&bom.direct, &component.bom_ref) {
                (Some(refs), Some(bom_ref)) => Some(refs.contains(bom_ref)),
                _ => None,
            };
            let id = match &component.purl {
                Some(purl) => format!("{}:{package}/{}", DEPENDENCY.scheme, id_part(bare(purl))),
                None => {
                    let mut named = full_name(component);
                    if let Some(version) = &component.version {
                        named.push('@');
                        named.push_str(version);
                    }
                    format!("{}:{package}/@{}", DEPENDENCY.scheme, id_part(&named))
                }
            };
            merged
                .entry(id)
                .and_modify(|seen| {
                    seen.scope = seen.scope.min(component.scope);
                    seen.direct = match (seen.direct, direct) {
                        (Some(a), Some(b)) => Some(a || b),
                        (a, b) => a.or(b),
                    };
                })
                .or_insert(Merged {
                    package,
                    component,
                    scope: component.scope,
                    direct,
                });
        }
    }
    Ok(merged.into_iter().map(|(id, seen)| seen.fact(id)).collect())
}

/// The components of one id seen so far: the first, and the merged scope and directness.
struct Merged<'a> {
    package: &'a str,
    component: &'a Component,
    scope: Scope,
    direct: Option<bool>,
}

impl Merged<'_> {
    fn fact(self, id: String) -> Fact {
        let mut fact = Fact::new(DEPENDENCY.kind, id).field("package", self.package);
        if let Some(purl) = &self.component.purl {
            fact = fact.field("purl", purl.as_str());
        }
        fact = fact.field("name", full_name(self.component));
        if let Some(version) = &self.component.version {
            fact = fact.field("version", version.as_str());
        }
        fact = fact.field("scope", self.scope.as_str());
        match self.direct {
            Some(direct) => fact.field("direct", direct),
            None => fact,
        }
    }
}

/// `group/name` when the component has a group, else its name.
fn full_name(component: &Component) -> String {
    match &component.group {
        Some(group) => format!("{group}/{}", component.name),
        None => component.name.clone(),
    }
}

/// `purl` without its qualifiers (`?…`) and subpath (`#…`).
fn bare(purl: &str) -> &str {
    purl.split(['?', '#']).next().unwrap_or(purl)
}

/// The id of the one served package the BOM root `root` names, else an error naming the root.
fn project_package<'a>(root: &Component, places: &Places<'a>) -> Result<&'a str, String> {
    let Some(purl) = &root.purl else {
        return Err(format!(
            "the BOM's root component '{}' has no purl, so it names no collected package",
            shown(&root.name)
        ));
    };
    let base = package_base(purl);
    let named = places.packages_named(&base);
    // The purl's subpath (`#crates/demo`) names the directory of a second package of one name.
    let subpath = purl
        .split_once('#')
        .map(|(_, subpath)| unescape(subpath).trim_matches('/').to_string());
    let by_subpath = named.iter().find(|id| {
        subpath.is_some() && PackageId::from_str(id).is_ok_and(|id| id.dir() == subpath.as_deref())
    });
    if let Some(package) = by_subpath.or_else(|| named.iter().find(|id| **id == base)) {
        return Ok(*package);
    }
    match named.as_slice() {
        [package] => Ok(*package),
        [] => Err(format!(
            "the BOM's root component '{}' names no collected package (looked for '{}')",
            shown(purl),
            shown(&base)
        )),
        several => Err(format!(
            "the BOM's root component '{}' names several collected packages: {}",
            shown(purl),
            several.join(", ")
        )),
    }
}

/// The package id a purl names: no qualifiers, subpath or version, a namespace starting with `@` written `%40`.
fn package_base(purl: &str) -> String {
    let bare = bare(purl);
    let last = bare.rfind('/').map_or(0, |at| at + 1);
    let unversioned = match bare[last..].rfind('@') {
        Some(at) => &bare[..last + at],
        None => bare,
    };
    match unversioned.split_once('/') {
        Some((head, rest)) if rest.starts_with('@') => format!("{head}/%40{}", &rest[1..]),
        _ => unversioned.to_string(),
    }
}

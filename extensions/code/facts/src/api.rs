//! The `api_item` facts of an API snapshot (ADR 0388): what a reader (`specforge-code-api`) read for each public
//! entry, and the facts minted from it, each citing the one served `item` its path names.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use specforge_extension_sdk::prelude::Fact;

use crate::ids::{ItemId, Owner, Scheme};
use crate::mint::{ItemKind, Visibility};
use crate::report::{id_part, rust_id_part, Repeats, API_ITEM};

/// What a public entry is: an item's kind, or one no `item` fact can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiKind {
    Item(ItemKind),
    /// An enum variant (Rust) or enum member (JavaScript).
    Variant,
    /// A `pub use`.
    Reexport,
    /// A `pub extern crate`.
    ExternCrate,
    /// An entry of a shape the reader knows, with a keyword it does not.
    Other,
}

/// The kinds beyond [`ItemKind`]'s, in vocabulary order.
const NON_ITEM_KINDS: [ApiKind; 4] = [
    ApiKind::Variant,
    ApiKind::Reexport,
    ApiKind::ExternCrate,
    ApiKind::Other,
];

impl ApiKind {
    /// Every kind's name: [`ItemKind::NAMES`], then `variant`, `reexport`, `extern_crate`, `other`. `@code/core`'s
    /// vocabulary of `api_item.kind`.
    pub const NAMES: [&'static str; 20] = {
        let mut names = [""; 20];
        let mut at = 0;
        while at < ItemKind::NAMES.len() {
            names[at] = ItemKind::NAMES[at];
            at += 1;
        }
        let mut more = 0;
        while more < NON_ITEM_KINDS.len() {
            names[at + more] = NON_ITEM_KINDS[more].as_str();
            more += 1;
        }
        names
    };

    /// Its name: an exhaustive match, so a new kind names itself or fails to compile.
    pub const fn as_str(self) -> &'static str {
        match self {
            ApiKind::Item(kind) => kind.as_str(),
            ApiKind::Variant => "variant",
            ApiKind::Reexport => "reexport",
            ApiKind::ExternCrate => "extern_crate",
            ApiKind::Other => "other",
        }
    }

    /// Whether an `item` fact can have this kind, so an entry of it should cite one.
    pub fn is_item(self) -> bool {
        matches!(self, ApiKind::Item(_))
    }
}

/// One public entry of an API snapshot, as its reader read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEntry {
    pub scheme: Scheme,
    /// The crate as the snapshot writes it (`example_api`), or the npm package (`@demo/widget`).
    pub container: String,
    /// Rust: the qualified path, the crate first (`[example_api, Struct, fmt]`); JavaScript: the member names below
    /// the entry point (`[Widget, render]`). Each segment as written.
    pub path: Vec<String>,
    /// The type a Rust member is defined on: its `impl`'s, or its parent type's.
    pub owner: Option<Owner>,
    pub kind: ApiKind,
    pub visibility: Visibility,
    /// The entry exactly as the snapshot wrote it: the whole line (Rust), the excerpt's text (JavaScript).
    pub signature: String,
    /// The api-extractor release tag (`Public`, `Beta`, …).
    pub tag: Option<String>,
}

/// The `api_item` facts of `entries`, each citing the one `served` `item` its path names, sorted by id:
///
/// - Rust: `api:rust:<crate>::<module>::…::<name>`, a member `…::<Type>::m` or `…::<Type as Trait>::m` as
///   [`ItemId::rust`] writes it, the crate root `api:rust:<crate>`, each part through [`id_part`] (a segment keeping a
///   plain space, as an owner's type has one). Its item is the served id of a library crate whose name, `-` read as
///   `_`, is the snapshot's crate, built from the parts as written: exactly one held, else none.
/// - JavaScript: `api:js:<package>::<name>[.<member>…]`, no source file, each part through [`id_part`] and a name's
///   own `.` written `%2E`. Its item is the one served `js` item of the package with that name path; none or several
///   cite nothing.
///
/// Encoded parts hold no `~`, so no entry can forge another's repeat suffix.
///
/// Only an entry whose kind an item can have cites one. Entries of one id with one signature merge; another
/// signature takes the next [`Repeats`] suffix, in document order (`~2`). Fields: `path` (as written), `kind`,
/// `visibility`, `signature`, `item` when cited, `tag` when stated.
pub fn api_facts(entries: &[ApiEntry], served: &[Fact]) -> Vec<Fact> {
    let held: BTreeSet<&str> = served
        .iter()
        .filter(|fact| fact.kind == "item")
        .map(|fact| fact.id.as_str())
        .collect();
    let parsed: Vec<ItemId> = held
        .iter()
        .filter_map(|id| ItemId::from_str(id).ok())
        .collect();
    let mut repeats = Repeats::default();
    let mut seen = BTreeSet::new();
    let mut facts = BTreeMap::new();
    for entry in entries {
        let (base, item, written) = match entry.scheme {
            Scheme::Rust => rust(entry, &parsed, &held),
            Scheme::Js => js(entry, &parsed),
        };
        if !seen.insert((base.clone(), entry.signature.as_str())) {
            continue;
        }
        let id = repeats.id(base);
        let mut fact = Fact::new(API_ITEM.kind, id.clone())
            .field("path", written)
            .field("kind", entry.kind.as_str())
            .field("visibility", entry.visibility.as_str())
            .field("signature", entry.signature.clone());
        if let Some(item) = item.filter(|_| entry.kind.is_item()) {
            fact = fact.field("item", item);
        }
        if let Some(tag) = &entry.tag {
            fact = fact.field("tag", tag.clone());
        }
        let replaced = facts.insert(id, fact);
        // A base never holds `~` (each part is encoded), so a repeat suffix is never met twice.
        debug_assert!(
            replaced.is_none(),
            "an api_item id minted twice: {replaced:?}"
        );
    }
    facts.into_values().collect()
}

/// A Rust entry's id, its item and its path as written.
fn rust(
    entry: &ApiEntry,
    parsed: &[ItemId],
    held: &BTreeSet<&str>,
) -> (String, Option<String>, String) {
    let written = entry.path.join("::");
    let names = entry.path.get(1..).unwrap_or_default();
    let Some((name, parents)) = names.split_last() else {
        // The crate root, which no item is.
        return (
            format!("{}:rust:{}", API_ITEM.scheme, id_part(&entry.container)),
            None,
            written,
        );
    };
    let skip = usize::from(entry.owner.is_some()).min(parents.len());
    let module: Vec<String> = parents[..parents.len() - skip]
        .iter()
        .map(|s| bare(s))
        .collect();
    let name = bare(name);
    let owner = entry.owner.as_ref();
    // The id is the item id's shape, each part encoded; the item is looked up by the parts as written.
    let path = ItemId::rust(&entry.container, &module, owner, &name);
    let base = std::iter::once(id_part(&entry.container))
        .chain(path.segments().iter().map(|s| rust_id_part(s)))
        .collect::<Vec<_>>()
        .join("::");
    let base = format!("{}:rust:{base}", API_ITEM.scheme);
    let krate = entry.container.replace('-', "_");
    let libs: BTreeSet<&str> = parsed
        .iter()
        .filter(|id| id.scheme() == Scheme::Rust)
        .map(ItemId::container_text)
        .filter(|container| !container.contains('/') && container.replace('-', "_") == krate)
        .collect();
    let found: Vec<String> = libs
        .into_iter()
        .map(|lib| ItemId::rust(lib, &module, owner, &name).to_string())
        .filter(|candidate| held.contains(candidate.as_str()))
        .collect();
    (base, single(found), written)
}

/// A JavaScript entry's id, its item and its path as written.
fn js(entry: &ApiEntry, parsed: &[ItemId]) -> (String, Option<String>, String) {
    // `.` joins the names, so a name holding one (a quoted member `"a.b"`) has it encoded.
    let names: Vec<String> = entry
        .path
        .iter()
        .map(|name| id_part(name).replace('.', "%2E"))
        .collect();
    let base = format!(
        "{}:js:{}::{}",
        API_ITEM.scheme,
        id_part(&entry.container),
        names.join(".")
    );
    let prefix = format!("{}/", entry.container);
    let found: Vec<String> = parsed
        .iter()
        .filter(|id| {
            id.scheme() == Scheme::Js
                && id.container_text().starts_with(&prefix)
                && id.segments() == entry.path.as_slice()
        })
        .map(ItemId::to_string)
        .collect();
    (base, single(found), entry.path.join("."))
}

/// The one candidate, or none when there are none or several.
fn single(mut found: Vec<String>) -> Option<String> {
    found.dedup();
    match found.len() {
        1 => found.pop(),
        _ => None,
    }
}

/// A path segment without its generic arguments: `Gen<T>` is `Gen`.
fn bare(segment: &str) -> String {
    segment
        .split('<')
        .next()
        .unwrap_or(segment)
        .trim()
        .to_string()
}

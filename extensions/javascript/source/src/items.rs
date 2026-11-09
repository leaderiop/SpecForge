//! A parsed file's matches as definitions, and the workspace's packages as package facts (ADR 0347).
//!
//! The matches of one defining node merge into one [`Group`] (the query matches a node once per pattern that names it:
//! a declaration and its modifiers, a function-valued declarator as a function and a const). A group is an item when
//! it is at item scope: every definition around it is a namespace, or, for a method or a field, the nearest is a class
//! or an interface (with namespaces above). Anything inside a `scope` (`declare module "x"`, `declare global`) or a
//! function is no item, nor is a member with no class or interface around it (a class expression handed to a call).
//! An item's path is the names of the definitions around it, then its own; a group with no name is the item
//! `default` when it is a default export, else no item.

use std::collections::{BTreeMap, BTreeSet};

use specforge_code_facts::{
    location, Definition, ItemId, ItemKind, Names, Package, PackageId, Visibility,
};
use specforge_extension_sdk::host::{SyntaxCapture, SyntaxMatch, SyntaxRole, SyntaxSpan};
use specforge_extension_sdk::prelude::Fact;
use specforge_javascript_workspace::Workspace;

/// One defining node: every definition match of its span, merged.
struct Group<'m> {
    span: SyntaxSpan,
    kind: &'m str,
    name: Option<&'m SyntaxCapture>,
    attrs: Vec<&'m str>,
    /// The index of the innermost group whose span holds this one.
    parent: Option<usize>,
}

/// Which role kind wins when two patterns name one node: a scope over all, a function- or class-valued declarator
/// over the plain `const` or `variable` the same declarator matches.
fn rank(kind: &str) -> u8 {
    match kind {
        "scope" => 3,
        "function" | "class" => 2,
        "const" | "variable" => 0,
        _ => 1,
    }
}

impl<'m> Group<'m> {
    fn merge(&mut self, found: &'m SyntaxMatch) {
        if rank(&found.kind) > rank(self.kind) {
            self.kind = &found.kind;
        }
        for capture in &found.captures {
            match capture.name.as_str() {
                "name" if self.name.is_none() => self.name = Some(capture),
                "attr" if !self.attrs.contains(&capture.text.as_str()) => {
                    self.attrs.push(&capture.text);
                }
                _ => {}
            }
        }
    }

    fn has_attr(&self, attr: &str) -> bool {
        self.attrs.contains(&attr)
    }

    /// Its name as path segments: a nested namespace `A.B` is two, a string member name loses its quotes, an unnamed
    /// default export is `default`; `None` for a group with no name.
    fn segments(&self) -> Option<Vec<String>> {
        let Some(name) = self.name else {
            return self
                .has_attr("default")
                .then(|| vec!["default".to_string()]);
        };
        let text = name.text.as_str();
        Some(match self.kind {
            "namespace" => text.split('.').map(|s| s.trim().to_string()).collect(),
            _ => vec![unquoted(text).to_string()],
        })
    }
}

/// A string name without its quotes (`'my-key'` is `my-key`).
fn unquoted(text: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|q| text.strip_prefix(*q)?.strip_suffix(*q))
        .unwrap_or(text)
}

/// The definition matches of a file as groups, in source order (the reader sorts matches by start, then by end
/// descending, so the matches of one node are adjacent and a holder comes before what it holds).
fn groups(matches: &[SyntaxMatch]) -> Vec<Group<'_>> {
    let mut groups: Vec<Group> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for found in matches.iter().filter(|m| m.role == SyntaxRole::Definition) {
        let (start, end) = (found.span.start.byte, found.span.end.byte);
        if let Some(last) = groups
            .last_mut()
            .filter(|g| (g.span.start.byte, g.span.end.byte) == (start, end))
        {
            last.merge(found);
            continue;
        }
        while let Some(&top) = open.last() {
            let held = groups[top].span;
            if held.start.byte <= start && end <= held.end.byte {
                break;
            }
            open.pop();
        }
        let mut group = Group {
            span: found.span,
            kind: &found.kind,
            name: None,
            attrs: Vec::new(),
            parent: open.last().copied(),
        };
        group.merge(found);
        open.push(groups.len());
        groups.push(group);
    }
    groups
}

/// The item kind of a group's role kind; `None` for a scope.
fn kind(kind: &str) -> Option<ItemKind> {
    Some(match kind {
        "function" => ItemKind::Function,
        "class" => ItemKind::Class,
        "interface" => ItemKind::Interface,
        "type" => ItemKind::Type,
        "enum" => ItemKind::Enum,
        "namespace" => ItemKind::Namespace,
        "const" => ItemKind::Const,
        "variable" => ItemKind::Variable,
        "method" => ItemKind::Method,
        "field" => ItemKind::Field,
        _ => return None,
    })
}

fn is_member(group: &Group) -> bool {
    matches!(group.kind, "method" | "field")
}

/// The groups around `at`, innermost first, when `at` is at item scope; `None` otherwise.
fn around<'g, 'm>(groups: &'g [Group<'m>], at: usize) -> Option<Vec<&'g Group<'m>>> {
    let group = &groups[at];
    let mut around = Vec::new();
    let mut parent = group.parent;
    while let Some(index) = parent {
        let holder = &groups[index];
        let allowed = if around.is_empty() && is_member(group) {
            matches!(holder.kind, "class" | "interface")
        } else {
            holder.kind == "namespace"
        };
        if !allowed {
            return None;
        }
        around.push(holder);
        parent = holder.parent;
    }
    (!is_member(group) || !around.is_empty()).then_some(around)
}

/// The item path of `at`: the names of the groups around it, outermost first, then its own.
fn path(groups: &[Group], at: usize) -> Option<Vec<String>> {
    let mut path = Vec::new();
    for holder in around(groups, at)?.iter().rev() {
        path.extend(holder.segments()?);
    }
    path.extend(groups[at].segments()?);
    Some(path)
}

/// The definitions of `file` (project-relative; `under` the package `package`'s directory), parsed as `matches`.
pub(crate) fn read(
    package: &str,
    file: &str,
    under: &str,
    matches: &[SyntaxMatch],
) -> Vec<Definition> {
    let groups = groups(matches);
    let mut definitions = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        let (Some(kind), Some(path)) = (kind(group.kind), path(&groups, at)) else {
            continue;
        };
        definitions.push(Definition {
            id: ItemId::js(package, under, &path),
            package: PackageId::npm(package),
            kind,
            visibility: Visibility::js(false),
            file: file.to_string(),
            at: location(file, group.span.start, group.span.end),
            signature: None,
            doc: None,
            attrs: Vec::new(),
            names: Names::new(),
            test_id: None,
        });
    }
    definitions
}

/// One `package` fact per named package of `workspace`, `pkg:npm/<name>`: its version, its engines, its directory and
/// the project's own packages it depends on. Two `package.json` of one name are one package, the shallowest's.
pub(crate) fn packages(workspace: &Workspace) -> Vec<Fact> {
    let mut ordered: Vec<_> = workspace.packages().iter().collect();
    ordered.sort_by_key(|p| (p.dir.split('/').filter(|s| !s.is_empty()).count(), &p.dir));
    let mut by_name = BTreeMap::new();
    for package in ordered {
        by_name.entry(package.name.as_str()).or_insert(package);
    }
    let own: BTreeSet<&str> = by_name.keys().copied().collect();
    by_name
        .values()
        .map(|package| {
            Package {
                id: PackageId::npm(&package.name),
                version: package.version.clone(),
                edition: None,
                rust_version: None,
                engines: package.engines.clone(),
                features: Vec::new(),
                targets: Vec::new(),
                path: package.dir.clone(),
                depends_on: package
                    .depends_on
                    .iter()
                    .filter(|name| own.contains(name.as_str()) && **name != package.name)
                    .map(|name| PackageId::npm(name))
                    .collect(),
                at: None,
            }
            .fact()
        })
        .collect()
}

//! A parsed file's matches as definitions, and the workspace's packages as package facts (ADR 0347).
//!
//! The matches of one defining node merge into one [`Group`] (the query matches a node once per pattern that names it:
//! a declaration and its modifiers, a function-valued declarator as a function and a const). A group is an item when
//! it is at item scope: every definition around it is a namespace, or, for a method or a field, the nearest is a class
//! or an interface (with namespaces above). Anything inside a `scope` (`declare module "x"`, `declare global`) or a
//! function is no item, nor is a member with no class or interface around it (a class expression handed to a call).
//! An item's path is the names of the definitions around it, then its own; a group with no name is the item
//! `default` when it is a default export, else no item.
//!
//! An item is `public` when an importer reaches it: a top-level one exported (`export`, `exports.x =`, an export list
//! or `module.exports = { … }` naming it), a namespace member exported from a public namespace, a member of a public
//! class or interface that is neither `#private` nor `private`. Its doc is the `/** */` comment that ends on its first
//! line or the line above, its signature the declared types, parameters and heritage the query captures (never a
//! value), its attrs the modifier tokens written on it.

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
    signature: Vec<&'m SyntaxCapture>,
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
                "signature" if !self.signature.iter().any(|s| s.span == capture.span) => {
                    self.signature.push(capture);
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
            signature: Vec::new(),
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

/// The names a file exports outside any definition (an export list, `export default <name>`, `module.exports = { … }`),
/// each with whether it is the default export.
fn exported(matches: &[SyntaxMatch]) -> BTreeMap<&str, bool> {
    let mut exported = BTreeMap::new();
    for found in matches {
        if found.role != SyntaxRole::Reference || found.kind != "export" || found.parent.is_some() {
            continue;
        }
        let Some(name) = found.captures.iter().find(|c| c.name == "name") else {
            continue;
        };
        let default = found
            .captures
            .iter()
            .any(|c| c.name == "attr" && c.text == "default");
        *exported.entry(name.text.as_str()).or_default() |= default;
    }
    exported
}

/// Whether an importer reaches the item `at` (its file's export names `exported`).
fn public(groups: &[Group], at: usize, exported: &BTreeMap<&str, bool>) -> bool {
    let group = &groups[at];
    match group.parent {
        None => {
            group.has_attr("export")
                || group.has_attr("exports")
                || group
                    .name
                    .is_some_and(|name| exported.contains_key(name.text.as_str()))
        }
        Some(holder) if is_member(group) => {
            let private = group.name.is_some_and(|name| name.text.starts_with('#'))
                || group.has_attr("private");
            !private && public(groups, holder, exported)
        }
        Some(holder) => group.has_attr("export") && public(groups, holder, exported),
    }
}

/// Each group's doc: a `/** */` comment goes to the first definition after it when that starts on the comment's last
/// line or the line below.
fn docs<'m>(matches: &'m [SyntaxMatch], groups: &[Group]) -> BTreeMap<usize, &'m str> {
    let mut docs = BTreeMap::new();
    for found in matches {
        if found.role != SyntaxRole::Reference || found.kind != "doc" {
            continue;
        }
        let Some(text) = found.captures.iter().find(|c| c.name == "doc") else {
            continue;
        };
        let end = found.span.end;
        let next = groups.partition_point(|g| g.span.start.byte < end.byte);
        if let Some(group) = groups.get(next) {
            if group.span.start.line <= end.line + 1 {
                docs.entry(next).or_insert(text.text.as_str());
            }
        }
    }
    docs
}

/// A `/** */` comment's text: the markers and each line's leading `*` dropped, its lines trimmed; `None` when empty.
fn doc_text(comment: &str) -> Option<String> {
    let body = comment.strip_prefix("/**").unwrap_or(comment);
    let body = body.strip_suffix("*/").unwrap_or(body);
    let lines: Vec<&str> = body
        .lines()
        .map(|line| {
            let line = line.trim();
            line.strip_prefix('*').map_or(line, str::trim_start)
        })
        .collect();
    let text = lines.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// A group's signature: its captured parts' texts joined by one space, whitespace collapsed.
fn signature(group: &Group) -> Option<String> {
    let text = group
        .signature
        .iter()
        .map(|part| part.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// The definitions of `file` (project-relative; `under` the package `package`'s directory), parsed as `matches`.
pub(crate) fn read(
    package: &str,
    file: &str,
    under: &str,
    matches: &[SyntaxMatch],
) -> Vec<Definition> {
    let groups = groups(matches);
    let exported = exported(matches);
    let docs = docs(matches, &groups);
    let mut definitions = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        let (Some(kind), Some(path)) = (kind(group.kind), path(&groups, at)) else {
            continue;
        };
        let mut attrs: Vec<String> = group.attrs.iter().map(|a| a.to_string()).collect();
        let named_default = group.parent.is_none()
            && group
                .name
                .is_some_and(|name| exported.get(name.text.as_str()) == Some(&true));
        if named_default && !group.has_attr("default") {
            attrs.push("default".to_string());
        }
        definitions.push(Definition {
            id: ItemId::js(package, under, &path),
            package: PackageId::npm(package),
            kind,
            visibility: Visibility::js(public(&groups, at, &exported)),
            file: file.to_string(),
            at: location(file, group.span.start, group.span.end),
            signature: signature(group),
            doc: docs.get(&at).and_then(|comment| doc_text(comment)),
            attrs,
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

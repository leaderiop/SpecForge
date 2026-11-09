//! A parsed file's matches as definitions (ADR 0347).
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

use std::collections::BTreeMap;

use specforge_code_facts::{location, Definition, ItemId, ItemKind, Names, Visibility};
use specforge_extension_sdk::host::{SyntaxCapture, SyntaxMatch, SyntaxRole, SyntaxSpan};
use specforge_javascript_workspace::Package;

/// What a definition is, as the query's `@definition.<kind>` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Function,
    Class,
    Interface,
    Type,
    Enum,
    Namespace,
    Const,
    Variable,
    Method,
    Field,
    /// A block whose contents are no item (`declare module "x"`, `declare global`).
    Scope,
}

impl Role {
    /// The role a match's kind names; `None` for a kind the pack's query never writes.
    fn of(kind: &str) -> Option<Role> {
        Some(match kind {
            "function" => Role::Function,
            "class" => Role::Class,
            "interface" => Role::Interface,
            "type" => Role::Type,
            "enum" => Role::Enum,
            "namespace" => Role::Namespace,
            "const" => Role::Const,
            "variable" => Role::Variable,
            "method" => Role::Method,
            "field" => Role::Field,
            "scope" => Role::Scope,
            _ => return None,
        })
    }

    /// Which role wins when two patterns name one node: a scope over all, a function- or class-valued declarator
    /// over the plain `const` or `variable` the same declarator matches.
    fn rank(self) -> u8 {
        match self {
            Role::Scope => 3,
            Role::Function | Role::Class => 2,
            Role::Const | Role::Variable => 0,
            _ => 1,
        }
    }

    /// The item kind; `None` for a scope.
    fn item_kind(self) -> Option<ItemKind> {
        Some(match self {
            Role::Function => ItemKind::Function,
            Role::Class => ItemKind::Class,
            Role::Interface => ItemKind::Interface,
            Role::Type => ItemKind::Type,
            Role::Enum => ItemKind::Enum,
            Role::Namespace => ItemKind::Namespace,
            Role::Const => ItemKind::Const,
            Role::Variable => ItemKind::Variable,
            Role::Method => ItemKind::Method,
            Role::Field => ItemKind::Field,
            Role::Scope => return None,
        })
    }

    fn is_member(self) -> bool {
        matches!(self, Role::Method | Role::Field)
    }
}

/// One defining node: every definition match of its span, merged.
struct Group<'m> {
    span: SyntaxSpan,
    role: Role,
    /// Its name's segments: one, or one per segment of a dotted namespace name; empty for a group with none.
    name: Vec<String>,
    /// The name was a string (`'a-b'() {}`): never a `#private` one.
    quoted: bool,
    attrs: Vec<&'m str>,
    signature: Vec<&'m SyntaxCapture>,
    /// The index of the innermost group whose span holds this one.
    parent: Option<usize>,
}

impl<'m> Group<'m> {
    fn merge(&mut self, role: Role, found: &'m SyntaxMatch) {
        if role.rank() > self.role.rank() {
            self.role = role;
        }
        if self.name.is_empty() {
            let named = |name: &'static str| found.captures.iter().filter(move |c| c.name == name);
            self.name = named("name").map(|c| c.text.clone()).collect();
            let parts: Vec<&SyntaxCapture> = named("part").collect();
            if self.name.is_empty() && !parts.is_empty() {
                self.name = vec![decoded(&parts)];
                self.quoted = true;
            }
        }
        for capture in &found.captures {
            match capture.name.as_str() {
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

    /// Its name as path segments: a dotted namespace `A.B` is two, a string member name is its value, an unnamed
    /// default export is `default`; `None` for a group with no name.
    fn segments(&self) -> Option<Vec<String>> {
        if self.name.is_empty() {
            return self
                .has_attr("default")
                .then(|| vec!["default".to_string()]);
        }
        Some(self.name.clone())
    }

    /// Its name when it is one identifier, as an export list names it.
    fn identifier(&self) -> Option<&str> {
        match &self.name[..] {
            [name] if !self.quoted => Some(name),
            _ => None,
        }
    }
}

/// A string's value from its fragments and escape sequences, as the grammar splits it: each escape decoded
/// (`\'` is `'`, `\u{41}` is `A`, a line continuation nothing).
fn decoded(parts: &[&SyntaxCapture]) -> String {
    let mut value = String::new();
    for part in parts {
        match part.text.strip_prefix('\\') {
            Some(escape) => value.push_str(&unescaped(escape)),
            None => value.push_str(&part.text),
        }
    }
    value
}

/// The value of one escape sequence (the grammar's `escape_sequence`, its backslash dropped).
fn unescaped(escape: &str) -> String {
    let code = |hex: &str, radix| {
        u32::from_str_radix(hex, radix)
            .ok()
            .and_then(char::from_u32)
    };
    let decoded = match escape.as_bytes() {
        [b'n'] => Some('\n'),
        [b't'] => Some('\t'),
        [b'r'] => Some('\r'),
        [b'b'] => Some('\u{8}'),
        [b'f'] => Some('\u{c}'),
        [b'v'] => Some('\u{b}'),
        [b'x', ..] => code(&escape[1..], 16),
        [b'u', b'{', .., b'}'] => code(&escape[2..escape.len() - 1], 16),
        [b'u', ..] => code(&escape[1..], 16),
        [b'0'..=b'7', ..] => code(escape, 8),
        // A line continuation: the backslash and the line end are no part of the value.
        [b'\r' | b'\n', ..] => return String::new(),
        _ if escape.starts_with(['\u{2028}', '\u{2029}']) => return String::new(),
        // `\'`, `\"`, `\\` and every other one-character escape are that character.
        _ if escape.chars().count() == 1 => return escape.to_string(),
        _ => None,
    };
    // shortcut: a surrogate pair written as two `\u` escapes keeps its text; decode pairs when a name needs it.
    decoded.map_or_else(|| format!("\\{escape}"), String::from)
}

/// The definition matches of a file as groups, in source order (the reader sorts matches by start, then by end
/// descending, so the matches of one node are adjacent and a holder comes before what it holds).
fn groups(matches: &[SyntaxMatch]) -> Vec<Group<'_>> {
    let mut groups: Vec<Group> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let definitions = matches
        .iter()
        .filter(|m| m.role == SyntaxRole::Definition)
        .filter_map(|m| Some((Role::of(&m.kind)?, m)));
    for (role, found) in definitions {
        let (start, end) = (found.span.start.byte, found.span.end.byte);
        if let Some(last) = groups
            .last_mut()
            .filter(|g| (g.span.start.byte, g.span.end.byte) == (start, end))
        {
            last.merge(role, found);
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
            role,
            name: Vec::new(),
            quoted: false,
            attrs: Vec::new(),
            signature: Vec::new(),
            parent: open.last().copied(),
        };
        group.merge(role, found);
        open.push(groups.len());
        groups.push(group);
    }
    groups
}

/// The groups around `at`, innermost first, when `at` is at item scope; `None` otherwise.
fn around<'g, 'm>(groups: &'g [Group<'m>], at: usize) -> Option<Vec<&'g Group<'m>>> {
    let group = &groups[at];
    let mut around = Vec::new();
    let mut parent = group.parent;
    while let Some(index) = parent {
        let holder = &groups[index];
        let allowed = if around.is_empty() && group.role.is_member() {
            matches!(holder.role, Role::Class | Role::Interface)
        } else {
            holder.role == Role::Namespace
        };
        if !allowed {
            return None;
        }
        around.push(holder);
        parent = holder.parent;
    }
    (!group.role.is_member() || !around.is_empty()).then_some(around)
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
                    .identifier()
                    .is_some_and(|name| exported.contains_key(name))
        }
        Some(holder) if group.role.is_member() => {
            let private = group.identifier().is_some_and(|name| name.starts_with('#'))
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

/// A group's signature: its captured parts' source texts joined by one space, as written (`item_facts` digests it).
fn signature(group: &Group) -> Option<String> {
    let text = group
        .signature
        .iter()
        .map(|part| part.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    (!text.trim().is_empty()).then_some(text)
}

/// The definitions of `file` (project-relative; `under` the directory of `package`, which owns it), parsed as
/// `matches`.
pub(crate) fn read(
    package: &Package,
    file: &str,
    under: &str,
    matches: &[SyntaxMatch],
) -> Vec<Definition> {
    let groups = groups(matches);
    let exported = exported(matches);
    let docs = docs(matches, &groups);
    let segment = package.segment();
    let mut definitions = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        let (Some(kind), Some(path)) = (group.role.item_kind(), path(&groups, at)) else {
            continue;
        };
        let mut attrs: Vec<String> = group.attrs.iter().map(|a| a.to_string()).collect();
        let named_default = group.parent.is_none()
            && group
                .identifier()
                .is_some_and(|name| exported.get(name) == Some(&true));
        if named_default && !group.has_attr("default") {
            attrs.push("default".to_string());
        }
        definitions.push(Definition {
            id: ItemId::js(&segment, under, &path),
            package: package.id.clone(),
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

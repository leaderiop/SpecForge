//! A parsed file's matches as definitions (ADR 0348 D6): every definition at module level, in inline modules and in
//! `impl` and `trait` blocks, never one inside a function body or any other item, with the attributes and doc
//! comments of the contiguous run of lines directly above it.

use specforge_code_facts::{location, Definition, ItemId, ItemKind, Names, Owner, Visibility};
use specforge_extension_sdk::host::{SyntaxMatch, SyntaxRole};

use crate::modules::{Crate, ModuleFile};

/// A file module a `mod <name>;` declares: the inline modules it sits in (outermost first) and the paths its `path`
/// attributes name (none: rustc's default files).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Child {
    pub name: String,
    pub inline: Vec<String>,
    pub paths: Vec<String>,
}

/// Where a definition sits inside its file: the inline modules around it (outermost first) and the `impl` or
/// `trait` that owns it.
struct Context {
    inline: Vec<String>,
    owner: Option<Owner>,
}

/// The definitions of `file` (the module `module` of `krate`, parsed as `matches`) pushed onto `out`, and the file
/// modules it declares.
pub(crate) fn read(
    krate: &Crate,
    module: &[String],
    file: &ModuleFile,
    matches: &[SyntaxMatch],
    out: &mut Vec<Definition>,
) -> Vec<Child> {
    let mut children = Vec::new();
    for (at, found) in matches.iter().enumerate() {
        if found.role != SyntaxRole::Definition {
            continue;
        }
        let (Some(kind), Some(name), Some(context)) = (
            kind(&found.kind),
            capture(found, "name"),
            context(matches, at),
        ) else {
            continue;
        };
        let kind = kind_in(kind, context.owner.is_some());
        let above = attached(matches, at);
        let mut path = module.to_vec();
        path.extend(context.inline.iter().cloned());
        out.push(Definition {
            id: ItemId::rust(&krate.segment, &path, context.owner.as_ref(), name),
            package: krate.package.clone(),
            kind,
            visibility: Visibility::rust(capture(found, "visibility")),
            file: file.file.clone(),
            at: location(&file.file, found.span.start, found.span.end),
            signature: signature(found),
            doc: doc(&above),
            attrs: attrs(&above),
            names: Names::new(),
            test_id: None,
        });
        if kind == ItemKind::Module && capture(found, "body").is_none() {
            children.push(Child {
                name: name.strip_prefix("r#").unwrap_or(name).to_string(),
                inline: context.inline,
                paths: captures(&above, "module_file", "file"),
            });
        }
    }
    children
}

/// The item kind a definition's role kind names; `None` for an `impl`, which is no item.
fn kind(role: &str) -> Option<ItemKind> {
    Some(match role {
        "module" => ItemKind::Module,
        "function" => ItemKind::Function,
        "struct" => ItemKind::Struct,
        "enum" => ItemKind::Enum,
        "union" => ItemKind::Union,
        "trait" => ItemKind::Trait,
        "type" => ItemKind::Type,
        "const" => ItemKind::Const,
        "static" => ItemKind::Static,
        "macro" => ItemKind::Macro,
        _ => return None,
    })
}

/// A function in an `impl` or a `trait` is a method.
fn kind_in(kind: ItemKind, owned: bool) -> ItemKind {
    match kind {
        ItemKind::Function if owned => ItemKind::Method,
        kind => kind,
    }
}

/// Where the definition `at` sits: `None` when an ancestor is neither an inline module nor an `impl` or `trait`
/// directly around it (a function body, a constant's block), where no path names it.
fn context(matches: &[SyntaxMatch], at: usize) -> Option<Context> {
    let mut inline = Vec::new();
    let mut owner = None;
    let mut parent = matches[at].parent;
    while let Some(index) = parent {
        let around = matches.get(index as usize)?;
        match around.kind.as_str() {
            "module" => inline.push(capture(around, "name")?.to_string()),
            "impl" if owner.is_none() && inline.is_empty() => {
                owner = Some(Owner {
                    self_ty: capture(around, "type")?.to_string(),
                    trait_ty: capture(around, "trait").map(str::to_string),
                });
            }
            "trait" if owner.is_none() && inline.is_empty() => {
                owner = Some(Owner {
                    self_ty: capture(around, "name")?.to_string(),
                    trait_ty: None,
                });
            }
            _ => return None,
        }
        parent = around.parent;
    }
    inline.reverse();
    Some(Context { inline, owner })
}

/// The attribute and doc comment matches of the contiguous run of lines directly above the definition `at`, among
/// its siblings, in source order: a blank line, another statement or a sibling definition ends the run.
fn attached(matches: &[SyntaxMatch], at: usize) -> Vec<&SyntaxMatch> {
    let definition = &matches[at];
    let mut top = definition.span.start.line;
    let mut above = Vec::new();
    for found in matches[..at].iter().rev() {
        if found.parent != definition.parent {
            continue;
        }
        if found.role == SyntaxRole::Definition || found.span.end.line + 1 < top {
            break;
        }
        top = top.min(found.span.start.line);
        above.push(found);
    }
    above.reverse();
    above
}

/// The text of `found`'s first capture `name`.
fn capture<'m>(found: &'m SyntaxMatch, name: &str) -> Option<&'m str> {
    found
        .captures
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.text.as_str())
}

/// The text of the capture `name` of each match of role kind `kind` in `above`.
fn captures(above: &[&SyntaxMatch], kind: &str, name: &str) -> Vec<String> {
    above
        .iter()
        .filter(|found| found.kind == kind)
        .filter_map(|found| capture(found, name))
        .map(str::to_string)
        .collect()
}

/// A function's signature: its generics, parameters and return type, as written.
fn signature(found: &SyntaxMatch) -> Option<String> {
    let parts: Vec<&str> = found
        .captures
        .iter()
        .filter(|c| c.name == "signature")
        .map(|c| c.text.as_str())
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// The doc comments above, one line each, the marker's following space dropped.
fn doc(above: &[&SyntaxMatch]) -> Option<String> {
    let lines: Vec<String> = captures(above, "doc", "doc")
        .iter()
        .flat_map(|text| text.lines().map(str::to_string).collect::<Vec<_>>())
        .map(|line| {
            line.strip_prefix(' ')
                .unwrap_or(&line)
                .trim_end()
                .to_string()
        })
        .collect();
    let doc = lines.join("\n").trim().to_string();
    (!doc.is_empty()).then_some(doc)
}

/// The attributes an item fact records: each `cfg(…)`, its whitespace collapsed.
fn attrs(above: &[&SyntaxMatch]) -> Vec<String> {
    above
        .iter()
        .filter(|found| found.kind == "attribute" && capture(found, "path") == Some("cfg"))
        .filter_map(|found| capture(found, "attribute"))
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

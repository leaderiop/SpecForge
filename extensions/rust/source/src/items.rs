//! A parsed file's matches as definitions (ADR 0348 D6): every definition at module level, in inline modules and in
//! `impl` and `trait` blocks, never one inside a function body or any other item, with the attributes and doc
//! comments of the contiguous run of lines directly above it. A test function knows the id nextest gives it and the
//! entity its spec-link attribute names (D7).

use specforge_code_facts::{
    location, signature_spelling, Definition, ItemId, ItemKind, Names, Owner, Visibility,
};
use specforge_extension_sdk::host::{SyntaxMatch, SyntaxRole};

use crate::modules::{Crate, ModuleFile};

/// A file module a `mod <name>;` declares: the inline modules it sits in (outermost first), the directory each of
/// them names (its `path` attribute, else its name), and the paths its own `path` attributes name (none: rustc's
/// default files). Every name is written without `r#`, as rustc finds its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Child {
    pub name: String,
    pub inline: Vec<String>,
    pub dirs: Vec<String>,
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
        let id = ItemId::rust(&krate.segment, &path, context.owner.as_ref(), name);
        let test = matches!(kind, ItemKind::Function | ItemKind::Method) && is_test(&above);
        out.push(Definition {
            test_id: if test {
                id.test_id(krate.edition)
            } else {
                None
            },
            id,
            package: krate.package.clone(),
            kind,
            visibility: Visibility::rust(capture(found, "visibility")),
            file: file.file.clone(),
            at: location(&file.file, found.span.start, found.span.end),
            signature: signature(found),
            doc: doc(&above),
            attrs: attrs(&above, test),
            names: names(&file.file, &above),
        });
        if kind == ItemKind::Module && capture(found, "body").is_none() {
            children.push(Child {
                name: unraw(name).to_string(),
                inline: context.inline,
                dirs: inline_dirs(matches, at),
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
            "module" => inline.push(unraw(capture(around, "name")?).to_string()),
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

/// The directory each inline module around the definition `at` names, outermost first: the first path its `path`
/// attributes name (relative to the directory around it), else its name.
fn inline_dirs(matches: &[SyntaxMatch], at: usize) -> Vec<String> {
    let mut dirs = Vec::new();
    let mut parent = matches[at].parent;
    while let Some(index) = parent.map(|index| index as usize) {
        let Some(around) = matches.get(index) else {
            break;
        };
        if around.kind == "module" {
            let path = captures(&attached(matches, index), "module_file", "file")
                .into_iter()
                .next();
            let name = capture(around, "name").map(|name| unraw(name).to_string());
            dirs.extend(path.or(name));
        }
        parent = around.parent;
    }
    dirs.reverse();
    dirs
}

/// A Rust identifier without its `r#`: the name rustc looks its module's file up by.
fn unraw(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
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

/// A function's signature: its generics, parameters and return type, in one spelling ([`signature_spelling`]).
fn signature(found: &SyntaxMatch) -> Option<String> {
    let parts: Vec<&str> = found
        .captures
        .iter()
        .filter(|c| c.name == "signature")
        .map(|c| c.text.as_str())
        .collect();
    // shortcut: a literal's whitespace is collapsed too (a tab char in a const generic argument, a string in an
    // unstable one); capture the literals as the JavaScript pack does when one matters.
    (!parts.is_empty()).then(|| signature_spelling(&parts.join(" "), &[]))
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

/// The attributes an item fact records: `test` for a test, `ignore` for an ignored one, each `cfg(…)` with its
/// whitespace collapsed.
fn attrs(above: &[&SyntaxMatch], test: bool) -> Vec<String> {
    let mut attrs: Vec<String> = above
        .iter()
        .filter(|found| found.kind == "attribute" && capture(found, "path") == Some("cfg"))
        .filter_map(|found| capture(found, "attribute"))
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    if test {
        attrs.push("test".to_string());
        if attribute_paths(above).any(|path| path == "ignore") {
            attrs.push("ignore".to_string());
        }
    }
    attrs
}

/// The paths of the attributes above, whitespace dropped (`tokio :: test` is `tokio::test`).
fn attribute_paths<'m>(above: &'m [&SyntaxMatch]) -> impl Iterator<Item = String> + 'm {
    above
        .iter()
        .filter(|found| found.kind == "attribute")
        .filter_map(|found| capture(found, "path"))
        .map(|path| path.split_whitespace().collect())
}

/// Whether an attribute above marks a test: its path is `test`, ends in `::test`, or is `specforge_test` or `spec`.
fn is_test(above: &[&SyntaxMatch]) -> bool {
    attribute_paths(above).any(|path| {
        path == "test" || path.ends_with("::test") || path == "specforge_test" || path == "spec"
    })
}

/// The entity each spec-link attribute above names (its first `<kind> = "<id>"`), at the id's string.
fn names(file: &str, above: &[&SyntaxMatch]) -> Names {
    let mut names = Names::new();
    for found in above.iter().filter(|found| found.kind == "entity") {
        if let Some(id) = found.captures.iter().find(|c| c.name == "id") {
            names.entry(id.text.clone()).or_default().push(location(
                file,
                id.span.start,
                id.span.end,
            ));
        }
    }
    names
}

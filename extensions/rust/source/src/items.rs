//! A parsed file's matches as definitions (ADR 0348 D6): every definition at module level, in inline modules and in
//! `impl` and `trait` blocks, never one inside a function body or any other item, with the attributes and doc
//! comments of the contiguous run of lines directly above it. A test function knows the id nextest gives it and the
//! entity its spec-link attribute names (D7).

use std::collections::{BTreeMap, BTreeSet};

use specforge_code_facts::{
    location, signature_spelling, Definition, ItemId, ItemKind, Names, Owner, Signature, Visibility,
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
    /// The `cfg(…)` attributes its file inherits: those around and on its `mod` declaration ([`read`]).
    pub cfgs: Vec<String>,
}

/// Where a definition sits inside its file: the inline modules around it (outermost first) and the `impl` or
/// `trait` that owns it.
struct Context {
    inline: Vec<String>,
    owner: Option<Owner>,
}

/// The definitions of `file` (the module `module` of `krate`, parsed as `matches`) pushed onto `out`, and the file
/// modules it declares. `inherited` holds the `cfg(…)` attributes the file's `mod` declarations put on it: a
/// definition records those, the file's inner `#![cfg(…)]`, those of every definition around it (outer, and inner in
/// its body) and its own, outermost first, as each compiles it out (ADR 0393 D5).
pub(crate) fn read(
    krate: &Crate,
    module: &[String],
    file: &ModuleFile,
    matches: &[SyntaxMatch],
    inherited: &[String],
    out: &mut Vec<Definition>,
) -> Vec<Child> {
    let inner = inner_cfgs(matches);
    let mut in_file = inherited.to_vec();
    in_file.extend(inner.get(&None).into_iter().flatten().cloned());
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
        let mut cfgs = in_file.clone();
        cfgs.extend(around_cfgs(matches, at, &inner));
        cfgs.extend(cfg_texts(above.iter().copied()));
        let mut seen = BTreeSet::new();
        cfgs.retain(|cfg| seen.insert(cfg.clone()));
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
            attrs: attrs(&above, test, cfgs.clone()),
            names: names(&file.file, &above),
        });
        if kind == ItemKind::Module && capture(found, "body").is_none() {
            children.push(Child {
                name: unraw(name).to_string(),
                inline: context.inline,
                dirs: inline_dirs(matches, at),
                paths: captures(&above, "module_file", "file"),
                cfgs,
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
fn signature(found: &SyntaxMatch) -> Option<Signature> {
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

/// The text of each `cfg(…)` attribute among `found`, outer or inner, its whitespace collapsed.
fn cfg_texts<'m>(found: impl Iterator<Item = &'m SyntaxMatch>) -> Vec<String> {
    found
        .filter(|found| {
            matches!(found.kind.as_str(), "attribute" | "inner_attribute")
                && capture(found, "path") == Some("cfg")
        })
        .filter_map(|found| capture(found, "attribute"))
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// The inner `#![cfg(…)]` attributes of a file's matches, by the definition whose body holds them (`None`: the file).
fn inner_cfgs(matches: &[SyntaxMatch]) -> BTreeMap<Option<u32>, Vec<String>> {
    let mut inner: BTreeMap<Option<u32>, Vec<String>> = BTreeMap::new();
    for found in matches.iter().filter(|m| m.kind == "inner_attribute") {
        inner
            .entry(found.parent)
            .or_default()
            .extend(cfg_texts(std::iter::once(found)));
    }
    inner
}

/// The `cfg(…)` attributes of every definition around the definition `at`, outermost first: each one's outer
/// attributes, then the inner ones in its body.
fn around_cfgs(
    matches: &[SyntaxMatch],
    at: usize,
    inner: &BTreeMap<Option<u32>, Vec<String>>,
) -> Vec<String> {
    let mut chain = Vec::new();
    let mut parent = matches[at].parent;
    while let Some(index) = parent.filter(|index| (*index as usize) < matches.len()) {
        chain.push(index);
        parent = matches[index as usize].parent;
    }
    chain
        .into_iter()
        .rev()
        .flat_map(|index| {
            let mut cfgs = cfg_texts(attached(matches, index as usize).into_iter());
            cfgs.extend(inner.get(&Some(index)).into_iter().flatten().cloned());
            cfgs
        })
        .collect()
}

/// The attributes an item fact records: each `cfg(…)` that compiles it out (`cfgs`), then `test` for a test and
/// `ignore` for an ignored one.
fn attrs(above: &[&SyntaxMatch], test: bool, cfgs: Vec<String>) -> Vec<String> {
    let mut attrs = cfgs;
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

/// The paths of the spec-link attributes, as the query's entity pattern and the macro name them.
const SPEC_LINKS: [&str; 5] = [
    "specforge_test",
    "spec",
    "specforge_test_macros::test",
    "specforge_test::test",
    "specforge::test",
];

/// Whether the attributes above register a test as `specforge_test_macros` does (ADR 0393 D6): a runner attribute
/// (`test`, or a path ending in `::test` that is no spec-link one) registers it itself; else each spec-link attribute
/// leaves registering to the next one below it, so the lowest registers it unless it names `__guard_only` (an
/// identifier among its arguments, the query's `guard_only` match on the same attribute; never a string's text).
fn is_test(above: &[&SyntaxMatch]) -> bool {
    let mut lowest = None;
    for found in above.iter().filter(|found| found.kind == "attribute") {
        let path: String = capture(found, "path")
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        let path = path.trim_start_matches("::");
        if SPEC_LINKS.contains(&path) {
            lowest = Some(found.span);
        } else if path == "test" || path.ends_with("::test") {
            return true;
        }
    }
    lowest.is_some_and(|span| {
        !above.iter().any(|found| {
            found.kind == "guard_only"
                && (found.span.start.byte, found.span.end.byte) == (span.start.byte, span.end.byte)
        })
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

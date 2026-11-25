//! The `cargo public-api` text snapshot: one entry per line, a parent before its children, an `impl` header before the
//! members it owns.

use std::collections::HashMap;

use specforge_code_facts::{ApiEntry, ApiKind, ItemKind, Owner, Scheme, Visibility};

/// What an entry line writes between its visibility and its path.
#[derive(Clone, Copy)]
enum Keyword {
    Known(ApiKind),
    /// No keyword: a field or a variant, told apart by its parent.
    Absent,
    /// A word the table lacks.
    Unknown,
}

/// An open `impl` header: the self type's path without generics, and the owner its members are defined on.
struct Header {
    self_path: String,
    owner: Owner,
}

/// The public entries of the `cargo public-api` snapshot `text`, in document order. A line is
/// `[#[attr] …] <visibility> [qualifiers] [keyword] <path><rest>`, its visibility `pub` or `pub(…)`; the keywords are
/// `mod`, `fn` (a method when its parent is the open `impl`'s self type or a type seen earlier), `struct`, `enum`,
/// `union`, `trait`, `type`, `const`, `static`, `macro`, `use` (a re-export) and `extern crate`; the qualifiers
/// `const`, `async`, `unsafe`, `auto` and `extern "…"` stay in the signature only. A line starting `impl` opens the
/// header of the members that follow and is no entry. A line with no keyword is a field when its parent is a struct
/// or union, a variant when it is an enum or a variant, `other` otherwise; a word the table lacks before a path reads
/// `other`. Any other line is an error naming its 1-based line; blank lines are skipped.
pub fn read_rust_text(text: &str) -> Result<Vec<ApiEntry>, String> {
    let mut entries = Vec::new();
    let mut kinds: HashMap<String, ApiKind> = HashMap::new();
    let mut header: Option<Header> = None;
    for (n, line) in text.lines().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty() {
            continue;
        }
        let bad = || {
            format!(
                "line {}: neither a cargo public-api entry nor an impl header",
                n + 1
            )
        };
        let body = without_attributes(line.trim()).ok_or_else(bad)?;
        if let Some(rest) = impl_header(body) {
            header = Some(parse_header(rest).ok_or_else(bad)?);
            continue;
        }
        let (visibility, keyword, path) = entry_parts(body).ok_or_else(bad)?;
        let segments = segments(path).ok_or_else(bad)?;
        let parents = &segments[..segments.len() - 1];
        let parent = bare_path(parents);
        let parent_kind = kinds.get(&parent).copied();
        let parent_owner = Owner {
            self_ty: parents.join("::"),
            trait_ty: None,
        };
        let typed_parent = matches!(
            parent_kind,
            Some(ApiKind::Item(
                ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Trait
            ))
        );
        let member_owner = header
            .as_ref()
            .filter(|open| !parents.is_empty() && open.self_path == parent)
            .map(|open| open.owner.clone())
            .or_else(|| typed_parent.then(|| parent_owner.clone()));
        let (kind, owner) = match keyword {
            Keyword::Known(ApiKind::Item(ItemKind::Function)) => match member_owner {
                Some(owner) => (ApiKind::Item(ItemKind::Method), Some(owner)),
                None => (ApiKind::Item(ItemKind::Function), None),
            },
            Keyword::Known(kind @ ApiKind::Item(ItemKind::Type | ItemKind::Const)) => {
                (kind, member_owner)
            }
            Keyword::Known(kind) => (kind, None),
            Keyword::Unknown => (ApiKind::Other, None),
            Keyword::Absent => match parent_kind {
                Some(ApiKind::Item(ItemKind::Struct | ItemKind::Union)) => {
                    (ApiKind::Item(ItemKind::Field), Some(parent_owner))
                }
                Some(ApiKind::Item(ItemKind::Enum) | ApiKind::Variant) => (ApiKind::Variant, None),
                _ => (ApiKind::Other, None),
            },
        };
        kinds.insert(bare_path(&segments), kind);
        entries.push(ApiEntry {
            scheme: Scheme::Rust,
            container: bare(&segments[0]),
            path: segments,
            owner,
            kind,
            visibility,
            signature: line.to_string(),
            tag: None,
        });
    }
    Ok(entries)
}

/// The crate the root module line of `text` names, `pub mod <crate>`, wherever it is: what a snapshot the reader
/// refuses was meant to be of (cargo public-api lists the root module first; a conflicted file still has it).
pub fn rust_text_crate(text: &str) -> Option<&str> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("pub mod ")
            .filter(|name| !name.is_empty() && !name.contains([':', ' ']))
    })
}

/// `line` after its leading `#[…]` attributes, or `None` when one does not close.
fn without_attributes(mut line: &str) -> Option<&str> {
    while line.starts_with("#[") {
        let mut depth = 0usize;
        let mut quoted = false;
        let mut escaped = false;
        let mut close = None;
        for (at, c) in line.char_indices() {
            match c {
                _ if escaped => escaped = false,
                '\\' if quoted => escaped = true,
                '"' => quoted = !quoted,
                '[' if !quoted => depth += 1,
                ']' if !quoted => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(at);
                        break;
                    }
                }
                _ => {}
            }
        }
        line = line[close? + 1..].trim_start();
    }
    Some(line)
}

/// What follows `impl` when `body` is an `impl` header (`unsafe` allowed before it).
fn impl_header(body: &str) -> Option<&str> {
    let rest = body.strip_prefix("unsafe ").unwrap_or(body).trim_start();
    let rest = rest.strip_prefix("impl")?;
    (rest.starts_with(char::is_whitespace) || rest.starts_with('<')).then_some(rest)
}

/// The header `impl[<generics>] [Trait for] SelfTy [where …]` (`rest` after `impl`): `None` with no self type.
fn parse_header(rest: &str) -> Option<Header> {
    let mut rest = rest.trim_start();
    if rest.starts_with('<') {
        rest = &rest[group_end(rest)?..];
    }
    let rest = rest.trim();
    let rest = find_top(rest, " where ")
        .map_or(rest, |at| &rest[..at])
        .trim();
    let (trait_ty, self_ty) = match find_top(rest, " for ") {
        Some(at) => (Some(rest[..at].trim()), rest[at + " for ".len()..].trim()),
        None => (None, rest),
    };
    if self_ty.is_empty() || trait_ty.is_some_and(str::is_empty) {
        return None;
    }
    Some(Header {
        self_path: bare_path(&split_top(self_ty, "::")),
        owner: Owner {
            self_ty: self_ty.to_string(),
            trait_ty: trait_ty.map(str::to_string),
        },
    })
}

/// An entry line's visibility, keyword and path, or `None` when it has none of these shapes.
fn entry_parts(body: &str) -> Option<(Visibility, Keyword, &str)> {
    let rest = body.strip_prefix("pub")?;
    let (modifier, mut s) = if rest.starts_with('(') {
        let close = rest.find(')')?;
        (&body[.."pub".len() + close + 1], &rest[close + 1..])
    } else if rest.starts_with(char::is_whitespace) {
        ("pub", rest)
    } else {
        return None;
    };
    let visibility = Visibility::rust(Some(modifier));
    let mut keyword = Keyword::Absent;
    loop {
        s = s.trim_start();
        let (word, after) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
        let after = after.trim_start();
        let next = after.split(char::is_whitespace).next().unwrap_or("");
        let kind = match word {
            "async" | "unsafe" | "auto" => {
                s = after;
                continue;
            }
            "const" if matches!(next, "fn" | "unsafe" | "async" | "extern") => {
                s = after;
                continue;
            }
            "extern" if next == "crate" => {
                s = after.strip_prefix("crate").unwrap_or(after);
                keyword = Keyword::Known(ApiKind::ExternCrate);
                break;
            }
            "extern" => {
                s = match after.strip_prefix('"') {
                    Some(abi) => &abi[abi.find('"')? + 1..],
                    None => after,
                };
                continue;
            }
            "static" => {
                s = after.strip_prefix("mut ").unwrap_or(after);
                keyword = Keyword::Known(ApiKind::Item(ItemKind::Static));
                break;
            }
            "mod" => ItemKind::Module,
            "fn" => ItemKind::Function,
            "struct" => ItemKind::Struct,
            "enum" => ItemKind::Enum,
            "union" => ItemKind::Union,
            "trait" => ItemKind::Trait,
            "type" => ItemKind::Type,
            "const" => ItemKind::Const,
            "macro" => ItemKind::Macro,
            "use" => {
                s = after;
                keyword = Keyword::Known(ApiKind::Reexport);
                break;
            }
            _ => break,
        };
        s = after;
        keyword = Keyword::Known(ApiKind::Item(kind));
        break;
    }
    let s = s.trim_start();
    let end = path_end(s);
    let path = &s[..end];
    if !path.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
        return None;
    }
    match keyword {
        // `pub frobnicate a::b`: a word the table lacks, then a path.
        Keyword::Absent if !path.contains("::") => {
            let rest = &s[end..];
            if !rest.starts_with(char::is_whitespace) {
                return None;
            }
            let rest = rest.trim_start();
            let path = &rest[..path_end(rest)];
            path.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                .then_some((visibility, Keyword::Unknown, path))
        }
        keyword => Some((visibility, keyword, path)),
    }
}

/// Where the path at the start of `s` ends: at whitespace, `(`, `{`, `=`, `;`, `,`, `!` or a lone `:` outside its
/// generic arguments.
fn path_end(s: &str) -> usize {
    let mut depth = 0usize;
    let mut prev = ' ';
    let mut chars = s.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '<' => depth += 1,
            '>' if prev != '-' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                if chars.peek().map(|&(_, next)| next) != Some(':') {
                    return at;
                }
                chars.next();
            }
            c if depth == 0 && (c.is_whitespace() || "({=;,!".contains(c)) => return at,
            _ => {}
        }
        prev = c;
    }
    s.len()
}

/// The `::` segments of `path`, generic arguments kept; `None` when one is empty.
fn segments(path: &str) -> Option<Vec<String>> {
    let segments: Vec<String> = split_top(path, "::")
        .into_iter()
        .map(str::to_string)
        .collect();
    segments.iter().all(|s| !s.is_empty()).then_some(segments)
}

/// `text` split at each `pat` outside `<…>`, `(…)` and `[…]`.
fn split_top<'t>(mut text: &'t str, pat: &str) -> Vec<&'t str> {
    let mut parts = Vec::new();
    while let Some(at) = find_top(text, pat) {
        parts.push(&text[..at]);
        text = &text[at + pat.len()..];
    }
    parts.push(text);
    parts
}

/// The first `pat` in `text` outside `<…>`, `(…)` and `[…]` (the `>` of `->` closes nothing).
fn find_top(text: &str, pat: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut prev = ' ';
    for (at, c) in text.char_indices() {
        if depth == 0 && text[at..].starts_with(pat) {
            return Some(at);
        }
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' if prev == '-' => {}
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        prev = c;
    }
    None
}

/// The end of the `<…>` group `text` starts with, past its closing `>`.
fn group_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut prev = ' ';
    for (at, c) in text.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if prev != '-' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at + 1);
                }
            }
            _ => {}
        }
        prev = c;
    }
    None
}

/// A segment without its generic arguments: `Gen<T>` is `Gen`.
fn bare(segment: &str) -> String {
    segment
        .split('<')
        .next()
        .unwrap_or(segment)
        .trim()
        .to_string()
}

/// A path's segments without generic arguments, joined by `::`: how a parent is looked up.
fn bare_path(segments: &[impl AsRef<str>]) -> String {
    segments
        .iter()
        .map(|s| bare(s.as_ref()))
        .collect::<Vec<_>>()
        .join("::")
}

//! The api-extractor `.api.json` doc model: a Package, its EntryPoints, and their members, nested.

use serde_json::Value;
use specforge_code_facts::{ApiEntry, ApiKind, ItemKind, Scheme, Visibility};

/// The public entries of the api-extractor doc model `text`, in document order: one per member below each
/// `EntryPoint` of the root `Package`, at any depth, its path the member names from the entry point down. The kinds:
/// Class class, Interface interface, Function function, Method, MethodSignature and Constructor method, Property
/// and PropertySignature field, Variable variable, TypeAlias type, Enum enum, EnumMember variant, Namespace namespace,
/// any other (IndexSignature, CallSignature, ConstructSignature included) `other`. A member the tool writes with no
/// `name` is named by its kind: Constructor `constructor`, IndexSignature `[]`, CallSignature `()`,
/// ConstructSignature `new()`. The signature is its `excerptTokens`' texts joined, the tag its `releaseTag`. An error:
/// text that is no JSON (nesting deeper than serde_json's limit included), a root that is no Package with a name, and
/// a member with no `kind`, or no `name` and none of those four kinds, named by its JSON path (`members[0].members[3]`).
pub fn read_js_model(text: &str) -> Result<Vec<ApiEntry>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("no JSON: {e}"))?;
    if root.get("kind").and_then(Value::as_str) != Some("Package") {
        return Err("the root is no api-extractor Package (kind \"Package\")".to_string());
    }
    let package = root
        .get("name")
        .and_then(Value::as_str)
        .ok_or("the Package has no name")?;
    let mut entries = Vec::new();
    for (i, entry_point) in members(&root, "")?.iter().enumerate() {
        let at = format!("members[{i}]");
        if named(entry_point, &at)?.0 == "EntryPoint" {
            walk(entry_point, &at, package, &[], &mut entries)?;
        }
    }
    Ok(entries)
}

/// One entry per member of `parent` (at the JSON path `at`) and its members, below the names `path`.
fn walk(
    parent: &Value,
    at: &str,
    package: &str,
    path: &[String],
    entries: &mut Vec<ApiEntry>,
) -> Result<(), String> {
    for (i, member) in members(parent, at)?.iter().enumerate() {
        let here = format!("{at}.members[{i}]");
        let (kind, name) = named(member, &here)?;
        let mut path = path.to_vec();
        path.push(name.to_string());
        let signature = member
            .get("excerptTokens")
            .and_then(Value::as_array)
            .map(|tokens| {
                tokens
                    .iter()
                    .filter_map(|token| token.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default();
        entries.push(ApiEntry {
            scheme: Scheme::Js,
            container: package.to_string(),
            path: path.clone(),
            owner: None,
            kind: api_kind(kind),
            visibility: Visibility::Public,
            signature,
            tag: member
                .get("releaseTag")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
        walk(member, &here, package, &path, entries)?;
    }
    Ok(())
}

/// The `members` of `parent`: none when absent, an error when not an array.
fn members<'v>(parent: &'v Value, at: &str) -> Result<&'v [Value], String> {
    match parent.get("members") {
        None => Ok(&[]),
        Some(Value::Array(members)) => Ok(members),
        Some(_) => Err(format!("{at}: members is no array")),
    }
}

/// A member's `kind` and `name`, or an error naming its JSON path `at`. The tool writes no `name` for a constructor or
/// a signature member, so one is named by its kind: `constructor`, `[]`, `()`, `new()`.
fn named<'v>(member: &'v Value, at: &str) -> Result<(&'v str, &'v str), String> {
    let none = |key: &str| format!("{at}: a member has no {key}");
    let kind = member
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| none("kind"))?;
    let by_kind = match kind {
        "Constructor" => Some("constructor"),
        "IndexSignature" => Some("[]"),
        "CallSignature" => Some("()"),
        "ConstructSignature" => Some("new()"),
        _ => None,
    };
    let name = member
        .get("name")
        .and_then(Value::as_str)
        .or(by_kind)
        .ok_or_else(|| none("name"))?;
    Ok((kind, name))
}

/// The kind of an api-extractor member kind.
fn api_kind(kind: &str) -> ApiKind {
    let item = match kind {
        "Class" => ItemKind::Class,
        "Interface" => ItemKind::Interface,
        "Function" => ItemKind::Function,
        "Method" | "MethodSignature" | "Constructor" => ItemKind::Method,
        "Property" | "PropertySignature" => ItemKind::Field,
        "Variable" => ItemKind::Variable,
        "TypeAlias" => ItemKind::Type,
        "Enum" => ItemKind::Enum,
        "Namespace" => ItemKind::Namespace,
        "EnumMember" => return ApiKind::Variant,
        _ => return ApiKind::Other,
    };
    ApiKind::Item(item)
}

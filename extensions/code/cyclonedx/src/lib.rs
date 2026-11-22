//! `specforge-code-cyclonedx`: the CycloneDX JSON reader the SBOM packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so an SBOM pack reads its BOM here; `specforge-code-facts` mints the `dependency` facts from
//! what it reads ([`specforge_code_facts::dependency_facts`]).
//!
//! - [`read`] reads a CycloneDX 1.4, 1.5 or 1.6 JSON BOM into a [`Bom`]: its root, its components and the root's
//!   direct dependencies when its dependency graph states them.

#[cfg(test)]
mod tests;

use serde_json::Value;
use specforge_code_facts::{shown_json, Component, Scope};

pub use specforge_code_facts::Bom;

/// The `specVersion`s read: the CycloneDX versions whose JSON shape of components and dependencies this reader knows.
const SPEC_VERSIONS: [&str; 3] = ["1.4", "1.5", "1.6"];

/// The BOM in the CycloneDX JSON `text`.
///
/// The root is `metadata.component`. The components are `components` and each component's nested `components`,
/// flattened depth first (nesting is no dependency tree). Per component: `bom-ref`, `purl`, `group`, `name`, `version`,
/// and `scope`, `required` when absent (the spec's default). `direct` is the `dependsOn` refs of the `dependencies`
/// entry whose `ref` is the root's `bom-ref`; `None` when the BOM has no `dependencies` array, no such entry, or a root
/// with no `bom-ref`: the BOM then states no graph for its root, and whether a component is direct is unknown.
///
/// An error, naming the JSON path or the value, and nothing is read: text that is no JSON (nesting deeper than
/// serde_json's limit included), a `bomFormat` other than `"CycloneDX"`, a `specVersion` other than `"1.4"`, `"1.5"`
/// or `"1.6"`, no `metadata.component`, a `components` or `dependencies` that is no array, a component with no
/// `name`, and a scope other than CycloneDX's three.
pub fn read(text: &str) -> Result<Bom, String> {
    let bom: Value = serde_json::from_str(text).map_err(|e| format!("no JSON: {e}"))?;
    match bom.get("bomFormat") {
        Some(Value::String(format)) if format == "CycloneDX" => {}
        Some(format) => {
            return Err(format!(
                "bomFormat: {} is not read, only \"CycloneDX\"",
                shown_json(format)
            ))
        }
        None => return Err("bomFormat: absent, only \"CycloneDX\" is read".to_string()),
    }
    let accepted = SPEC_VERSIONS.map(|v| format!("\"{v}\"")).join(", ");
    match bom.get("specVersion") {
        Some(Value::String(version)) if SPEC_VERSIONS.contains(&version.as_str()) => {}
        Some(version) => {
            return Err(format!(
                "specVersion: {} is not read, only {accepted}",
                shown_json(version)
            ))
        }
        None => return Err(format!("specVersion: absent, only {accepted} are read")),
    }
    let root = bom
        .pointer("/metadata/component")
        .ok_or("metadata.component: absent, so the BOM describes no package")?;
    let root = component(root, "metadata.component")?;
    let mut components = Vec::new();
    if let Some(list) = bom.get("components") {
        flatten(list, "components", &mut components)?;
    }
    let direct = match bom.get("dependencies") {
        None => None,
        Some(Value::Array(graph)) => root.bom_ref.as_deref().and_then(|root_ref| {
            graph
                .iter()
                .find(|entry| entry.get("ref").and_then(Value::as_str) == Some(root_ref))
                .map(|entry| {
                    array(entry.get("dependsOn"))
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
        }),
        Some(_) => return Err("dependencies: no array".to_string()),
    };
    Ok(Bom {
        root,
        components,
        direct,
    })
}

/// Appends each component of the `components` array `list` (at the JSON path `at`) to `out`, each followed by its own
/// nested `components`, depth first.
fn flatten(list: &Value, at: &str, out: &mut Vec<Component>) -> Result<(), String> {
    let Value::Array(list) = list else {
        return Err(format!("{at}: no array"));
    };
    for (i, value) in list.iter().enumerate() {
        let here = format!("{at}[{i}]");
        out.push(component(value, &here)?);
        if let Some(nested) = value.get("components") {
            flatten(nested, &format!("{here}.components"), out)?;
        }
    }
    Ok(())
}

/// The component `value` at the JSON path `at`.
fn component(value: &Value, at: &str) -> Result<Component, String> {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    let name = text("name").ok_or_else(|| format!("{at}.name: absent"))?;
    let scope = match value.get("scope") {
        None => Scope::Required,
        Some(scope) => scope.as_str().and_then(Scope::parse).ok_or_else(|| {
            format!(
                "{at}.scope: {} is not one of {}",
                shown_json(scope),
                Scope::NAMES.join(", ")
            )
        })?,
    };
    Ok(Component {
        bom_ref: text("bom-ref"),
        purl: text("purl"),
        group: text("group"),
        name,
        version: text("version"),
        scope,
    })
}

/// The elements of `value` when it is an array, else none.
fn array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

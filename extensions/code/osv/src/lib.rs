//! `specforge-code-osv`: the osv-scanner JSON reader the vulnerability packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so a vulnerability pack reads its scan here; `specforge-code-facts` mints the `vulnerability`
//! facts from what it reads ([`specforge_code_facts::vulnerability_facts`]).
//!
//! - [`read`] reads an `osv-scanner --format json` report into an [`OsvReport`]: each scanned lockfile, project-relative,
//!   and the OSV records of each package it lists.

#[cfg(test)]
mod tests;

use serde_json::Value;
use specforge_code_facts::{relative, OsvPackage, OsvRecord, OsvScore, OsvSource};

pub use specforge_code_facts::OsvReport;

/// The `source.type`s read: a scan of a lockfile or an SBOM names the project's dependencies; a scanned image or
/// commit holds no project package.
const SOURCE_TYPES: [&str; 2] = ["lockfile", "sbom"];

/// The report in the osv-scanner JSON `text`, its lockfile paths made relative to the project root `cwd`.
///
/// Per `results[]` entry: `source.path` and `source.type`, then per `packages[]` entry its `package.name`,
/// `package.version` and `package.ecosystem`, and every record of its `vulnerabilities`: `id`, `aliases`, `summary`,
/// `database_specific.severity` when it is a string, the `severity[]` entries with a `type` and a `score`, and the
/// `package.purl` of the `affected[]` entry whose name and ecosystem are the package's. `groups` is not read.
///
/// An error, naming the JSON path or the value, and nothing is read: text that is no JSON (nesting deeper than
/// serde_json's limit included), no `results` array, a result with no `source.path` or `source.type`, a `source.type`
/// other than `lockfile` or `sbom`, a `source.path` outside the project, a `packages` or `vulnerabilities` that is no
/// array, a package with no `package.name`, `package.version` or `package.ecosystem`, and a vulnerability with no `id`.
pub fn read(text: &str, cwd: &str) -> Result<OsvReport, String> {
    let report: Value = serde_json::from_str(text).map_err(|e| format!("no JSON: {e}"))?;
    let Some(Value::Array(results)) = report.get("results") else {
        return Err("results: absent or no array".to_string());
    };
    let mut sources = Vec::new();
    for (i, result) in results.iter().enumerate() {
        let at = format!("results[{i}]");
        let path = string(result, "/source/path", &format!("{at}.source.path"))?;
        let kind = string(result, "/source/type", &format!("{at}.source.type"))?;
        if !SOURCE_TYPES.contains(&kind) {
            return Err(format!(
                "{at}.source.type: '{kind}' of '{path}' is not read, only {}",
                SOURCE_TYPES.join(" and ")
            ));
        }
        let lockfile = relative(path, cwd)
            .ok_or_else(|| format!("{at}.source.path: '{path}' lies outside the project"))?;
        let mut packages = Vec::new();
        for (j, entry) in array(result, "packages", &at)?.iter().enumerate() {
            packages.push(package(entry, &format!("{at}.packages[{j}]"))?);
        }
        sources.push(OsvSource { lockfile, packages });
    }
    Ok(OsvReport { sources })
}

/// The package `entry` at the JSON path `at`, with its records.
fn package(entry: &Value, at: &str) -> Result<OsvPackage, String> {
    let name = string(entry, "/package/name", &format!("{at}.package.name"))?;
    let ecosystem = string(
        entry,
        "/package/ecosystem",
        &format!("{at}.package.ecosystem"),
    )?;
    let version = string(entry, "/package/version", &format!("{at}.package.version"))?;
    let mut records = Vec::new();
    for (k, value) in array(entry, "vulnerabilities", at)?.iter().enumerate() {
        let id = string(value, "/id", &format!("{at}.vulnerabilities[{k}].id"))?;
        let text = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
        let strings = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice)
        };
        records.push(OsvRecord {
            id: id.to_string(),
            aliases: strings("aliases")
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            summary: text("/summary").map(str::to_string),
            label: text("/database_specific/severity").map(str::to_string),
            scores: strings("severity")
                .iter()
                .filter_map(|score| {
                    Some(OsvScore {
                        kind: score.get("type")?.as_str()?.to_string(),
                        score: score.get("score")?.as_str()?.to_string(),
                    })
                })
                .collect(),
            purl: strings("affected")
                .iter()
                .find(|affected| {
                    affected.pointer("/package/name").and_then(Value::as_str) == Some(name)
                        && affected
                            .pointer("/package/ecosystem")
                            .and_then(Value::as_str)
                            == Some(ecosystem)
                })
                .and_then(|affected| affected.pointer("/package/purl"))
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    Ok(OsvPackage {
        name: name.to_string(),
        version: version.to_string(),
        ecosystem: ecosystem.to_string(),
        records,
    })
}

/// The string at `pointer` in `value`, else an error naming the JSON path `at`.
fn string<'v>(value: &'v Value, pointer: &str, at: &str) -> Result<&'v str, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}: absent or no string"))
}

/// The array at `key` of `value` (none when absent), else an error naming `<at>.<key>`.
fn array<'v>(value: &'v Value, key: &str, at: &str) -> Result<&'v [Value], String> {
    match value.get(key) {
        None => Ok(&[]),
        Some(Value::Array(list)) => Ok(list),
        Some(_) => Err(format!("{at}.{key}: no array")),
    }
}

//! `specforge-code-sarif`: the SARIF 2.1.0 reader the lint packs link (ADR 0388).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so a lint pack reads its report here; `specforge-code-facts` mints the `static_finding` facts
//! from what it reads ([`specforge_code_facts::finding_facts`]).
//!
//! - [`read`] reads a SARIF 2.1.0 log into a [`Sarif`]: one [`SarifResult`] per result of each run, in document
//!   order, and the files outside the project its results named, which it skips and counts (ADR 0389 D7).

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use serde_json::Value;
use specforge_code_facts::{file_uri_path, relative, shown, shown_json, Level, UNKNOWN};

pub use specforge_code_facts::SarifResult;

/// What a SARIF log says: its results in the project, and the files outside it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sarif {
    /// The results whose file lies in the project, or that name no file, in document order.
    pub results: Vec<SarifResult>,
    /// Each `file:` path outside the project, once, in document order, as [`shown`] writes it: its results mint
    /// nothing.
    pub outside: Vec<String>,
}

/// The results of the SARIF 2.1.0 log `text`, every run's in document order, their files relative to the project
/// root `cwd`.
///
/// Per result: the rule is `ruleId`, else `rule.id`, else the id of the driver rule at `ruleIndex` (or
/// `rule.index`), else `unknown`. The level is the result's `level`, else `none` when its `kind` is stated and is
/// not `fail`, else its rule's `defaultConfiguration.level` (the rule found by index, else by id), else `unknown`:
/// what the log states, never SARIF's own `warning` default (F5). The message is `message.text`, empty when absent.
/// The file is the first location's `artifactLocation.uri`, else the `uri` of the run artifact at its `index`: a
/// `file:` URI decoded to its path, any other (with a `uriBaseId`, or no scheme) taken as project-relative, either
/// way through [`relative`]. The lines are its `region`'s `startLine` and `endLine`. A result with no physical
/// location has no file and no line; a suppressed result is read like any other.
///
/// A result whose path lies outside the project (a `file:` URI elsewhere, or a relative one climbing out through
/// `..`) is skipped: its path is counted once in [`Sarif::outside`], never opened (ADR 0389 D7).
///
/// An error, naming the JSON path or the value, and nothing is read: text that is no JSON (nesting deeper than
/// serde_json's limit included), a `version` other than `"2.1.0"`, no `runs` array, a run with no
/// `tool.driver.name`, a run with no `results` (absent means the tool did not run, SARIF §3.14.23; an empty array is
/// a run that found nothing), an index naming no rule or artifact, and a URI of a scheme other than `file:`.
pub fn read(text: &str, cwd: &str) -> Result<Sarif, String> {
    let log: Value = serde_json::from_str(text).map_err(|e| format!("no JSON: {e}"))?;
    match log.get("version") {
        Some(Value::String(version)) if version == "2.1.0" => {}
        Some(version) => {
            return Err(format!(
                "version: {} is not read, only \"2.1.0\"",
                shown_json(version)
            ))
        }
        None => return Err("version: absent, only \"2.1.0\" is read".to_string()),
    }
    let runs = log
        .get("runs")
        .and_then(Value::as_array)
        .ok_or("runs: absent or no array")?;
    let mut read = Sarif::default();
    let mut seen = HashSet::new();
    for (r, run) in runs.iter().enumerate() {
        let at = format!("runs[{r}]");
        let Run {
            tool,
            rules,
            artifacts,
        } = Run::of(run, &at)?;
        let results = match run.get("results") {
            Some(Value::Array(results)) => results,
            Some(_) => return Err(format!("{at}.results: no array")),
            None => {
                return Err(format!(
                    "{at}.results: absent, so the tool did not run (SARIF §3.14.23)"
                ))
            }
        };
        for (i, result) in results.iter().enumerate() {
            let here = format!("{at}.results[{i}]");
            let rule = rule_of(result, rules, &here)?;
            let (file, line, end_line) = match place_of(result, artifacts, cwd, &here)? {
                Located::In(place) => place,
                Located::Outside(path) => {
                    if seen.insert(path.clone()) {
                        read.outside.push(path);
                    }
                    continue;
                }
            };
            read.results.push(SarifResult {
                tool: tool.to_string(),
                rule: rule.and_then(|(id, _)| id).unwrap_or(UNKNOWN).to_string(),
                level: level_of(result, rule.and_then(|(_, descriptor)| descriptor)),
                message: text_at(result, "/message/text")
                    .unwrap_or_default()
                    .to_string(),
                file,
                line,
                end_line,
            });
        }
    }
    Ok(read)
}

/// What a result of a run reads from its run.
struct Run<'v> {
    tool: &'v str,
    rules: &'v [Value],
    artifacts: &'v [Value],
}

impl<'v> Run<'v> {
    fn of(run: &'v Value, at: &str) -> Result<Run<'v>, String> {
        Ok(Run {
            tool: text_at(run, "/tool/driver/name")
                .ok_or_else(|| format!("{at}.tool.driver.name: absent"))?,
            rules: array_at(run, "/tool/driver/rules"),
            artifacts: array_at(run, "/artifacts"),
        })
    }
}

/// The rule a result names, its id and its driver descriptor, each when known; `None` when it names none.
type Rule<'v> = Option<(Option<&'v str>, Option<&'v Value>)>;

/// The rule of `result` (at the JSON path `at`): by `ruleIndex` or `rule.index` into `rules` first, then by `ruleId`
/// or `rule.id`. An index naming no rule is an error.
fn rule_of<'v>(result: &'v Value, rules: &'v [Value], at: &str) -> Result<Rule<'v>, String> {
    let id = text_at(result, "/ruleId").or_else(|| text_at(result, "/rule/id"));
    // A negative index (SARIF's -1) reads as none.
    let index = result
        .get("ruleIndex")
        .or_else(|| result.pointer("/rule/index"))
        .and_then(Value::as_u64);
    let descriptor = match index {
        Some(index) => Some(
            usize::try_from(index)
                .ok()
                .and_then(|index| rules.get(index))
                .ok_or_else(|| format!("{at}: rule index {index} names no rule of the driver"))?,
        ),
        None => id.and_then(|id| rules.iter().find(|rule| text_at(rule, "/id") == Some(id))),
    };
    let id = id.or_else(|| descriptor.and_then(|rule| text_at(rule, "/id")));
    Ok((id.is_some() || descriptor.is_some()).then_some((id, descriptor)))
}

/// The level `result` states, else `none` for a stated `kind` other than `fail`, else its rule's default, else
/// `unknown` (F5).
fn level_of(result: &Value, rule: Option<&Value>) -> Level {
    if let Some(level) = text_at(result, "/level").and_then(Level::parse) {
        return level;
    }
    if text_at(result, "/kind").is_some_and(|kind| kind != "fail") {
        return Level::None;
    }
    rule.and_then(|rule| text_at(rule, "/defaultConfiguration/level"))
        .and_then(Level::parse)
        .unwrap_or(Level::Unknown)
}

/// The file and the start and end lines of `result`'s first physical location, each when stated.
type Place = (Option<String>, Option<usize>, Option<usize>);

/// Where a result lies: its place in the project, or the path outside it, [`shown`].
enum Located {
    In(Place),
    Outside(String),
}

/// The place of `result` (at the JSON path `at`); a URI of another scheme than `file:`, or an artifact index naming
/// no artifact, is an error.
fn place_of(result: &Value, artifacts: &[Value], cwd: &str, at: &str) -> Result<Located, String> {
    let Some(physical) = result.pointer("/locations/0/physicalLocation") else {
        return Ok(Located::In((None, None, None)));
    };
    let artifact = physical.get("artifactLocation");
    let uri = match artifact.and_then(|a| text_at(a, "/uri")) {
        Some(uri) => Some(uri),
        None => match artifact
            .and_then(|a| a.get("index"))
            .and_then(Value::as_u64)
        {
            Some(index) => Some(
                usize::try_from(index)
                    .ok()
                    .and_then(|index| artifacts.get(index))
                    .and_then(|artifact| text_at(artifact, "/location/uri"))
                    .ok_or_else(|| {
                        format!("{at}: artifact index {index} names no artifact with a uri")
                    })?,
            ),
            None => None,
        },
    };
    let file = match uri {
        // shortcut: a `uriBaseId` is taken as the project root, whatever `originalUriBaseIds` maps it to; read the
        // map when a tool writes a base below the root.
        Some(uri) => {
            let path = file_uri_path(uri).ok_or_else(|| {
                format!(
                    "{at}: the uri `{}` is no file: uri, so it names no file of the project `{cwd}`",
                    shown(uri)
                )
            })?;
            match relative(&path, cwd) {
                Some(file) => Some(file),
                None => return Ok(Located::Outside(shown(&path))),
            }
        }
        None => None,
    };
    let line = |name: &str| {
        physical
            .pointer(&format!("/region/{name}"))
            .and_then(Value::as_u64)
            .and_then(|line| usize::try_from(line).ok())
            .filter(|&line| line >= 1)
    };
    Ok(Located::In((file, line("startLine"), line("endLine"))))
}

/// The string at the JSON pointer `pointer` of `value`, if it is one.
fn text_at<'v>(value: &'v Value, pointer: &str) -> Option<&'v str> {
    value.pointer(pointer).and_then(Value::as_str)
}

/// The array at the JSON pointer `pointer` of `value`; empty when absent or not an array.
fn array_at<'v>(value: &'v Value, pointer: &str) -> &'v [Value] {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

//! `gsd-junit`: reads a JUnit report and links its tests.
//!
//! A test is linked two ways. A `[gsd:<id>]` tag in its name links the test to that entity, by the id as written: the
//! host's `collect` resolves it (a short form to its entity's full path; an unknown or ambiguous one is W115, ADR
//! 0312).
//! A deliverable's check links by GSD's own coverage reference, `test/x.test.js#name` (`gsd-core/templates/
//! summary.md:59`): a test whose name is `name`, and whose `file` is `test/x.test.js` when the reference says so,
//! proves that check, and its result is named by the reference exactly, which is what the derive pass looks up. A test
//! that matches neither is reported unlinked, for the host's naming convention to link when it can.
//!
//! The collector declares `writes` only to be handed the graph and the project root, for the checks. It never answers
//! a plan. It has no command (GSD is language-agnostic): run the project's tests, then
//! `specforge collect --runner gsd-junit --no-run`, or name the report with `--report <file>`.
//!
//! Also ports gsd-core bin/lib modules: normalize-test-command.cjs.

#[cfg(test)]
mod tests;
mod xml;

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use xml::{Event, events};

const NAME: &str = "gsd-junit";

pub fn declare(c: &mut ContributionsBuilder) {
    c.collector(NAME, |k| {
        k.input_format("junit-xml")
            .report("junit.xml")
            .writes()
            .collect(collect);
    });
}

/// One `<testcase>`.
#[derive(Debug, PartialEq)]
struct Case {
    name: String,
    classname: String,
    file: String,
    seconds: Option<f64>,
    status: TestStatus,
}

fn cases(text: &str) -> Result<Vec<Case>, String> {
    let mut found = Vec::new();
    let mut open: Option<Case> = None;
    let mut rooted = false;
    for event in events(text)? {
        match event {
            Event::Open { name, .. } if name == "testsuite" || name == "testsuites" => {
                rooted = true
            }
            Event::Open { name, attrs, empty } if name == "testcase" => {
                let attr = |key: &str| {
                    attrs
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                };
                let case = Case {
                    name: attr("name"),
                    classname: attr("classname"),
                    file: attr("file"),
                    seconds: attr("time")
                        .trim()
                        .parse()
                        .ok()
                        .filter(|s: &f64| s.is_finite() && *s >= 0.0),
                    status: TestStatus::Passed,
                };
                if empty {
                    found.push(case);
                } else {
                    open = Some(case);
                }
            }
            Event::Open { name, .. } => {
                if let Some(case) = open.as_mut() {
                    match name.as_str() {
                        "failure" | "error" => case.status = TestStatus::Failed,
                        "skipped" if case.status == TestStatus::Passed => {
                            case.status = TestStatus::Skipped;
                        }
                        _ => {}
                    }
                }
            }
            Event::Close(name) if name == "testcase" => found.extend(open.take()),
            Event::Close(_) => {}
        }
    }
    // A report that stops inside a test, or is no JUnit at all, is no report: a failing last test must not vanish.
    if open.is_some() {
        return Err("the report stops inside a <testcase>".to_string());
    }
    if !rooted && !text.trim().is_empty() {
        return Err(
            "no <testsuite> or <testsuites> element: this is not a JUnit report".to_string(),
        );
    }
    Ok(found)
}

/// The ids named by `[gsd:<id>]` tags in `name`.
fn tags(name: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = name;
    while let Some(at) = rest.find("[gsd:") {
        rest = &rest[at + "[gsd:".len()..];
        let Some(close) = rest.find(']') else { break };
        let id = rest[..close].trim();
        if !id.is_empty() {
            found.push(id);
        }
        rest = &rest[close + 1..];
    }
    found
}

/// `path` relative to the project root `cwd`, as a report written by a tool that prints absolute paths gives it.
fn relative<'a>(path: &'a str, cwd: &str) -> &'a str {
    let path = path
        .strip_prefix(cwd)
        .filter(|rest| rest.starts_with('/'))
        .map_or(path, |rest| rest.trim_start_matches('/'));
    path.strip_prefix("./").unwrap_or(path)
}

/// The `(deliverable id, ref)` of every check of every deliverable of the graph.
fn refs(nodes: &[GraphNode]) -> Vec<(&str, &str)> {
    nodes
        .iter()
        .filter(|n| n.kind == "deliverable")
        .flat_map(|n| {
            n.fields
                .get("checks")
                .and_then(|checks| checks.as_array())
                .into_iter()
                .flatten()
                .filter_map(|check| check.get("ref")?.as_str())
                .map(|r| (n.id.as_str(), r))
        })
        .collect()
}

fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let nodes = input.graph.as_ref().map_or(&[][..], |g| &g.nodes[..]);
    let cwd = input.cwd.as_deref().unwrap_or("").trim_end_matches('/');
    let checks = refs(nodes);
    let mut linked: BTreeMap<String, Vec<TestResult>> = BTreeMap::new();
    let mut unlinked = Vec::new();
    for report in &input.reports {
        for case in cases(&report.content).map_err(|why| format!("{}: {why}", report.path))? {
            let result = |name: &str| TestResult {
                name: name.to_string(),
                status: case.status,
                verify: None,
                duration_ms: case.seconds.map(|s| s * 1000.0),
            };
            let mut hit = false;
            let mut seen: Vec<&str> = Vec::new();
            // A tag's id goes as written: the host resolves it (a short form to its full path, W115 for an unknown or
            // ambiguous one, ADR 0312). Two tags that spell one id are one result.
            for id in tags(&case.name) {
                if !seen.contains(&id) {
                    seen.push(id);
                    linked
                        .entry(id.to_string())
                        .or_default()
                        .push(result(&case.name));
                    hit = true;
                }
            }
            let file = relative(&case.file, cwd);
            for (deliverable, reference) in &checks {
                let (want_file, want_name) = reference.split_once('#').unwrap_or(("", reference));
                if want_name == case.name && (want_file.is_empty() || want_file == file) {
                    linked
                        .entry((*deliverable).to_string())
                        .or_default()
                        .push(result(reference));
                    hit = true;
                }
            }
            // A test that links to nothing is listed.
            if !hit {
                let mut path: Vec<String> = case
                    .classname
                    .split('.')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                path.push(case.name.clone());
                unlinked.push(CollectUnlinkedTest {
                    name: case.name.clone(),
                    path,
                    status: case.status,
                });
            }
        }
    }
    Ok(CollectOutput {
        entity_results: linked
            .into_iter()
            .map(|(entity_id, test_results)| CollectEntityResult {
                entity_id,
                test_results,
            })
            .collect(),
        unlinked,
        plan: None,
    })
}

//! The inputs a verdict is recorded with: one digest per named input of the rule's target, and `cited` when the
//! verdict cites (ADR 0290). Recording one and judging it later digest the same way, so a fresh verdict is exactly
//! one whose digests all agree.

use std::cell::OnceCell;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use specforge_graph::{Found, Graph, Node};
use specforge_package::Sha256;
use specforge_protocol_types::JudgedFile;
use specforge_registry::judged::{CITED, JudgedRule, Selector};
use specforge_registry::{Disk, EntitySnapshot};

use super::report::{self, Report};
use super::select::{FileRef, Select};
use super::{DigestMemo, entity_digest, file_digest};
use crate::{CompiledProject, Environment};

/// What a verdict cites (`cited`, `cited_tests`): entity references as written, file paths, test names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cited {
    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub tests: Vec<String>,
}

impl Cited {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.files.is_empty() && self.tests.is_empty()
    }

    /// What the verdict statement `verdict` cites: bare `cited` items are entities, quoted ones files.
    pub fn of(verdict: &specforge_parser::VerdictMember) -> Cited {
        use specforge_parser::{FieldValue, Written};
        let items = |key: &str| match verdict.body.get(key) {
            Some(FieldValue::List(items)) => items.clone(),
            _ => Vec::new(),
        };
        let cited = items("cited");
        let bare = |bare: bool| {
            cited
                .iter()
                .filter(|i| (i.written == Written::Bare) == bare)
                .map(|i| i.text.clone())
                .collect()
        };
        Cited {
            entities: bare(true),
            files: bare(false),
            tests: items("cited_tests").into_iter().map(|i| i.text).collect(),
        }
    }
}

/// The inputs of one rule on one entity, now.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// Each named input's digest, in the rule's order, `cited` last when the verdict cites.
    pub digests: Vec<(String, String)>,
    /// The files its `file()` inputs and `cited` read, with their digests.
    pub files: Vec<JudgedFile>,
    /// A `test(..)` input could not be digested: no report, or an unusable one.
    pub tests_unjudged: bool,
}

/// The inputs of `rule` on `entity` in `project`, files read through `disk`; `None` when the rule does not judge
/// its kind.
pub fn inputs(
    project: &CompiledProject,
    disk: &dyn Disk,
    entity: &str,
    rule: &JudgedRule,
    cited: &Cited,
) -> Option<Inputs> {
    let node = project.graph().node(entity)?;
    rule.target(node.kind.raw.as_str())?;
    let cx = Cx::new(
        project.environment(),
        project.graph(),
        project.entities(),
        disk,
        project.digests(),
    );
    Some(cx.inputs(node, rule, cited))
}

/// One evaluation's reads: the report once, each entity's digest once.
pub(super) struct Cx<'a> {
    pub env: &'a Environment,
    pub graph: &'a Graph,
    entities: &'a EntitySnapshot,
    disk: &'a dyn Disk,
    memo: &'a DigestMemo,
    report: OnceCell<Report>,
}

pub(super) fn hash(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("a JSON value serializes");
    format!("sha256:{}", Sha256::of(&bytes))
}

impl<'a> Cx<'a> {
    pub fn new(
        env: &'a Environment,
        graph: &'a Graph,
        entities: &'a EntitySnapshot,
        disk: &'a dyn Disk,
        memo: &'a DigestMemo,
    ) -> Self {
        Self {
            env,
            graph,
            entities,
            disk,
            memo,
            report: OnceCell::new(),
        }
    }

    pub fn report(&self) -> &Report {
        self.report
            .get_or_init(|| report::read(self.env.root(), self.entities, self.disk))
    }

    fn select(&self) -> Select<'_> {
        Select {
            graph: self.graph,
            fields: self.env.registries().fields(),
            forms: self.env.registries().field_forms(),
        }
    }

    fn entity(&self, id: &str) -> String {
        self.memo.entity(id, || {
            self.graph.node(id).map_or_else(
                || "absent".to_string(),
                |node| entity_digest(self.graph, self.env.registries().field_forms(), node),
            )
        })
    }

    fn entity_set(&self, selector: &str, ids: impl IntoIterator<Item = String>) -> Value {
        let members: Vec<Value> = ids
            .into_iter()
            .map(|id| json!([self.entity(&id), id]))
            .collect();
        json!({"selector": selector, "members": members})
    }

    /// A file's digest and where it is shown: `absent` and `unreadable` are digests, and a `.spec` source digests as
    /// the entities it declares, so a verdict recorded into it does not stale a reader of it.
    fn file(&self, file: &FileRef) -> JudgedFile {
        let root = self.env.root();
        let base = if file.project {
            root
        } else {
            self.env.spec_root()
        };
        let abs = normalize(&base.join(&file.path));
        let shown = abs
            .strip_prefix(root)
            .unwrap_or(&abs)
            .to_string_lossy()
            .replace('\\', "/");
        let digest = match self.disk.stamp(&abs) {
            Err(e) if e.kind() == ErrorKind::NotFound => "absent".to_string(),
            Err(_) => "unreadable".to_string(),
            Ok(_) if abs.extension().is_some_and(|e| e == "spec") => {
                let key = abs
                    .strip_prefix(self.env.spec_root())
                    .unwrap_or(&abs)
                    .to_string_lossy()
                    .replace('\\', "/");
                let mut declared: Vec<(String, String)> = self
                    .graph
                    .nodes_in_file(&key)
                    .into_iter()
                    .map(|n| (n.id.raw.to_string(), self.entity(n.id.raw.as_str())))
                    .collect();
                declared.sort();
                hash(&json!({"source": declared}))
            }
            Ok(stamp) => self.memo.file(&abs, &stamp, || match self.disk.read(&abs) {
                Ok(bytes) => file_digest(&bytes),
                Err(_) => "unreadable".to_string(),
            }),
        };
        JudgedFile {
            path: shown,
            digest,
        }
    }

    fn files(
        &self,
        selector: &str,
        files: &std::collections::BTreeSet<FileRef>,
        read: &mut Vec<JudgedFile>,
    ) -> Value {
        let entries: Vec<JudgedFile> = files.iter().map(|f| self.file(f)).collect();
        let value = json!({
            "selector": selector,
            "files": entries.iter().map(|f| json!([f.path, f.digest])).collect::<Vec<_>>(),
        });
        read.extend(entries);
        value
    }

    /// The recorded tests of `id`, as `[name, status]`; `None` without a usable report.
    fn tests_of(&self, id: &str) -> Option<Vec<Value>> {
        let Report::Read(results) = self.report() else {
            return None;
        };
        Some(
            results
                .results
                .get(id)
                .map(|t| {
                    t.tests
                        .iter()
                        .map(|t| {
                            json!([
                                t.name,
                                serde_json::to_value(t.status).unwrap_or(Value::Null)
                            ])
                        })
                        .collect()
                })
                .unwrap_or_default(),
        )
    }

    pub fn inputs(&self, node: &Node, rule: &JudgedRule, cited: &Cited) -> Inputs {
        let id = node.id.raw.as_str();
        let mut out = Inputs::default();
        let Some(target) = rule.target(node.kind.raw.as_str()) else {
            return out;
        };
        let select = self.select();
        for (name, selector) in &target.inputs {
            let shown = selector.to_string();
            let value = match selector {
                Selector::Entities(chain) => self.entity_set(&shown, select.entities(id, chain)),
                Selector::FilesOf(chain) => {
                    self.files(&shown, &select.files(id, chain), &mut out.files)
                }
                Selector::Files(paths) => {
                    let files = paths
                        .iter()
                        .map(|path| FileRef {
                            path: path.clone(),
                            project: true,
                        })
                        .collect();
                    self.files(&shown, &files, &mut out.files)
                }
                Selector::Tests(chain) => {
                    let tests: Option<Vec<Value>> = select
                        .entities(id, chain)
                        .into_iter()
                        .map(|t| self.tests_of(&t).map(|tests| json!([t, tests])))
                        .collect();
                    match tests {
                        Some(tests) => json!({"selector": shown, "tests": tests}),
                        None => {
                            out.tests_unjudged = true;
                            out.digests.push((name.clone(), NONE.to_string()));
                            continue;
                        }
                    }
                }
            };
            out.digests.push((name.clone(), hash(&value)));
        }
        if !cited.is_empty() {
            let entities: Vec<Value> = {
                let mut found: Vec<String> = cited
                    .entities
                    .iter()
                    .map(|text| {
                        match self
                            .graph
                            .scopes()
                            .chain(Some(node.id.raw), text, &|_| true)
                        {
                            Found::One(path) | Found::Mistyped(path) => path.to_string(),
                            _ => text.clone(),
                        }
                    })
                    .collect();
                found.sort();
                found.iter().map(|p| json!([self.entity(p), p])).collect()
            };
            let paths = cited
                .files
                .iter()
                .map(|path| FileRef {
                    path: path.clone(),
                    project: true,
                })
                .collect();
            let files = self.files("cited", &paths, &mut out.files);
            // shortcut: a cited test is a test name looked up across every entity's recorded tests; name a
            // test by entity when a project reuses names.
            let tests = match self.report() {
                Report::Read(results) => {
                    let mut seen: Vec<Value> = Vec::new();
                    for name in &cited.tests {
                        for (entity, recorded) in &results.results {
                            for test in recorded.tests.iter().filter(|t| &t.name == name) {
                                seen.push(json!([
                                    entity,
                                    name,
                                    serde_json::to_value(test.status).unwrap_or(Value::Null)
                                ]));
                            }
                        }
                    }
                    seen.sort_by_key(Value::to_string);
                    Value::Array(seen)
                }
                _ if cited.tests.is_empty() => json!([]),
                _ => {
                    out.tests_unjudged = true;
                    Value::String(NONE.into())
                }
            };
            let digest = if tests == Value::String(NONE.into()) {
                NONE.to_string()
            } else {
                hash(&json!({"entities": entities, "files": files, "tests": tests}))
            };
            out.digests.push((CITED.to_string(), digest));
        }
        out
    }
}

/// The digest a `test(..)` input has when no report could be read.
pub(super) const NONE: &str = "none";

/// `path` with `.` and `..` resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

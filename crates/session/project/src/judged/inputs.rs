//! The inputs a verdict is recorded with: one digest per named input of the rule's target, and `cited` when the
//! verdict cites (ADR 0290). Recording one and judging it later digest the same way, so a fresh verdict is exactly
//! one whose digests all agree.

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};
use specforge_graph::{Graph, Node, wire};
use specforge_package::Sha256;
use specforge_protocol_types::JudgedFile;
use specforge_registry::judged::{CITED, CITED_TESTS, Chain, JudgedRule, Selector};
use specforge_registry::{Disk, EntitySnapshot, FileStamp};

use super::{DigestMemo, entity_digest, file_digest};
use crate::coverage::{ReportRead, read_report};
use crate::select::{FileRef, Selection};
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
    /// The names of the inputs that could not be judged, `cited_tests` included: a `test(..)` input none of whose
    /// chains could be read (entities that are no facts with no usable report, references whose facts no collect has
    /// stored; ADR 0331 D7), and `cited_tests` with no usable report. Each digests as `none`, and freshness skips it.
    pub unjudged: Vec<String>,
    /// Each named input as it was read: what it picked and its digest.
    pub reports: Vec<InputReport>,
    /// What the verdict cites, resolved, and its digest.
    pub cited: CitedReport,
}

/// One input as it was read.
#[derive(Debug, Clone)]
pub struct InputReport {
    pub name: String,
    /// The selector, printed canonically.
    pub selector: String,
    /// `entities`, `files` or `tests`.
    pub kind: &'static str,
    /// The entities (by path) or files (from the project root) it picked.
    pub members: Vec<String>,
    pub digest: String,
}

/// What a verdict cites, resolved.
#[derive(Debug, Clone, Default)]
pub struct CitedReport {
    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub tests: Vec<String>,
    pub digest: String,
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
    report: OnceCell<ReportRead>,
    selection: Selection<'a>,
    /// The schemes the served fact stores hold facts of, read once.
    held: OnceCell<BTreeSet<String>>,
}

/// What a `test(..)` input reads now (ADR 0331 D7).
struct Tested {
    /// Every entity its chains pick, and every reference they name that is no held fact, sorted.
    members: Vec<String>,
    /// The results of the chains that could be read, sorted: empty when nothing is linked (the value `none`, judged
    /// and compared like any digest); `None` when no chain could be read, so the input is not judged.
    results: Option<Vec<Value>>,
}

/// What one chain of a `test(..)` input reads of one entity it picks or reference it names.
enum Read {
    /// A result, digested.
    Result(Value),
    /// It links no test.
    Nothing,
    /// What it needs is not recorded: no usable report for an entity that is no fact, or no store of a reference's
    /// scheme.
    Unrecorded,
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
            selection: Selection::of(graph, env.registries()),
            held: OnceCell::new(),
        }
    }

    pub fn report(&self) -> &ReportRead {
        self.report
            .get_or_init(|| read_report(self.env.root(), self.entities, self.disk))
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
        // A file outside the project is not read: an absolute path, or one that climbs out with `..`.
        let inside = abs.starts_with(root) || abs.starts_with(normalize(self.env.spec_root()));
        let shown = if inside {
            abs.strip_prefix(root)
                .unwrap_or(&abs)
                .to_string_lossy()
                .replace('\\', "/")
        } else {
            file.path.clone()
        };
        let declared_by = |abs: &Path| -> Vec<(String, String)> {
            let key = abs
                .strip_prefix(self.env.spec_root())
                .unwrap_or(abs)
                .to_string_lossy()
                .replace('\\', "/");
            let mut declared: Vec<(String, String)> = self
                .graph
                .nodes_in_file(&key)
                .into_iter()
                .map(|n| (n.id.raw.to_string(), self.entity(n.id.raw.as_str())))
                .collect();
            declared.sort();
            declared
        };
        let plain = |abs: &Path, stamp: &FileStamp| {
            self.memo.file(abs, stamp, || match self.disk.read(abs) {
                Ok(bytes) => file_digest(&bytes),
                Err(_) => "unreadable".to_string(),
            })
        };
        let digest = match self.disk.stamp(&abs) {
            _ if !inside => "unreadable".to_string(),
            Err(e) if e.kind() == ErrorKind::NotFound => "absent".to_string(),
            Err(_) => "unreadable".to_string(),
            Ok(stamp) if abs.extension().is_some_and(|e| e == "spec") => {
                // a source the project does not compile declares nothing: its bytes are what changes
                match declared_by(&abs) {
                    declared if declared.is_empty() => plain(&abs, &stamp),
                    declared => hash(&json!({"source": declared})),
                }
            }
            Ok(stamp) => plain(&abs, &stamp),
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

    /// Whether a served store holds facts of `scheme`.
    fn holds(&self, scheme: &str) -> bool {
        self.held
            .get_or_init(|| {
                crate::facts::stores(self.entities)
                    .into_iter()
                    .flat_map(|store| store.schemes)
                    .collect()
            })
            .contains(scheme)
    }

    /// What `test(chains)` reads from `this`. Each chain counts on its own: it reads the entities it picks and the
    /// references it names that are no held fact. A fact whose kind names its result field is `[id, status]`
    /// (`unknown` while unwritten); any other entity is `[id, [[name, status]…]]` from the report, nothing when it has
    /// none, unrecorded without a usable report; an unheld reference is `[id, absent]` while a store holds its scheme
    /// (the test is gone), unrecorded while none does (a fresh clone). A chain that reads only unrecorded items is not
    /// judged and adds nothing; the input is judged when one chain is, or when no chain picks anything.
    fn tests(&self, this: &str, chains: &[Chain]) -> Tested {
        let select = &self.selection;
        let mut members = BTreeSet::new();
        let (mut results, mut judged, mut unjudged) = (Vec::new(), false, false);
        for chain in chains {
            let picked = select.entities(this, chain);
            let unheld = select.unheld(this, chain);
            let reads: Vec<Read> = picked
                .iter()
                .map(|id| self.read(id))
                .chain(unheld.iter().map(|id| self.unheld(id)))
                .collect();
            members.extend(picked);
            members.extend(unheld);
            if reads.is_empty() {
                continue;
            }
            if reads.iter().all(|read| matches!(read, Read::Unrecorded)) {
                unjudged = true;
                continue;
            }
            judged = true;
            results.extend(reads.into_iter().filter_map(|read| match read {
                Read::Result(value) => Some(value),
                Read::Nothing | Read::Unrecorded => None,
            }));
        }
        results.sort_by_key(Value::to_string);
        results.dedup();
        Tested {
            members: members.into_iter().collect(),
            results: (judged || !unjudged).then_some(results),
        }
    }

    /// What a `test(..)` chain reads of `id`, an entity it picks.
    fn read(&self, id: &str) -> Read {
        let Some(node) = self.graph.node(id) else {
            return Read::Nothing;
        };
        let registries = self.env.registries();
        match registries
            .kinds()
            .collected(node.kind.raw.as_str())
            .and_then(|kind| kind.result())
        {
            Some(field) => {
                let status = wire::resolved(self.graph, registries.field_forms(), node, field)
                    .and_then(|value| value.as_value().as_str().map(str::to_string))
                    .unwrap_or_else(|| "unknown".to_string());
                Read::Result(json!([id, status]))
            }
            None => match self.tests_of(id) {
                None => Read::Unrecorded,
                Some(tests) if tests.is_empty() => Read::Nothing,
                Some(tests) => Read::Result(json!([id, tests])),
            },
        }
    }

    /// What a `test(..)` chain reads of `id`, a reference it names that is no held fact.
    fn unheld(&self, id: &str) -> Read {
        match specforge_common::scheme::of(id).is_some_and(|scheme| self.holds(scheme)) {
            true => Read::Result(json!([id, "absent"])),
            false => Read::Unrecorded,
        }
    }

    /// The recorded tests of `id`, as `[name, status]`; `None` without a usable report.
    fn tests_of(&self, id: &str) -> Option<Vec<Value>> {
        let ReportRead::Read(results) = self.report() else {
            return None;
        };
        let mut tests: Vec<Value> = results
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
            .unwrap_or_default();
        // a re-run that reorders the same tests is the same input
        tests.sort_by_key(Value::to_string);
        Some(tests)
    }

    pub fn inputs(&self, node: &Node, rule: &JudgedRule, cited: &Cited) -> Inputs {
        let id = node.id.raw.as_str();
        let mut out = Inputs::default();
        let Some(target) = rule.target(node.kind.raw.as_str()) else {
            return out;
        };
        let select = &self.selection;
        for (name, selector) in &target.inputs {
            let shown = selector.to_string();
            let read = out.files.len();
            // `None`: the input could not be judged.
            let (kind, members, digest) = match selector {
                Selector::Entities(chain) => {
                    let ids = select.entities(id, chain);
                    let members: Vec<String> = ids.iter().cloned().collect();
                    (
                        "entities",
                        members,
                        Some(hash(&self.entity_set(&shown, ids))),
                    )
                }
                Selector::FilesOf(chain) => {
                    let value = self.files(&shown, &select.files(id, chain), &mut out.files);
                    ("files", Vec::new(), Some(hash(&value)))
                }
                Selector::Files(paths) => {
                    let files = paths
                        .iter()
                        .map(|path| FileRef {
                            path: path.clone(),
                            project: true,
                        })
                        .collect();
                    let value = self.files(&shown, &files, &mut out.files);
                    ("files", Vec::new(), Some(hash(&value)))
                }
                Selector::Tests(chains) => {
                    let Tested { members, results } = self.tests(id, chains);
                    let digest = results.map(|tests| match tests.is_empty() {
                        true => NONE.to_string(),
                        false => hash(&json!({"selector": shown, "tests": tests})),
                    });
                    ("tests", members, digest)
                }
            };
            let members = if kind == "files" {
                out.files[read..].iter().map(|f| f.path.clone()).collect()
            } else {
                members
            };
            let digest = digest.unwrap_or_else(|| {
                out.unjudged.push(name.clone());
                NONE.to_string()
            });
            out.reports.push(InputReport {
                name: name.clone(),
                selector: shown,
                kind,
                members,
                digest: digest.clone(),
            });
            out.digests.push((name.clone(), digest));
        }
        {
            let entities: Vec<Value> = {
                let mut found: Vec<String> = cited
                    .entities
                    .iter()
                    .map(|text| wire::cited(self.graph, node.id.raw, text))
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
            let cited_from = out.files.len();
            let files = self.files("cited", &paths, &mut out.files);
            // The entities and files are digested whatever the report holds; the cited tests are an input of their
            // own, so a missing report hides nothing else.
            let digest = hash(&json!({"entities": entities, "files": files}));
            out.cited = CitedReport {
                entities: entities
                    .iter()
                    .filter_map(|e| e[1].as_str().map(str::to_string))
                    .collect(),
                files: out.files[cited_from..]
                    .iter()
                    .map(|f| f.path.clone())
                    .collect(),
                tests: cited.tests.clone(),
                digest: digest.clone(),
            };
            out.digests.push((CITED.to_string(), digest));
            // shortcut: a cited test is a test name looked up across every entity's recorded tests; name a
            // test by entity when a project reuses names.
            match self.report() {
                _ if cited.tests.is_empty() => {}
                ReportRead::Read(results) => {
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
                    out.digests
                        .push((CITED_TESTS.to_string(), hash(&Value::Array(seen))));
                }
                _ => {
                    out.unjudged.push(CITED_TESTS.to_string());
                    out.digests
                        .push((CITED_TESTS.to_string(), NONE.to_string()));
                }
            }
        }
        out
    }
}

/// The digest of a `test(..)` input that links nothing, and of one that could not be judged ([`Inputs::unjudged`]).
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

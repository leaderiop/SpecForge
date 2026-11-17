//! The old project, compiled in-process with its own builtins (ADR 0360 D1): every authored entity, whatever its
//! kind, each value in the one wire form every output writes (ADR 0064). Nothing is filtered by the kind an extension
//! declares (ADR 0372 D1): an entity of a kind with no fate stops the run, never passes unread. A `ref` declaration
//! is read as one too, and dropped whole by its fate. Only the project's `spec` block, which the target scaffolds
//! fresh, and the collected facts are left out ([`authored`]).

use serde_json::Value;
use specforge_common::facts::is_fact_store;
use specforge_common::{ProjectConfig, ProjectRoot, Severity, read_project_config};
use specforge_component::ComponentRuntime;
use specforge_graph::{Members, field_form};
use specforge_project::CompiledProject;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// The core grammar's structural block absorb does not read as an entity: the project's `spec` block, which the target
/// scaffolds fresh and the project module ledgers. A `ref` declaration is read, and dropped whole by its fate.
const SCAFFOLDED: &str = "spec";

/// How many of a refused source's errors, and of its dropped declarations, are printed.
const ERRORS_SHOWN: usize = 10;

/// The compiled old project.
pub struct Old {
    /// Its name (`specforge.json`'s `name`, else its directory's).
    pub name: String,
    pub project: CompiledProject,
}

/// The source's project block as the target keeps it: its name (`specforge.json`'s, else its directory's), its
/// version and its spec root (`None`: unset, the sources are read from the project's root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub version: Option<String>,
    pub spec_root: Option<String>,
}

impl Project {
    /// The project at `from` as `config` (its `specforge.json`) says.
    fn of(from: &Path, config: &ProjectConfig) -> Project {
        let name = config
            .name
            .clone()
            .or_else(|| {
                from.canonicalize()
                    .ok()?
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        Project {
            name,
            version: config.version.clone(),
            spec_root: config.spec_root.clone(),
        }
    }

    /// The project at `from`, its `specforge.json` read as a compile reads it, before anything is compiled.
    pub fn read(from: &Path) -> Project {
        Project::of(from, &read_project_config(from).config)
    }
}

/// One old authored entity.
#[derive(Debug, Clone, PartialEq)]
pub struct OldEntity {
    pub kind: String,
    pub id: String,
    pub title: Option<String>,
    /// Its file, relative to the old project's root.
    pub file: String,
    pub line: usize,
    /// Each field it writes (or takes from a file's defaults), as `field_form::write` gives it.
    pub fields: BTreeMap<String, Value>,
    /// Its `verify` statements in written order: (kind, text).
    pub obligations: Vec<(Option<String>, String)>,
    /// Its `method` members in written order, each as the graph export writes one (`name`, `params` with `name` and
    /// `type`, `returns`).
    pub methods: Vec<Value>,
    /// Each annotated field's annotations, by the field's path, each list as the graph export writes it.
    pub annotations: BTreeMap<String, Value>,
}

/// Whether a graph node of `kind` whose span names `file` is an authored entity absorb reads: any kind but the `spec`
/// block, a `ref` declaration among them, from any source but a fact store (a collected fact, which the graph tells by
/// the same test).
pub fn authored(kind: &str, file: &str) -> bool {
    kind != SCAFFOLDED && !is_fact_store(file)
}

/// The warning of a declaration whose id another kind declared first: the graph keeps the first and drops this one.
const DROPPED_DECLARATION: &str = "W060";

/// Compile the project at `from` with the builtins it names; refused when it reports an error, or a declaration its
/// graph drops (W060: an id another kind declared first), which absorb could neither convert nor ledger (ADR 0372
/// D2). Each refusal names the first [`ERRORS_SHOWN`] of its kind. No recorded item is required: no kept kind holds a
/// proof (ADR 0370 D8).
pub fn compile(from: &Path) -> Result<Old, String> {
    let root = ProjectRoot::of(from).map_err(|e| format!("{}: {e:?}", from.display()))?;
    let project =
        CompiledProject::compile(&root, Some(Arc::new(ComponentRuntime::with_user_cache())));
    let (mut errors, mut dropped) = (Vec::new(), Vec::new());
    for d in project.diagnostics() {
        let at = d
            .span
            .as_ref()
            .map(|s| format!("{}:{}: ", s.file, s.start_line))
            .unwrap_or_default();
        let line = format!("{at}{} {}", d.code, d.message);
        if d.severity == Severity::Error {
            errors.push(line);
        } else if d.code == DROPPED_DECLARATION {
            dropped.push(line);
        }
    }
    let listed = |lines: &[String]| {
        let shown: Vec<&str> = lines
            .iter()
            .take(ERRORS_SHOWN)
            .map(String::as_str)
            .collect();
        shown.join("\n  ")
    };
    let mut refusals = Vec::new();
    if !errors.is_empty() {
        refusals.push(format!(
            "the source reports {} error(s); fix them first:\n  {}",
            errors.len(),
            listed(&errors)
        ));
    }
    if !dropped.is_empty() {
        refusals.push(format!(
            "the source holds {} declaration(s) its graph drops, each an id another kind declared first, which absorb \
             would lose; rename them first:\n  {}",
            dropped.len(),
            listed(&dropped)
        ));
    }
    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }
    let name = Project::of(from, project.environment().config()).name;
    Ok(Old { name, project })
}

impl Old {
    /// Its authored entities ([`authored`]), by file, then line.
    pub fn entities(&self) -> Vec<OldEntity> {
        let mut entities: Vec<OldEntity> = self
            .project
            .graph()
            .nodes()
            .into_iter()
            .filter(|node| authored(node.kind.raw.as_str(), node.source_span.file.as_str()))
            .map(|node| {
                let members = Members::of(node);
                OldEntity {
                    kind: node.kind.raw.to_string(),
                    id: node.id.raw.to_string(),
                    title: node.title.clone(),
                    file: node.source_span.file.to_string(),
                    line: node.source_span.start_line,
                    fields: node
                        .fields
                        .entries()
                        .iter()
                        .map(|entry| {
                            (
                                entry.key.to_string(),
                                field_form::write(&entry.value).as_value().clone(),
                            )
                        })
                        .collect(),
                    obligations: node
                        .obligations
                        .iter()
                        .map(|o| {
                            let kind = (!o.kind.is_empty()).then(|| o.kind.clone());
                            (kind, o.description.clone())
                        })
                        .collect(),
                    methods: members.methods.iter().map(json).collect(),
                    annotations: members
                        .annotations
                        .iter()
                        .map(|(field, annotations)| (field.clone(), json(annotations)))
                        .collect(),
                }
            })
            .collect();
        entities.sort_by(|a, b| (&a.file, a.line, &a.id).cmp(&(&b.file, b.line, &b.id)));
        entities
    }
}

/// `value` as the graph export writes it.
fn json(value: &impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

//! The old project, compiled in-process with its own builtins (ADR 0360 D1): its entities of the absorbed kinds,
//! each value in the one wire form every output writes (ADR 0064).

use serde_json::Value;
use specforge_common::{ProjectRoot, Severity};
use specforge_component::ComponentRuntime;
use specforge_graph::field_form;
use specforge_project::CompiledProject;
use specforge_registry::PlainDisk;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use super::proof::{self, Item};
use super::table::OLD_KINDS;

/// The extension whose recorded `item` facts are the test items a proof names.
const RUST_SOURCE: &str = "@rust/source";

/// How many of a refused source's errors are printed.
const ERRORS_SHOWN: usize = 10;

/// The compiled old project.
pub struct Old {
    /// Its name (`specforge.json`'s `name`, else its directory's).
    pub name: String,
    pub project: CompiledProject,
    /// Its recorded test items, when it loads `@rust/source`.
    pub items: Vec<Item>,
}

/// One old entity of an absorbed kind.
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
}

/// Every old entity's kind by id, whatever its kind: what a reference's target is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index(pub BTreeMap<String, String>);

impl Index {
    pub fn kind(&self, id: &str) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }
}

/// Compile the project at `from` with the builtins it names; refused when it reports an error, naming the first, or
/// when it loads `@rust/source` and holds no recorded item, so a proof is never silently missing.
pub fn compile(from: &Path) -> Result<Old, String> {
    let root = ProjectRoot::of(from).map_err(|e| format!("{}: {e:?}", from.display()))?;
    let project =
        CompiledProject::compile(&root, Some(Arc::new(ComponentRuntime::with_user_cache())));
    let errors: Vec<String> = project
        .diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| {
            let at = d
                .span
                .as_ref()
                .map(|s| format!("{}:{}: ", s.file, s.start_line))
                .unwrap_or_default();
            format!("{at}{} {}", d.code, d.message)
        })
        .collect();
    if !errors.is_empty() {
        let shown: Vec<&str> = errors
            .iter()
            .take(ERRORS_SHOWN)
            .map(String::as_str)
            .collect();
        return Err(format!(
            "the source reports {} error(s); fix them first:\n  {}",
            errors.len(),
            shown.join("\n  ")
        ));
    }
    let name = project
        .environment()
        .config()
        .name
        .clone()
        .or_else(|| {
            from.canonicalize()
                .ok()?
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let items = if project
        .environment()
        .config()
        .extensions
        .iter()
        .any(|e| e == RUST_SOURCE)
    {
        let facts = specforge_project::facts::recorded_facts(&PlainDisk, root.dir(), &["item"]);
        if facts.is_empty() {
            return Err(format!(
                "it loads {RUST_SOURCE} and holds no recorded item: collect the source first \
                 (`specforge collect`), so its invariants' proofs can name their tests"
            ));
        }
        proof::items(&facts)
    } else {
        Vec::new()
    };
    Ok(Old {
        name,
        project,
        items,
    })
}

impl Old {
    /// Its entities of the absorbed kinds, by file, then line.
    pub fn entities(&self) -> Vec<OldEntity> {
        let mut entities: Vec<OldEntity> = self
            .project
            .graph()
            .nodes()
            .into_iter()
            .filter(|node| OLD_KINDS.contains(&node.kind.raw.as_str()))
            .map(|node| OldEntity {
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
            })
            .collect();
        entities.sort_by(|a, b| (&a.file, a.line, &a.id).cmp(&(&b.file, b.line, &b.id)));
        entities
    }

    /// Every entity's kind by id.
    pub fn index(&self) -> Index {
        Index(
            self.project
                .graph()
                .nodes()
                .into_iter()
                .map(|node| (node.id.raw.to_string(), node.kind.raw.to_string()))
                .collect(),
        )
    }
}

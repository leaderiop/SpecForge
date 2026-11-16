//! The old project, compiled in-process with its own builtins (ADR 0360 D1): its entities of the absorbed kinds,
//! each value in the one wire form every output writes (ADR 0064).

use serde_json::Value;
use specforge_common::{ProjectConfig, ProjectRoot, Severity, read_project_config};
use specforge_component::ComponentRuntime;
use specforge_graph::field_form;
use specforge_project::CompiledProject;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use super::table::OLD_KINDS;

/// How many of a refused source's errors are printed.
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

/// Compile the project at `from` with the builtins it names; refused when it reports an error, naming the first. No
/// recorded item is required: no kept kind holds a proof (ADR 0370 D8).
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
    let name = Project::of(from, project.environment().config()).name;
    Ok(Old { name, project })
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
}

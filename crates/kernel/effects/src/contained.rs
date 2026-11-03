//! Where a written path may lead (ADR 0335 D1): the roots a path is contained in.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// A root a path is written against: `spec`, `project`, or one a project names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RootName {
    Spec,
    Project,
    Named(String),
}

impl RootName {
    /// The root `name` names: `spec`, `project`, else a named root.
    pub fn of(name: &str) -> RootName {
        match name {
            "spec" => RootName::Spec,
            "project" => RootName::Project,
            other => RootName::Named(other.to_string()),
        }
    }
}

impl fmt::Display for RootName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RootName::Spec => f.write_str("spec"),
            RootName::Project => f.write_str("project"),
            RootName::Named(name) => f.write_str(name),
        }
    }
}

/// A project's roots: the project root, the spec root and the roots its configuration names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    project: PathBuf,
    spec: PathBuf,
    named: BTreeMap<String, PathBuf>,
}

impl Roots {
    /// The project root and the spec root, as the environment resolved them.
    pub fn new(project: impl Into<PathBuf>, spec: impl Into<PathBuf>) -> Self {
        Roots {
            project: project.into(),
            spec: spec.into(),
            named: BTreeMap::new(),
        }
    }

    /// These roots and `name` at `path`: plan 05's named roots (`specforge.json` `roots`) are built with it.
    pub fn with_named(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.named.insert(name.into(), path.into());
        self
    }

    /// Where `root` lies; `None` for a root no one named.
    pub fn get(&self, root: &RootName) -> Option<&Path> {
        match root {
            RootName::Spec => Some(&self.spec),
            RootName::Project => Some(&self.project),
            RootName::Named(name) => self.named.get(name).map(PathBuf::as_path),
        }
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn spec(&self) -> &Path {
        &self.spec
    }

    /// The spec root does not lie under the project root (ADR 0145 D4): it is an outside root, which a grant treats
    /// as a named root outside the project (plan 02).
    pub fn spec_outside(&self) -> bool {
        !self.spec.starts_with(&self.project)
    }
}

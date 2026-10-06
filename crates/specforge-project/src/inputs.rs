//! What a project session is built from, and what a changed path is to it
//! (behavior `classify_project_changes`).
//!
//! A session is built from its sources (the `.spec` files discovery finds
//! under the spec root), its environment inputs (`specforge.json`,
//! `specforge.lock` and the extension modules the environment loads) and
//! its check inputs (files the checks read but no parse does: the build
//! cache check-phase passes read, the files `file_reference` fields name).
//! Watch, the LSP and MCP all ask the session what a changed path means,
//! so "a change" has one meaning.

use std::path::{Path, PathBuf};

use specforge_common::{ExtensionEntry, ProjectConfig};
use specforge_parser::FieldValue;

use crate::Environment;
use crate::build_cache::BUILD_CACHE_FILE;
use crate::snapshot::EntitySnapshot;

/// What a changed path is to a project session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputRole {
    /// A `.spec` file discovery finds under the spec root (key relative to it).
    Source(String),
    /// specforge.json, specforge.lock, or an extension module the environment loaded.
    Environment,
    /// A file the checks read: specforge-cache.json (ADR 0009), or a file a
    /// `file_reference` field names.
    CheckInput,
    /// Anything else (an unloaded `.wasm`, an excluded `.spec`, target/…).
    Unrelated,
}

/// What a batch of changed paths means: the input to
/// [`crate::ProjectSession::apply`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Changes {
    /// Changed sources, by key relative to the spec root: sorted, deduplicated.
    pub sources: Vec<String>,
    /// An environment input changed: the environment must load again.
    pub environment: bool,
    /// A check input changed: the checks must run again.
    pub check_inputs: bool,
}

impl Changes {
    /// Nothing the session is built from changed.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty() && !self.environment && !self.check_inputs
    }

    /// The changes `roles` amount to.
    pub fn from_roles(roles: impl IntoIterator<Item = InputRole>) -> Self {
        let mut changes = Changes::default();
        for role in roles {
            match role {
                InputRole::Source(key) => changes.sources.push(key),
                InputRole::Environment => changes.environment = true,
                InputRole::CheckInput => changes.check_inputs = true,
                InputRole::Unrelated => {}
            }
        }
        changes.sources.sort();
        changes.sources.dedup();
        changes
    }
}

/// The files an environment is loaded from, whether or not they exist now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentInputs {
    /// `<root>/specforge.json`.
    pub config: PathBuf,
    /// `<root>/specforge.lock`.
    pub lock: PathBuf,
    /// The module of every extension the config enables that is not built
    /// in: `.specforge/extensions/<name>/extension.wasm` for an installed
    /// one, the resolved path of a local `.wasm` entry.
    pub modules: Vec<PathBuf>,
    /// `<root>/specforge-cache.json` when the environment has check-phase
    /// passes (they read it on every compile, ADR 0009).
    pub check_inputs: Vec<PathBuf>,
}

impl EnvironmentInputs {
    /// Every path whose change reloads the environment.
    pub fn environment_paths(&self) -> impl Iterator<Item = &Path> {
        [self.config.as_path(), self.lock.as_path()]
            .into_iter()
            .chain(self.modules.iter().map(PathBuf::as_path))
    }
}

/// Where a session's project comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Opened from the project on disk: it can be brought up to date.
    Disk,
    /// No project (an editor with no workspace folder): files are buffers
    /// keyed by absolute path.
    None,
}

/// What kind of change an [`crate::Update`] applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    /// Sources were re-parsed (and every check ran again).
    Sources,
    /// Only the checks ran again: a check input changed.
    Checks,
    /// The environment loaded again and the project was rebuilt from disk.
    Environment,
}

impl Environment {
    /// The files this environment is loaded from (behavior
    /// `classify_project_changes`). Built-in extensions are compiled into
    /// the binary and have no module on disk.
    pub fn inputs(&self) -> EnvironmentInputs {
        environment_inputs(
            &self.root,
            &self.config,
            self.registries.check_passes().next().is_some(),
        )
    }

    /// The check inputs of this environment over `graph` (and `entities`,
    /// its snapshot), as the checks read them: the build cache when
    /// check-phase passes read it, every file a `file_reference` field or a
    /// `file_exists` rule names, and the directory of each one that is
    /// missing (the E016 suggestion lists it).
    pub(crate) fn check_inputs(
        &self,
        graph: &specforge_graph::Graph,
        entities: &EntitySnapshot,
    ) -> Vec<PathBuf> {
        let references = self.named_files(graph, entities);
        let mut inputs: Vec<PathBuf> = self.inputs().check_inputs;
        inputs.extend(
            references
                .iter()
                .filter(|path| !path.exists())
                .filter_map(|path| path.parent().map(Path::to_path_buf)),
        );
        inputs.extend(references);
        inputs.sort();
        inputs.dedup();
        inputs
    }

    /// A `.spec` path's key: relative to the spec root when the file is
    /// under it (as `specforge check` names it), else the path itself.
    /// With no spec root (no project), every key is the path itself.
    pub fn source_key(&self, path: &Path) -> String {
        source_key(&self.spec_root, path)
    }

    /// The files the checks read on `graph`, resolved against the spec
    /// root: those its `file_reference` fields name
    /// ([`Self::referenced_files`]) and those the `file_exists` rules read
    /// over `entities`, its snapshot (ADR 0020), sorted and unique.
    pub fn named_files(
        &self,
        graph: &specforge_graph::Graph,
        entities: &EntitySnapshot,
    ) -> Vec<PathBuf> {
        let mut files = self.referenced_files(graph);
        files.extend(self.registries.rules.files(&entities.rule_input()));
        files.sort();
        files.dedup();
        files
    }

    /// The files the `file_reference` fields of `graph` name, resolved
    /// against the spec root: the checks report one that does not exist
    /// (E016), suggesting a similar file in its directory.
    pub fn referenced_files(&self, graph: &specforge_graph::Graph) -> Vec<PathBuf> {
        let fields: std::collections::BTreeSet<&str> = self
            .registries
            .fields
            .iter()
            .filter(|(_, _, entry)| entry.declared.file_reference)
            .map(|(_, field, _)| field)
            .collect();
        if fields.is_empty() {
            return Vec::new();
        }
        let mut files: Vec<PathBuf> = graph
            .nodes()
            .iter()
            .flat_map(|node| {
                fields
                    .iter()
                    .filter_map(|field| match node.fields.get(field) {
                        Some(FieldValue::StringList(paths)) => Some(paths.as_slice()),
                        _ => None,
                    })
                    .flatten()
                    .map(|path| self.spec_root.join(path))
                    .collect::<Vec<_>>()
            })
            .collect();
        files.sort();
        files.dedup();
        files
    }
}

/// The files an environment rooted at `root` with `config` is loaded from;
/// `check_passes`: it declares check-phase passes, which read the build
/// cache.
pub(crate) fn environment_inputs(
    root: &Path,
    config: &ProjectConfig,
    check_passes: bool,
) -> EnvironmentInputs {
    let installed = root.join(".specforge").join("extensions");
    let modules = config
        .extensions
        .iter()
        .filter_map(|entry| match ExtensionEntry::parse(entry) {
            // Relative to the root, as `specforge_component::project_runtime`
            // resolves it.
            file @ ExtensionEntry::File { .. } => file.file(root),
            ExtensionEntry::Named(name) if specforge_component::builtins::is_builtin(name) => None,
            ExtensionEntry::Named(name) => {
                Some(specforge_wasm::installed_wasm_path(&installed, name))
            }
        })
        .collect();
    EnvironmentInputs {
        config: root.join("specforge.json"),
        lock: root.join("specforge.lock"),
        modules,
        check_inputs: if check_passes {
            vec![root.join(BUILD_CACHE_FILE)]
        } else {
            Vec::new()
        },
    }
}

/// `path`'s key under `spec_root` (see [`Environment::source_key`]). The
/// editor and the workspace may spell one directory differently (a
/// symlinked temp dir), so canonical forms are compared when the plain
/// ones differ; a deleted file is compared through its directory.
pub fn source_key(spec_root: &Path, path: &Path) -> String {
    if spec_root.as_os_str().is_empty() {
        return path.to_string_lossy().into_owned();
    }
    if let Ok(relative) = path.strip_prefix(spec_root) {
        return relative.to_string_lossy().into_owned();
    }
    if let Ok(canonical_root) = std::fs::canonicalize(spec_root)
        && let Ok(relative) = canonical(path).strip_prefix(&canonical_root)
    {
        return relative.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}

/// `path`, canonical when it can be: the path itself when it exists, else
/// its directory's canonical form joined with its name (a deleted file),
/// else as given.
pub(crate) fn canonical(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            canonical(parent).join(name)
        }
        _ => path.to_path_buf(),
    }
}

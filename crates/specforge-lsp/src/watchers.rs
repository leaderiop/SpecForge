//! The files the LSP asks its client to watch (`workspace/
//! didChangeWatchedFiles`), derived from the project session: every file
//! the project is built from (`lsp_extension_reload_consistency`). What a
//! reported change means is the session's to say
//! (`ProjectSession::changes`); this only makes sure the client reports it.

use std::path::Path;

use specforge_project::{Origin, ProjectSession};
use tower_lsp::lsp_types::{
    FileSystemWatcher, GlobPattern, OneOf, RelativePattern, Url, WatchKind,
};

/// The id the watchers are registered under, so they can be replaced.
pub const REGISTRATION_ID: &str = "specforge-file-watcher";

/// What the client watches before a project is open, or when it does not
/// accept the project's own watchers: every `.spec`, config and lock file.
pub fn default_watchers() -> Vec<FileSystemWatcher> {
    ["**/*.spec", "**/specforge.json", "**/specforge.lock"]
        .into_iter()
        .map(|glob| watcher(GlobPattern::String(glob.to_string())))
        .collect()
}

/// The watchers that cover every file `session` is built from: the `.spec`
/// files under its spec root, its config and lock, the build cache its
/// check passes read, the module of each extension it loads, and each file
/// a `file_reference` field names. With `relative_patterns` (the client
/// declared `relativePatternSupport`) each is a pattern relative to its
/// directory; otherwise an absolute glob. A session not opened from disk
/// gets [`default_watchers`].
pub fn file_watchers(session: &ProjectSession, relative_patterns: bool) -> Vec<FileSystemWatcher> {
    let (Origin::Disk, Some(root), Some(spec_root)) =
        (session.origin(), session.root(), session.spec_root())
    else {
        return default_watchers();
    };
    let env = session.environment();
    let inputs = env.inputs();
    let pattern = |base: &Path, glob: &str| -> Option<FileSystemWatcher> {
        let pattern = if relative_patterns {
            GlobPattern::Relative(RelativePattern {
                base_uri: OneOf::Right(Url::from_directory_path(base).ok()?),
                pattern: glob.to_string(),
            })
        } else {
            GlobPattern::String(format!("{}/{glob}", base.to_string_lossy()))
        };
        Some(watcher(pattern))
    };
    let file = |path: &Path| -> Option<FileSystemWatcher> {
        pattern(path.parent()?, &path.file_name()?.to_string_lossy())
    };
    let mut watchers: Vec<FileSystemWatcher> = Vec::new();
    watchers.extend(pattern(spec_root, "**/*.spec"));
    watchers.extend(file(&root.join("specforge.json")));
    watchers.extend(file(&root.join("specforge.lock")));
    let references = env.named_files(session.graph(), session.entities());
    for path in inputs
        .modules
        .iter()
        .chain(&inputs.check_inputs)
        .chain(&references)
    {
        watchers.extend(file(path));
    }
    watchers
}

fn watcher(glob_pattern: GlobPattern) -> FileSystemWatcher {
    FileSystemWatcher {
        glob_pattern,
        kind: Some(WatchKind::all()),
    }
}

//! What a change the client reports asks of the project session, and what
//! applying it did. The plan is read from the state while the backend holds
//! it; it is applied to the session on the blocking pool with no lock held;
//! what it did names what to publish. A change applies as one update of the
//! session, whatever number of buffers it carries (ADR 0023 D9).

use std::path::{Path, PathBuf};

use specforge_project::{
    Changes, CheckMode, InputRole, ProjectSession, SourceChange, Update, UpdateKind,
};
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

use crate::LspState;
use crate::navigation::Compiled;
use crate::publish::Publication;
use crate::uri::uri_to_file_path;

/// A change the client reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The workspace opened at this root (`initialized`): then every open
    /// buffer.
    Open(PathBuf),
    /// These documents were opened or edited: everything the debounce
    /// coalesced. Each one still open has its buffer as the truth for its
    /// file.
    Edited(Vec<Url>),
    /// This document was closed: its buffer is no longer the truth for its
    /// file.
    Closed(Url),
    /// The client's file watchers reported these events.
    Watched(Vec<FileEvent>),
    /// The client watches what the session is built from now: apply what
    /// changed on disk while it did not (`ProjectSession::stale`), except an
    /// open document's file that still exists (its buffer is the truth).
    CatchUp,
}

/// What a closed document's file becomes to the project.
#[derive(Debug)]
enum Closed {
    /// A project source: read from disk again.
    Reread(String),
    /// Any other file (outside the spec root, excluded, or any file of a
    /// session with no project): it leaves the project.
    Drop(String),
}

/// What a change will apply.
pub struct Plan {
    /// The project to open (`Change::Open`).
    root: Option<PathBuf>,
    /// What the watched paths amount to, applied first.
    disk: Option<Changes>,
    /// The absolute path of the document that was closed.
    closed: Option<PathBuf>,
    /// The open buffers to apply, by absolute path: keys are the session's
    /// to give once the environment it applies them to is loaded.
    buffers: Vec<(PathBuf, String)>,
    /// The editor is typing: the checks are skipped while a buffer does not
    /// parse.
    typing: bool,
    /// The document an anchorless diagnostic goes to.
    edited: Option<Url>,
}

impl Plan {
    /// What `change` asks of the session `state` holds, read from the state
    /// as it is now:
    /// - `Open`: the root, then every open buffer, with every check;
    /// - `Edited`: the buffers of the documents still open, the checks
    ///   skipped while any of them does not parse (the typing fast path);
    /// - `Closed`: a project source is read from disk again; any other file
    ///   (and every file of a session with no project) leaves the project;
    /// - `Watched`: the paths that are not open documents, and the
    ///   deletions of those that are, as the session classifies them; after
    ///   an environment reload, every open buffer again;
    /// - `CatchUp`: what the session finds changed on disk since it last
    ///   read it, the same way, except an open document whose file still
    ///   exists.
    ///
    /// `None` when it asks nothing: no session is held, every edited
    /// document was closed since, the closed document was opened again, no
    /// watched path is an input of the project.
    pub fn of(change: Change, state: &LspState) -> Option<Plan> {
        let session = state.session()?;
        let nothing = Plan {
            root: None,
            disk: None,
            closed: None,
            buffers: Vec::new(),
            typing: false,
            edited: None,
        };
        match change {
            Change::Open(root) => Some(Plan {
                root: Some(root),
                buffers: open_buffers(state),
                ..nothing
            }),
            Change::Edited(uris) => {
                let open: Vec<&Url> = uris
                    .iter()
                    .filter(|uri| state.is_open(uri.as_str()))
                    .collect();
                let edited = open.last().map(|uri| (*uri).clone())?;
                Some(Plan {
                    buffers: open
                        .iter()
                        .filter_map(|uri| buffer_of(state, uri.as_str()))
                        .collect(),
                    typing: true,
                    edited: Some(edited),
                    ..nothing
                })
            }
            Change::Closed(uri) => {
                if state.is_open(uri.as_str()) {
                    return None;
                }
                Some(Plan {
                    closed: Some(PathBuf::from(uri_to_file_path(&uri))),
                    ..nothing
                })
            }
            Change::Watched(events) => {
                // An open document's buffer is the truth for its file, so
                // of its changes on disk only its deletion counts. What the
                // others are (a source, an environment or check input,
                // nothing) is the session's to say (classify_project_changes).
                let paths: Vec<PathBuf> = events
                    .iter()
                    .filter(|event| {
                        event.typ == FileChangeType::DELETED || !state.is_open(event.uri.as_str())
                    })
                    .map(|event| PathBuf::from(uri_to_file_path(&event.uri)))
                    .collect();
                let changes = session.inputs().changes(paths.iter().map(PathBuf::as_path));
                Plan::on_disk(changes, state)
            }
            Change::CatchUp => {
                // What the session finds changed on disk since it last read
                // it. An open document's buffer is the truth for its file:
                // of its changes only its deletion counts.
                let compiled = Compiled::new(state);
                let mut changes = session.stale();
                changes.sources.retain(|key| {
                    !state.is_open(compiled.uri(key).as_str()) || !state.file_path(key).exists()
                });
                Plan::on_disk(changes, state)
            }
        }
    }

    /// The plan for changes found on disk, applied first; `None` when there
    /// are none.
    fn on_disk(changes: Changes, state: &LspState) -> Option<Plan> {
        if changes.is_empty() {
            return None;
        }
        // The reload read every file from disk again: the open buffers are
        // still the truth for theirs.
        let buffers = if changes.environment {
            open_buffers(state)
        } else {
            Vec::new()
        };
        Some(Plan {
            root: None,
            disk: Some(changes),
            closed: None,
            buffers,
            typing: false,
            edited: None,
        })
    }

    /// The root a project opens at (`Change::Open`). The caller opens it
    /// (`ProjectSession::begin_open`, showing its environment to readers,
    /// then `OpeningProject::finish`) and passes the opened session to
    /// [`Plan::apply`].
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Apply the plan to `session`. Blocking: it reads files and runs the
    /// checks. The watched changes first (`ProjectSession::apply`), then the
    /// closed file (nothing when its disk text is the compiled text), then
    /// every buffer as one update (`SourceChange::Buffers`).
    pub fn apply(self, session: &mut ProjectSession) -> Applied {
        let opened = self.root.is_some();
        let mut applied = Applied {
            environment: opened,
            changed: opened,
            inputs_changed: opened,
            divergences: Vec::new(),
            edited: self.edited,
            touched: Vec::new(),
            closed: false,
        };
        if let Some(changes) = &self.disk {
            if let Some(update) = session.apply(changes) {
                applied.environment |= update.kind == UpdateKind::Environment;
                applied.record(update);
                applied.changed = true;
            }
            applied.touched.extend(changes.sources.iter().cloned());
        }
        if let Some(path) = &self.closed {
            applied.close(session, path);
        }
        if !self.buffers.is_empty() {
            // Keys are the session's now: a reload may have moved the spec
            // root.
            let buffers: Vec<(String, String)> = self
                .buffers
                .iter()
                .map(|(path, text)| (session.source_key(path), text.clone()))
                .collect();
            let keys: Vec<&str> = buffers.iter().map(|(key, _)| key.as_str()).collect();
            let mode = if self.typing {
                // The syntax-only fast path (C4-07): no checks while an
                // edited file does not parse.
                CheckMode::SyntaxOnlyIfParseErrorsIn(&keys)
            } else {
                CheckMode::Full
            };
            let update = session.update_with(SourceChange::Buffers(&buffers), mode);
            applied.record(update);
            applied
                .touched
                .extend(buffers.iter().map(|(key, _)| key.clone()));
            applied.changed = true;
        }
        applied
    }
}

/// The open document `uri` as a buffer to apply.
fn buffer_of(state: &LspState, uri: &str) -> Option<(PathBuf, String)> {
    let document = state.document(uri)?;
    let url = Url::parse(uri).ok()?;
    Some((
        PathBuf::from(uri_to_file_path(&url)),
        document.text().to_string(),
    ))
}

/// Every open document as a buffer to apply, in URI order.
fn open_buffers(state: &LspState) -> Vec<(PathBuf, String)> {
    state
        .open_uris()
        .into_iter()
        .filter_map(|uri| buffer_of(state, uri))
        .collect()
}

/// What applying a plan did.
#[derive(Debug, Clone, Default)]
pub struct Applied {
    /// The environment was loaded: the project opened, or it reloaded.
    pub environment: bool,
    /// The session changed.
    pub changed: bool,
    /// The session's inputs changed (`Update::inputs_changed`): what the
    /// client is asked to watch must follow them.
    pub inputs_changed: bool,
    /// Where an incremental rebuild differed from a cold build, when the
    /// session verifies its updates (debug builds).
    pub divergences: Vec<String>,
    edited: Option<Url>,
    touched: Vec<String>,
    /// A document was closed: the editor showed its buffer's diagnostics for this file, so the
    /// file is published as the project reports it now, whether or not the session changed.
    closed: bool,
}

impl Applied {
    /// What the client is asked to watch must follow (ADR 0035): the
    /// environment was loaded, or the session's inputs changed.
    pub fn rewatch(&self) -> bool {
        self.environment || self.inputs_changed
    }

    /// What an update says about the session's inputs and its own check.
    fn record(&mut self, update: Update) {
        self.inputs_changed |= update.inputs_changed;
        self.divergences
            .extend(update.divergence().map(str::to_string));
        self.touched.extend(update.rebuilt_files);
    }

    /// A closed document's file stops being the editor's: a project source
    /// is read from disk again (nothing to do when the disk text is what
    /// the project holds), any other file leaves the project. Either way
    /// the file is published as the project reports it now.
    fn close(&mut self, session: &mut ProjectSession, path: &Path) {
        let key = session.source_key(path);
        self.closed = true;
        self.touched.push(key.clone());
        let source = session.root().is_some()
            && matches!(session.inputs().classify(path), InputRole::Source(_));
        let closed = if source {
            Closed::Reread(key)
        } else {
            Closed::Drop(key)
        };
        match closed {
            Closed::Reread(key) => {
                let disk = std::fs::read_to_string(session.environment().spec_root.join(&key)).ok();
                if disk.as_deref() != session.source_text(&key).as_deref() {
                    let update = session.update(SourceChange::Disk(std::slice::from_ref(&key)));
                    self.record(update);
                    self.changed = true;
                }
            }
            Closed::Drop(key) => {
                let update = session.update(SourceChange::Buffer {
                    path: &key,
                    text: None,
                });
                self.record(update);
                self.changed = true;
            }
        }
    }

    /// What to publish now ([`Publication::of`]): the last edited document
    /// is where a diagnostic about no entity goes, and every touched file
    /// without diagnostics gets an empty list. `None` when the session did
    /// not change and no document was closed.
    pub fn publication(&self, state: &LspState) -> Option<Publication> {
        if !self.changed && !self.closed {
            return None;
        }
        let compiled = Compiled::new(state);
        let touched: Vec<Url> = self.touched.iter().map(|key| compiled.uri(key)).collect();
        Some(Publication::of(state, self.edited.as_ref(), &touched))
    }
}

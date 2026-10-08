//! A project on disk as the LSP serves it, without a client: its files in a
//! temporary directory, its extensions declared with the SDK and run by the
//! in-process runtime (ADR 0025), its session opened and its documents
//! changed through `specforge_lsp::changes`, every request answered by
//! `specforge_lsp::answers`.

use std::path::Path;
use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_lsp::LspState;
use specforge_lsp::answers;
use specforge_lsp::changes::{Applied, Change, Plan};
use specforge_lsp::publish::Publication;
use specforge_project::{ProjectSession, SharedRuntime};
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;
use tower_lsp::lsp_types::{HoverContents, Position, Url};

type Declare = Arc<dyn Fn(&mut ContributionsBuilder) + Send + Sync>;

/// What applying a change did, and what it published.
pub type Applying = (Applied, Option<Publication>);

pub struct Served {
    dir: TempDir,
    state: LspState,
    extensions: Vec<(String, Declare)>,
}

impl Served {
    /// A project holding `files` (relative path, text); its
    /// `specforge.json` enables the extensions added next.
    pub fn new(files: &[(&str, &str)]) -> Served {
        let dir = TempDir::new().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Served {
            dir,
            state: LspState::new(),
            extensions: Vec::new(),
        }
    }

    /// Serve extension `name`, declared by `declare` (the runtime builds it
    /// per call).
    pub fn extension(
        mut self,
        name: &str,
        declare: impl Fn(&mut ContributionsBuilder) + Send + Sync + 'static,
    ) -> Served {
        self.extensions.push((name.to_string(), Arc::new(declare)));
        self
    }

    /// The runtime serving every declared extension.
    fn runtime(&self) -> SharedRuntime {
        let mut runtime = InProcessRuntime::new();
        for (name, declare) in &self.extensions {
            let (name, declare) = (name.clone(), Arc::clone(declare));
            runtime = runtime.with(move || {
                let mut c = ContributionsBuilder::new(ExtensionMeta::new(&name, "1.0.0"));
                declare(&mut c);
                c
            });
        }
        Arc::new(runtime)
    }

    /// Open the project as `initialized` does (`Change::Open`, the session
    /// opened with the in-process runtime), then `files` as documents with
    /// their disk text (`Change::Edited`).
    pub fn open(mut self, files: &[&str]) -> Served {
        let names: Vec<&str> = self.extensions.iter().map(|(n, _)| n.as_str()).collect();
        let config = serde_json::json!({"name": "t", "version": "0.1.0", "extensions": names});
        std::fs::write(self.dir.path().join("specforge.json"), config.to_string()).unwrap();
        self.apply(Change::Open(self.dir.path().to_path_buf()))
            .expect("the project opens");
        let uris: Vec<Url> = files
            .iter()
            .map(|file| {
                let uri = self.uri(file);
                let text = std::fs::read_to_string(self.dir.path().join(file)).unwrap();
                self.state.open_document(uri.as_str(), &text);
                uri
            })
            .collect();
        if !uris.is_empty() {
            self.apply(Change::Edited(uris));
        }
        self
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn uri(&self, file: &str) -> Url {
        Url::from_file_path(self.dir.path().join(file)).unwrap()
    }

    pub fn state(&self) -> &LspState {
        &self.state
    }

    /// The buffer of `file` becomes `text`; nothing is compiled (the stale
    /// window).
    pub fn type_text(&mut self, file: &str, text: &str) {
        let uri = self.uri(file);
        self.state.apply_change(uri.as_str(), None, text);
    }

    /// Apply `change` as the backend does: `Plan::of`, the session taken
    /// out, `Plan::apply`, put back, the publication recorded. `None` when
    /// the plan asks nothing.
    pub fn apply(&mut self, change: Change) -> Option<Applying> {
        let runtime = self.runtime();
        apply_change(&mut self.state, change, Some(runtime))
    }

    /// `type_text` then `apply(Change::Edited([file]))`.
    pub fn edit(&mut self, file: &str, text: &str) -> Option<Applying> {
        self.type_text(file, text);
        let uri = self.uri(file);
        self.apply(Change::Edited(vec![uri]))
    }

    /// Close `file` (`LspState::close_document`, then `Change::Closed`).
    pub fn close(&mut self, file: &str) -> Option<Applying> {
        let uri = self.uri(file);
        self.state.close_document(uri.as_str());
        self.apply(Change::Closed(uri))
    }

    /// Write `text` at `file` on disk (no event).
    pub fn write(&self, file: &str, text: &str) {
        let path = self.dir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// The hover markdown at a position.
    pub fn hover(&self, file: &str, line: u32, character: u32) -> Option<String> {
        let hover = answers::hover(&self.state, &self.uri(file), Position::new(line, character))?;
        match hover.contents {
            HoverContents::Markup(markup) => Some(markup.value),
            other => panic!("a hover of markup, not {other:?}"),
        }
    }

    /// The hover on the declaration of `id` in `file`: the first line whose
    /// second word is `id`.
    pub fn hover_on(&self, file: &str, id: &str) -> Option<String> {
        let text = std::fs::read_to_string(self.dir.path().join(file)).ok()?;
        let (line, column) = text.lines().enumerate().find_map(|(n, line)| {
            let mut words = line.split_whitespace();
            words.next()?;
            (words.next()? == id).then(|| (n, line.find(id).unwrap()))
        })?;
        self.hover(file, line as u32, column as u32 + 1)
    }

    /// `f` over the state while the session is out for an update (its
    /// stand-in).
    pub fn rebuilding<R>(&mut self, f: impl FnOnce(&LspState) -> R) -> R {
        let session = self.state.take_session().expect("held");
        let result = f(&self.state);
        self.state.set_session(session);
        result
    }
}

/// Apply `change` to the session `state` holds as the backend does:
/// `Plan::of`, the session taken out (a project opened with `runtime`
/// replaces it for `Change::Open`), `Plan::apply`, put back, the publication
/// recorded. `None` when the plan asks nothing.
pub fn apply_change(
    state: &mut LspState,
    change: Change,
    runtime: Option<SharedRuntime>,
) -> Option<Applying> {
    let plan = Plan::of(change, state)?;
    let mut session = state.take_session()?;
    if let Some(root) = plan.root() {
        session = ProjectSession::open_with_runtime(root, runtime);
    }
    let applied = plan.apply(&mut session);
    state.set_session(session);
    let publication = applied.publication(state);
    if let Some(publication) = &publication {
        state.record(publication);
    }
    Some((applied, publication))
}

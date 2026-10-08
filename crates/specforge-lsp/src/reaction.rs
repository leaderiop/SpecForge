//! The LSP's one reaction to a change the client reports (ADR 0035): apply
//! it to the project session, publish what the project reports, then, when
//! what the session is built from moved, ask the client to watch it and
//! catch up on what changed on disk while it did not, and last tell the
//! client its highlighting may be stale. Every handler that changes the
//! session goes through [`Reaction::react`]; none decides these steps for
//! itself.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Mutex, RwLock};
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use specforge_project::{OpeningProject, ProjectSession};

use crate::LspState;
use crate::changes::{Applied, Change, Plan};

/// How many times the client's watchers follow the session after one
/// change: each round needs the previous catch-up to have moved what the
/// session is built from again, so it ends long before this in practice.
const FOLLOW_ROUNDS: usize = 8;

/// What the backend's handlers and its reparse worker share to react to a
/// change.
#[derive(Clone)]
pub(crate) struct Reaction {
    client: Client,
    state: Arc<RwLock<LspState>>,
    /// Held by every change to the project session (edits, files changed
    /// on disk, extension reloads, opening the project), so changes apply
    /// one at a time and none is lost to another.
    updates: Arc<Mutex<()>>,
    /// Whether the client declared `workspace.semanticTokens.refreshSupport`
    /// at initialize: only then is it sent `workspace/semanticTokens/refresh`.
    tokens_refresh_support: Arc<AtomicBool>,
    /// The file watchers the client was asked to register
    /// ([`crate::watchers`]), so a change to what the project is built from
    /// re-registers.
    watched: Arc<Mutex<Vec<FileSystemWatcher>>>,
    /// Whether the client declared
    /// `workspace.didChangeWatchedFiles.relativePatternSupport`.
    relative_patterns: Arc<AtomicBool>,
}

impl Reaction {
    pub(crate) fn new(client: Client, state: Arc<RwLock<LspState>>) -> Self {
        Self {
            client,
            state,
            updates: Arc::new(Mutex::new(())),
            tokens_refresh_support: Arc::new(AtomicBool::new(false)),
            watched: Arc::new(Mutex::new(Vec::new())),
            relative_patterns: Arc::new(AtomicBool::new(false)),
        }
    }

    /// What the client declared at initialize that decides how it is
    /// reacted to.
    pub(crate) fn declared(&self, tokens_refresh_support: bool, relative_patterns: bool) {
        self.tokens_refresh_support
            .store(tokens_refresh_support, Ordering::Relaxed);
        self.relative_patterns
            .store(relative_patterns, Ordering::Relaxed);
    }

    /// Apply `change` to the project session and react to what it did:
    /// the project's diagnostics are published, the client's watchers
    /// follow (and the session catches up) when what the session is built
    /// from moved, and an editor whose highlighting went stale is asked to
    /// ask again. `None` when there was nothing to apply, or it could not
    /// be applied.
    pub(crate) async fn react(&self, change: Change) -> Option<Applied> {
        let opens = matches!(change, Change::Open(_));
        let applied = self.apply(change).await;
        if let Some(applied) = &applied {
            if applied.environment && !opens {
                // The environment loaded again (hardening-plan H4 / R-5):
                // the spec root re-indexed and everything republished.
                let ext_count = self.state.read().await.registries().declarations().len();
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!(
                            "specforge-lsp: extension environment changed, reloaded {ext_count} extension(s)"
                        ),
                    )
                    .await;
            }
            if applied.rewatch() {
                self.follow().await;
            }
        }
        self.refresh_semantic_tokens_if_stale().await;
        applied
    }

    /// Register the watchers that hold until the project is open: every
    /// `.spec`, config and lock file. Once it is, the watchers cover exactly
    /// what it is built from ([`Self::follow`]).
    pub(crate) async fn watch_defaults(&self) {
        let defaults = crate::watchers::default_watchers();
        if Self::register_watchers(&self.client, defaults.clone())
            .await
            .is_ok()
        {
            *self.watched.lock().await = defaults;
        }
    }

    /// Apply `change` to the project session, the one `specforge watch`
    /// holds, and publish everything the project reports now: the
    /// diagnostics `specforge check` reports for the same sources and
    /// buffers. What the change asks of the session is [`Plan::of`]'s to
    /// say. Returns `None` when there was nothing to apply, or it could not
    /// be applied.
    ///
    /// Changes apply one at a time (`updates`). The session does
    /// synchronous file reads and whole-graph checks, so it runs on the
    /// blocking pool: it is taken out of the state (a brief write lock),
    /// updated without any lock held, and put back. Meanwhile readers see
    /// its last complete graph and environment (C4-05).
    pub(crate) async fn apply(&self, change: Change) -> Option<Applied> {
        let (state, client) = (&self.state, &self.client);
        let _one_at_a_time = self.updates.lock().await;
        let (session, plan) = {
            let mut st = state.write().await;
            let plan = Plan::of(change, &st)?;
            (st.take_session()?, plan)
        };

        // Opening a project loads its environment first and shows it to
        // readers before the sources are read: the kinds and fields
        // keyword completion offers need no `.spec` file, so they are
        // answered while indexing runs (CONTEXT: Environment).
        let opening = match plan.root().map(Path::to_path_buf) {
            Some(root) => {
                match tokio::task::spawn_blocking(move || ProjectSession::begin_open(&root)).await {
                    Ok(loaded) => {
                        state
                            .write()
                            .await
                            .show_environment(Arc::clone(loaded.environment()));
                        Some(loaded)
                    }
                    Err(e) => {
                        self.lose_session(e).await;
                        return None;
                    }
                }
            }
            None => None,
        };

        let joined = tokio::task::spawn_blocking(move || {
            let mut session = opening.map_or(session, OpeningProject::finish);
            let applied = plan.apply(&mut session);
            (session, applied)
        })
        .await;

        match joined {
            Ok((session, applied)) => {
                // Every session verifies its updates in a debug build (ADR
                // 0032): a rebuild that differs from a cold build is
                // reported here.
                for divergence in &applied.divergences {
                    client
                        .log_message(
                            MessageType::ERROR,
                            format!(
                                "an incremental rebuild diverged from a cold build: {divergence}"
                            ),
                        )
                        .await;
                }
                debug_assert!(applied.divergences.is_empty(), "{:?}", applied.divergences);
                state.write().await.set_session(session);
                self.publish(&applied).await;
                Some(applied)
            }
            Err(e) => {
                self.lose_session(e).await;
                None
            }
        }
    }

    /// An update panicked: the session is lost, so the state falls back to
    /// an empty one rather than a stale stand-in.
    async fn lose_session(&self, error: tokio::task::JoinError) {
        self.state
            .write()
            .await
            .set_session(ProjectSession::detached());
        self.client
            .log_message(
                MessageType::ERROR,
                format!("specforge-lsp: recompile failed: {error}"),
            )
            .await;
    }

    /// What the session is built from moved: the client watches it now
    /// ([`Self::sync_watchers`]), then the session catches up on what
    /// changed on disk while it did not ([`Change::CatchUp`]), again while
    /// that catch-up moves it (ADR 0035; at most `FOLLOW_ROUNDS`).
    async fn follow(&self) {
        for _ in 0..FOLLOW_ROUNDS {
            if !self.sync_watchers().await {
                return;
            }
            let caught_up = self.apply(Change::CatchUp).await;
            if !caught_up.is_some_and(|applied| applied.rewatch()) {
                return;
            }
        }
    }

    /// Ask the client to watch every file the project is built from
    /// ([`crate::watchers::file_watchers`]), replacing the watchers it was
    /// asked for before when they differ. A client that refuses the
    /// project's watchers keeps the static ones. Whether it registered a
    /// different set.
    async fn sync_watchers(&self) -> bool {
        let relative_patterns = self.relative_patterns.load(Ordering::Relaxed);
        let wanted = {
            let st = self.state.read().await;
            match st.session() {
                Some(session) => {
                    crate::watchers::file_watchers(session.inputs(), relative_patterns)
                }
                None => return false,
            }
        };
        let mut watched = self.watched.lock().await;
        if *watched == wanted {
            return false;
        }
        let _ = self
            .client
            .unregister_capability(vec![Unregistration {
                id: crate::watchers::REGISTRATION_ID.into(),
                method: "workspace/didChangeWatchedFiles".into(),
            }])
            .await;
        *watched = match Self::register_watchers(&self.client, wanted.clone()).await {
            Ok(()) => wanted,
            Err(_) => {
                let defaults = crate::watchers::default_watchers();
                let _ = Self::register_watchers(&self.client, defaults.clone()).await;
                defaults
            }
        };
        true
    }

    /// Register `watchers` for `workspace/didChangeWatchedFiles`.
    async fn register_watchers(client: &Client, watchers: Vec<FileSystemWatcher>) -> Result<()> {
        client
            .register_capability(vec![Registration {
                id: crate::watchers::REGISTRATION_ID.into(),
                method: "workspace/didChangeWatchedFiles".into(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions { watchers })
                        .expect("watcher options serialize"),
                ),
            }])
            .await
    }

    /// Publish what `applied` says the project reports now
    /// ([`Applied::publication`]). What is published is kept: code actions
    /// act on it.
    async fn publish(&self, applied: &Applied) {
        let Some(publication) = applied.publication(&*self.state.read().await) else {
            return;
        };
        self.state.write().await.record(&publication);
        for (uri, file) in publication.files {
            self.client
                .publish_diagnostics(uri, file.diagnostics, file.version)
                .await;
        }
    }

    /// After a recompile: when the graph changed in anything semantic
    /// tokens depend on (entity IDs, kinds, titles, the kind registry's
    /// classification), ask a client that declared refreshSupport to
    /// re-request tokens. The LSP does not subscribe to watch deltas; this
    /// is how open editors learn their highlighting went stale. The request
    /// is sent from its own task so a slow client never stalls a recompile.
    async fn refresh_semantic_tokens_if_stale(&self) {
        let stale = self.state.write().await.record_token_signature();
        if stale && self.tokens_refresh_support.load(Ordering::Relaxed) {
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.semantic_tokens_refresh().await;
            });
        }
    }
}

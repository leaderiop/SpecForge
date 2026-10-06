//! What a client hears about, in both subscription eras (ADR 0024 D6).
//!
//! A resource's content changes with the graph or with the diagnostics:
//! [`Watched::of`] says which, and the handshake era's `resources/subscribe`
//! channels and the stateless era's `subscriptions/listen` streams both read
//! it. One update of the served project is computed once ([`Changes`]) and
//! asked what it [touched](Changes::touched).

use specforge_common::Diagnostic;
use specforge_project::Update;

use crate::notifications::{
    DIAGNOSTICS_CHANNEL, DiagnosticsDelta, GRAPH_CHANNEL, GraphDelta, compute_diagnostics_delta,
};
use crate::state::{McpState, Subscription};

/// What a resource's content changes with, in both subscription eras.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Watched {
    Graph,
    Diagnostics,
}

impl Watched {
    /// `specforge://diagnostics` changes with the diagnostics; every other
    /// resource the server serves (the graph views, the schema, an
    /// extension's) with the graph.
    pub fn of(uri: &str) -> Self {
        if uri == "specforge://diagnostics" {
            Watched::Diagnostics
        } else {
            Watched::Graph
        }
    }

    /// The handshake era's notification channel: `specforge/graphChanged`,
    /// `specforge/diagnosticsChanged`.
    pub fn channel(self) -> &'static str {
        match self {
            Watched::Graph => GRAPH_CHANNEL,
            Watched::Diagnostics => DIAGNOSTICS_CHANNEL,
        }
    }
}

/// What one update of the served project changed, computed once and read by
/// both eras.
pub struct Changes<'u> {
    pub graph: &'u GraphDelta,
    pub diagnostics: DiagnosticsDelta,
}

impl<'u> Changes<'u> {
    /// What `update` changed in the graph, and what the diagnostics the
    /// server reports went from `previous` to `current`.
    pub fn of(update: &'u Update, previous: &[Diagnostic], current: &[Diagnostic]) -> Self {
        Changes {
            graph: &update.delta,
            diagnostics: compute_diagnostics_delta(previous, current),
        }
    }

    /// Whether what `watched` names changed.
    pub fn touched(&self, watched: Watched) -> bool {
        match watched {
            Watched::Graph => !self.graph.is_empty(),
            Watched::Diagnostics => {
                !self.diagnostics.added.is_empty() || !self.diagnostics.removed.is_empty()
            }
        }
    }
}

/// `client` hears about `watched` (handshake era). False when it already
/// did.
pub fn subscribe(state: &mut McpState, client_id: &str, watched: Watched) -> bool {
    let channel = watched.channel();
    let subs = state.subscriptions.entry(channel.to_string()).or_default();

    // Don't duplicate
    if subs.iter().any(|s| s.client_id == client_id) {
        return false;
    }

    subs.push(Subscription {
        client_id: client_id.to_string(),
        channel: channel.to_string(),
    });
    state.push_event(
        "mcp_subscription_created",
        serde_json::json!({"subscriptionType": channel, "clientId": client_id}),
    );
    true
}

/// `client` no longer hears about `watched`. False when it did not.
pub fn unsubscribe(state: &mut McpState, client_id: &str, watched: Watched) -> bool {
    let channel = watched.channel();
    if let Some(subs) = state.subscriptions.get_mut(channel) {
        let before = subs.len();
        subs.retain(|s| s.client_id != client_id);
        let removed = subs.len() < before;
        if subs.is_empty() {
            state.subscriptions.remove(channel);
        }
        if removed {
            state.push_event(
                "mcp_subscription_removed",
                serde_json::json!({"subscriptionType": channel, "clientId": client_id}),
            );
            return true;
        }
    }
    false
}

pub fn unsubscribe_all(state: &mut McpState, client_id: &str) {
    for watched in [Watched::Graph, Watched::Diagnostics] {
        unsubscribe(state, client_id, watched);
    }
}

pub fn subscribers(state: &McpState, watched: Watched) -> Vec<&str> {
    state
        .subscriptions
        .get(watched.channel())
        .map(|subs| subs.iter().map(|s| s.client_id.as_str()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "listen_for_mcp_resource_updates",
        verify = "both eras decide what a change touches by one rule"
    )]
    fn watched_follows_the_uri() {
        assert_eq!(Watched::of("specforge://diagnostics"), Watched::Diagnostics);
        for uri in [
            "specforge://graph",
            "specforge://graph/alpha",
            "specforge://context?scope=alpha",
            "specforge://schema",
            "acme://doc/1",
        ] {
            assert_eq!(Watched::of(uri), Watched::Graph, "{uri}");
        }
        assert_eq!(Watched::Graph.channel(), "specforge/graphChanged");
        assert_eq!(
            Watched::Diagnostics.channel(),
            "specforge/diagnosticsChanged"
        );
    }
}

//! Who hears about what (ADR 0024 D6): the resources the client subscribed
//! to with `resources/subscribe` (the handshake revisions), the streams it
//! opened with `subscriptions/listen` (MCP 2026-07-28), and the
//! notifications queued for it until the host sends them. One rule says what
//! a resource's content changes with ([`Watched::of`]); one update's changes
//! are computed once ([`Changes`]), in `McpState::applied`, the one place
//! every update of the served project passes (ADR 0035 D3), and every
//! subscription they touched hears about them ([`Subscriptions::updated`]).
//! Subscriptions belong to the connection: they end with it, or at shutdown.
//! The requests that change them are in [`requests`]; the notifications'
//! JSON shapes in `wire`.

pub(crate) mod requests;
mod wire;

use std::collections::HashSet;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_project::{GraphDelta, Update};

use crate::types::McpEvent;

/// What a resource's content changes with: the one rule both eras read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Watched {
    /// The graph: every graph view (`graph`, `context`, `brief`, an entity's
    /// subgraph, a kind's entities), the schema, and every extension
    /// resource.
    Graph,
    /// What the server reports: `specforge://diagnostics`.
    Diagnostics,
}

impl Watched {
    /// What `uri`'s content changes with. `uri` is one the server serves
    /// (the requests check it first).
    pub fn of(uri: &str) -> Self {
        if uri == "specforge://diagnostics" {
            Watched::Diagnostics
        } else {
            Watched::Graph
        }
    }

    /// The handshake era's notification method for this aspect.
    fn channel(self) -> &'static str {
        match self {
            Watched::Graph => wire::GRAPH_CHANGED,
            Watched::Diagnostics => wire::DIAGNOSTICS_CHANGED,
        }
    }
}

/// The diagnostics added and removed by one update.
#[derive(Debug, Default)]
struct DiagnosticsDelta {
    added: Vec<Diagnostic>,
    removed: Vec<Diagnostic>,
}

impl DiagnosticsDelta {
    /// What names a diagnostic in a delta: its code, its message and its
    /// file.
    fn identity(diagnostic: &Diagnostic) -> (&str, &str, &str) {
        (
            diagnostic.code.as_str(),
            diagnostic.message.as_str(),
            diagnostic
                .span
                .as_ref()
                .map_or("", |span| span.file.as_str()),
        )
    }

    /// The diagnostics in `after` and not in `before`, and the reverse.
    fn between(before: &[Diagnostic], after: &[Diagnostic]) -> Self {
        let before_keys: HashSet<_> = before.iter().map(Self::identity).collect();
        let after_keys: HashSet<_> = after.iter().map(Self::identity).collect();
        DiagnosticsDelta {
            added: after
                .iter()
                .filter(|d| !before_keys.contains(&Self::identity(d)))
                .cloned()
                .collect(),
            removed: before
                .iter()
                .filter(|d| !after_keys.contains(&Self::identity(d)))
                .cloned()
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// What one update of the served project changed, computed once and read by
/// both eras.
pub struct Changes<'u> {
    graph: &'u GraphDelta,
    diagnostics: Option<DiagnosticsDelta>,
}

impl<'u> Changes<'u> {
    /// What `update` changed. `diagnostics` is what the server reported
    /// before the update and reports after it, read only when someone hears
    /// about the diagnostics ([`Subscriptions::hears_diagnostics`]); `None`
    /// reads as "unchanged".
    pub fn of(update: &'u Update, diagnostics: Option<(&[Diagnostic], &[Diagnostic])>) -> Self {
        Self::new(&update.delta, diagnostics)
    }

    /// The same from its parts: the graph delta, and the diagnostics before
    /// and after.
    pub fn new(graph: &'u GraphDelta, diagnostics: Option<(&[Diagnostic], &[Diagnostic])>) -> Self {
        Changes {
            graph,
            diagnostics: diagnostics
                .map(|(before, after)| DiagnosticsDelta::between(before, after)),
        }
    }

    /// Whether what `watched` names changed: the graph delta is not empty
    /// (`Graph`); a diagnostic was added or removed, named by its code,
    /// message and file (`Diagnostics`).
    pub fn touched(&self, watched: Watched) -> bool {
        match watched {
            Watched::Graph => !self.graph.is_empty(),
            Watched::Diagnostics => self
                .diagnostics
                .as_ref()
                .is_some_and(|delta| !delta.is_empty()),
        }
    }
}

/// One `subscriptions/listen` stream (MCP 2026-07-28): the resources it
/// asked to hear about, and the listen request's id every notification on it
/// carries as `io.modelcontextprotocol/subscriptionId`.
#[derive(Debug, Clone, PartialEq)]
struct Stream {
    id: Value,
    uris: Vec<String>,
}

/// Who hears about what, and what waits to be sent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Subscriptions {
    /// `resources/subscribe`: what the client hears about, once each.
    channels: Vec<Watched>,
    /// `subscriptions/listen`: the open streams, in opening order.
    streams: Vec<Stream>,
    /// Queued notifications, oldest first.
    outbox: Vec<Value>,
}

impl Subscriptions {
    /// Nothing subscribed, nothing open, nothing queued.
    pub fn new() -> Self {
        Self::default()
    }

    /// `resources/subscribe`: the client hears when what `uri` changes with
    /// changes, until it unsubscribes or the connection ends. False when it
    /// already did. The request checked that the server serves `uri`.
    /// Records `mcp_subscription_created`.
    pub fn subscribe(&mut self, uri: &str, events: &mut Vec<McpEvent>) -> bool {
        let watched = Watched::of(uri);
        if self.channels.contains(&watched) {
            return false;
        }
        self.channels.push(watched);
        events.push(subscription_event(
            "mcp_subscription_created",
            watched.channel(),
            "default",
        ));
        true
    }

    /// `resources/unsubscribe`: the client no longer hears about what `uri`
    /// changes with. False when it did not. Records
    /// `mcp_subscription_removed`.
    pub fn unsubscribe(&mut self, uri: &str, events: &mut Vec<McpEvent>) -> bool {
        let watched = Watched::of(uri);
        let Some(position) = self.channels.iter().position(|held| *held == watched) else {
            return false;
        };
        self.channels.remove(position);
        events.push(subscription_event(
            "mcp_subscription_removed",
            watched.channel(),
            "default",
        ));
        true
    }

    /// `subscriptions/listen`: open the stream `id` on `uris`, queueing its
    /// acknowledgement before anything else on it. Records
    /// `mcp_subscription_created` per resource.
    pub fn listen(&mut self, id: Value, uris: Vec<String>, events: &mut Vec<McpEvent>) {
        self.outbox.push(wire::acknowledged(&id, &uris));
        for uri in &uris {
            events.push(subscription_event(
                "mcp_subscription_created",
                uri,
                &id.to_string(),
            ));
        }
        self.streams.retain(|open| open.id != id);
        self.streams.push(Stream { id, uris });
    }

    /// The client cancelled the listen request `id`: its stream ends, and
    /// nothing more is sent on it, not even a response. False when no stream
    /// had that id. Records `mcp_subscription_removed` per resource.
    pub fn end(&mut self, id: &Value, events: &mut Vec<McpEvent>) -> bool {
        let Some(position) = self.streams.iter().position(|open| &open.id == id) else {
            return false;
        };
        let ended = self.streams.remove(position);
        for uri in &ended.uris {
            events.push(subscription_event(
                "mcp_subscription_removed",
                uri,
                &ended.id.to_string(),
            ));
        }
        true
    }

    /// The connection ended, or the server shuts down: every subscription
    /// and every stream ends, each removal recorded. The queue stays: the
    /// host sends it after the last response. How many subscriptions ended
    /// (a stream counts one per resource).
    pub fn disconnect(&mut self, events: &mut Vec<McpEvent>) -> usize {
        let mut ended = 0;
        for watched in self.channels.drain(..) {
            events.push(subscription_event(
                "mcp_subscription_removed",
                watched.channel(),
                "default",
            ));
            ended += 1;
        }
        for stream in self.streams.drain(..) {
            for uri in &stream.uris {
                events.push(subscription_event(
                    "mcp_subscription_removed",
                    uri,
                    &stream.id.to_string(),
                ));
                ended += 1;
            }
        }
        ended
    }

    /// Whether anyone hears about the diagnostics: only then does an update
    /// read them before and after it applies.
    pub fn hears_diagnostics(&self) -> bool {
        self.hears(Watched::Diagnostics)
            || self
                .streams
                .iter()
                .flat_map(|stream| &stream.uris)
                .any(|uri| Watched::of(uri) == Watched::Diagnostics)
    }

    /// After an update of the served project: queue, for each open stream,
    /// `notifications/resources/updated` (with the stream's id) for each
    /// resource it names that `changes` touched; then `specforge/graphChanged`
    /// and `specforge/diagnosticsChanged` for each subscribed aspect that
    /// changed. Records `mcp_delta_notified` per delta queued.
    pub fn updated(&mut self, changes: &Changes<'_>, events: &mut Vec<McpEvent>) {
        for stream in &self.streams {
            for uri in &stream.uris {
                if changes.touched(Watched::of(uri)) {
                    self.outbox
                        .push(wire::resource_updated(uri, Some(&stream.id)));
                }
            }
        }
        for watched in [Watched::Graph, Watched::Diagnostics] {
            if !self.hears(watched) || !changes.touched(watched) {
                continue;
            }
            let (notification, summary) = match (watched, &changes.diagnostics) {
                (Watched::Graph, _) => (
                    wire::graph_changed(changes.graph),
                    json!({
                        "notificationType": "graph",
                        "subscriberCount": 1,
                        "addedNodes": changes.graph.added_nodes.len(),
                        "removedNodes": changes.graph.removed_nodes.len(),
                        "modifiedNodes": changes.graph.modified_nodes.len(),
                    }),
                ),
                (Watched::Diagnostics, Some(delta)) => (
                    wire::diagnostics_changed(delta),
                    json!({
                        "notificationType": "diagnostics",
                        "subscriberCount": 1,
                        "addedDiagnostics": delta.added.len(),
                        "removedDiagnostics": delta.removed.len(),
                    }),
                ),
                (Watched::Diagnostics, None) => continue,
            };
            self.outbox.push(notification);
            events.push(McpEvent::new("mcp_delta_notified", summary));
        }
    }

    /// Take the queued notifications, oldest first.
    pub fn drain(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.outbox)
    }

    /// How many notifications wait (what shutdown reports flushing).
    pub fn pending(&self) -> usize {
        self.outbox.len()
    }

    /// Whether the client subscribed to a resource that changes with
    /// `watched`.
    pub fn hears(&self, watched: Watched) -> bool {
        self.channels.contains(&watched)
    }

    /// Nothing subscribed and no stream open.
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty() && self.streams.is_empty()
    }
}

/// An `mcp_subscription_created` / `_removed` event.
fn subscription_event(name: &str, subscription_type: &str, client_id: &str) -> McpEvent {
    McpEvent::new(
        name,
        json!({"subscriptionType": subscription_type, "clientId": client_id}),
    )
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

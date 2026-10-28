//! What a project keeps of its digests between updates (ADR 0290): an entity's digest until the graph delta names the
//! entity, a file's digest until its stamp changes. A kept digest only saves work: the same digest is computed cold
//! when the memo is empty, and a verifying session compares the two after each check.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use specforge_graph::{Graph, GraphDelta};
use specforge_registry::FileStamp;

use super::entity_digest;

#[derive(Debug, Default)]
pub struct DigestMemo {
    entities: Mutex<HashMap<String, String>>,
    files: Mutex<HashMap<PathBuf, (FileStamp, String)>>,
}

impl Clone for DigestMemo {
    fn clone(&self) -> Self {
        Self {
            entities: Mutex::new(self.entities.lock().expect("the memo").clone()),
            files: Mutex::new(self.files.lock().expect("the memo").clone()),
        }
    }
}

impl DigestMemo {
    pub(super) fn entity(&self, id: &str, cold: impl FnOnce() -> String) -> String {
        if let Some(kept) = self.entities.lock().expect("the memo").get(id) {
            return kept.clone();
        }
        let digest = cold();
        self.entities
            .lock()
            .expect("the memo")
            .insert(id.to_string(), digest.clone());
        digest
    }

    /// The digest kept for `path` at `stamp`, else `cold()` kept against it.
    pub(super) fn file(
        &self,
        path: &std::path::Path,
        stamp: &FileStamp,
        cold: impl FnOnce() -> String,
    ) -> String {
        if let Some((kept, digest)) = self.files.lock().expect("the memo").get(path)
            && kept == stamp
        {
            return digest.clone();
        }
        let digest = cold();
        self.files
            .lock()
            .expect("the memo")
            .insert(path.to_path_buf(), (stamp.clone(), digest.clone()));
        digest
    }

    /// This memo without what `delta` changed: the entities it adds, removes and modifies, and the sources of the
    /// edges it adds and removes (an entity digests the full path of what it names, which a rename elsewhere moves).
    #[must_use]
    pub fn after(&self, delta: &GraphDelta) -> Self {
        let next = self.clone();
        {
            let mut entities = next.entities.lock().expect("the memo");
            let ids = delta
                .added_nodes
                .iter()
                .chain(&delta.removed_nodes)
                .map(|n| &n.id)
                .chain(delta.modified_nodes.iter().map(|n| &n.id))
                .chain(
                    delta
                        .added_edges
                        .iter()
                        .chain(&delta.removed_edges)
                        .map(|e| &e.source),
                );
            for id in ids {
                entities.remove(id);
            }
        }
        next
    }

    /// Whether every kept digest is what a cold evaluation over `graph` gives now; `Err` names the first that is not.
    pub fn verify(
        &self,
        graph: &Graph,
        forms: &specforge_graph::FieldForms,
        disk: &dyn specforge_registry::Disk,
    ) -> Result<(), String> {
        for (id, kept) in self.entities.lock().expect("the memo").iter() {
            let Some(node) = graph.node(id) else {
                continue;
            };
            let cold = entity_digest(graph, forms, node);
            if &cold != kept {
                return Err(format!("entity digest of '{id}': kept {kept}, cold {cold}"));
            }
        }
        for (path, (stamp, kept)) in self.files.lock().expect("the memo").iter() {
            // A file changed since its stamp is not a divergence: the next check stamps it again.
            if disk.stamp(path).ok().as_ref() != Some(stamp) {
                continue;
            }
            let Ok(bytes) = disk.read(path) else {
                continue;
            };
            let cold = super::file_digest(&bytes);
            if &cold != kept {
                return Err(format!(
                    "file digest of '{}': kept {kept}, cold {cold}",
                    path.display()
                ));
            }
        }
        Ok(())
    }
}

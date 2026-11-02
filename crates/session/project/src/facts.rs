//! A collector's facts (ADR 0327): the store a collect writes, the key it files it under, and what the host admits.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specforge_protocol_types::Fact;
use specforge_registry::{Disk, KindRegistry};
pub use specforge_registry::{FACTS_DIR, filed, is_fact_store};

use crate::Environment;
use crate::history::Located;

/// One store file, `.specforge/facts/<key>/<collector>.json`: the facts one collector answered, filed under one key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Store {
    /// The store format, [`Store::VERSION`].
    pub store: u32,
    pub collector: String,
    pub extension: String,
    pub key: String,
    pub facts: Vec<Fact>,
}

impl Store {
    pub const VERSION: u32 = 1;

    /// The store of `facts`, sorted by kind and id.
    pub fn new(collector: &str, extension: &str, key: &str, mut facts: Vec<Fact>) -> Self {
        facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        Store {
            store: Self::VERSION,
            collector: collector.to_string(),
            extension: extension.to_string(),
            key: key.to_string(),
            facts,
        }
    }

    /// The canonical bytes (ADR 0327 D2): two-space JSON, fields in key order, a final newline. Equal facts give
    /// equal bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut text = serde_json::to_string_pretty(self).expect("a store serializes");
        text.push('\n');
        text.into_bytes()
    }

    /// The store file of `collector` under `key`, from the project root.
    pub fn path(collector: &str, key: &str) -> String {
        format!("{FACTS_DIR}/{key}/{collector}.json")
    }
}

/// Why a collect's key is not a clean commit's sha (I209).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyReason {
    /// The tree holds a tracked change, a conflict or an untracked file: `<sha>-dirty`.
    Dirty,
    /// The project is in no repository: `worktree`.
    NoRepository,
    /// HEAD names no commit yet: `worktree`.
    UnbornHead,
    /// Git can't be asked, and why: `worktree`.
    NoHistory(String),
}

impl KeyReason {
    /// What I209 says of it.
    pub fn says(&self) -> String {
        match self {
            KeyReason::Dirty => "the tree has changes git does not hold at HEAD".to_string(),
            KeyReason::NoRepository => "the project is in no git repository".to_string(),
            KeyReason::UnbornHead => "HEAD names no commit yet".to_string(),
            KeyReason::NoHistory(why) => format!("git can't be asked ({why})"),
        }
    }
}

/// The key a collect files its facts under (ADR 0327 D3), asked through the history port: HEAD's sha when the tree
/// is clean, `<sha>-dirty` when it is not, `worktree` when there is no commit to name; with why, unless it is clean.
pub fn key(env: &Environment) -> (String, Option<KeyReason>) {
    const WORKTREE: &str = "worktree";
    let Some(history) = env.history() else {
        return (
            WORKTREE.into(),
            Some(KeyReason::NoHistory("no history is installed".into())),
        );
    };
    let place = match env.place(history.as_ref()) {
        Located::Repo(place) => place,
        Located::NotARepository => return (WORKTREE.into(), Some(KeyReason::NoRepository)),
        Located::Unavailable(why) => return (WORKTREE.into(), Some(KeyReason::NoHistory(why))),
    };
    let sha = match history.commit(&place, "HEAD") {
        Ok(Some(sha)) => sha,
        Ok(None) => return (WORKTREE.into(), Some(KeyReason::UnbornHead)),
        Err(error) => return (WORKTREE.into(), Some(KeyReason::NoHistory(error.0))),
    };
    match history.status(&place) {
        Ok(status) if status.dirty => (format!("{}-dirty", sha.as_str()), Some(KeyReason::Dirty)),
        Ok(_) => (sha.as_str().to_string(), None),
        Err(error) => (WORKTREE.into(), Some(KeyReason::NoHistory(error.0))),
    }
}

/// The fact stores under the project root `root` now, as source keys, sorted: every
/// `.specforge/facts/<key>/<collector>.json`, listed through `disk`.
pub fn keys(disk: &dyn Disk, root: &Path) -> Vec<String> {
    let dir = root.join(FACTS_DIR);
    let mut keys = Vec::new();
    for key in disk.list(&dir).unwrap_or_default() {
        for file in disk.list(&dir.join(&key)).unwrap_or_default() {
            let store = format!("{FACTS_DIR}/{key}/{file}");
            if is_fact_store(&store) {
                keys.push(store);
            }
        }
    }
    keys.sort();
    keys
}

/// The stores of `collector` under `root` filed under a key other than `key`, listed through `disk`: what a collect
/// that files its facts under `key` replaces (ADR 0327 D4).
pub fn superseded(disk: &dyn Disk, root: &Path, collector: &str, key: &str) -> Vec<String> {
    keys(disk, root)
        .into_iter()
        .filter(|store| filed(store).is_some_and(|(by, under)| by == collector && under != key))
        .collect()
}

/// Why the host refuses `facts` (E116, ADR 0327 Decision 6), each a reason: a kind that is no loaded collected kind,
/// an id that is empty or names none of its kind's schemes, an id the answer repeats. None: every fact is admitted.
pub fn admit(facts: &[Fact], kinds: &KindRegistry) -> Result<(), Vec<String>> {
    let mut reasons = Vec::new();
    let mut seen = BTreeSet::new();
    for fact in facts {
        let Some(kind) = kinds.collected(&fact.kind) else {
            reasons.push(format!(
                "fact '{}' is of the kind '{}', which no loaded extension declares a collector writes",
                fact.id, fact.kind
            ));
            continue;
        };
        if fact.id.is_empty() {
            reasons.push(format!(
                "a fact of the kind '{}' has an empty id",
                fact.kind
            ));
            continue;
        }
        if !specforge_common::scheme::of(&fact.id)
            .is_some_and(|scheme| kind.schemes().iter().any(|s| s == scheme))
        {
            reasons.push(format!(
                "fact '{}' names none of the schemes of the kind '{}' ({})",
                fact.id,
                fact.kind,
                kind.schemes().join(", ")
            ));
            continue;
        }
        if !seen.insert((fact.kind.as_str(), fact.id.as_str())) {
            reasons.push(format!("fact '{}' is answered twice", fact.id));
        }
    }
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_is_written_canonically_whatever_order_its_facts_came_in() {
        let a = Fact::new("test", "js:b#2").field("z", 1).field("a", "x");
        let b = Fact::new("test", "js:a#1").title("one");
        let one = Store::new("node", "@t/node", "k", vec![a.clone(), b.clone()]);
        let two = Store::new("node", "@t/node", "k", vec![b, a]);

        assert_eq!(one.bytes(), two.bytes());
        let text = String::from_utf8(one.bytes()).unwrap();
        assert!(
            text.ends_with("}\n") && text.contains("\n  \"store\": 1,"),
            "{text}"
        );
        assert!(text.find("js:a#1").unwrap() < text.find("js:b#2").unwrap());
        assert!(text.find("\"a\": \"x\"").unwrap() < text.find("\"z\": 1").unwrap());
        assert_eq!(Store::path("node", "k"), ".specforge/facts/k/node.json");
    }
}

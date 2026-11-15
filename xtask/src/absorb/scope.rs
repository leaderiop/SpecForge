//! Which old entities a run converts (ADR 0360 D4): every one of the absorbed kinds, or the ids `--only ids:<file>`
//! names. A reference or an inverse write onto an entity outside the scope is ledgered, never stubbed.

use std::collections::BTreeSet;
use std::path::Path;

use super::read::OldEntity;
use super::table;

/// The ids a run converts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    All,
    Only(BTreeSet<String>),
}

impl Scope {
    /// `--only`'s value: `ids:<file>`, the file a JSON array of old ids.
    pub fn from_arg(arg: &str) -> Result<Scope, String> {
        let file = arg
            .strip_prefix("ids:")
            .ok_or_else(|| format!("--only takes ids:<file>, not '{arg}'"))?;
        let text =
            std::fs::read_to_string(Path::new(file)).map_err(|e| format!("--only {file}: {e}"))?;
        let ids: Vec<String> = serde_json::from_str(&text)
            .map_err(|e| format!("--only {file}: a JSON array of ids: {e}"))?;
        Ok(Scope::Only(ids.into_iter().collect()))
    }

    /// The entities of `all` (every old entity of the absorbed kinds) this scope converts, in their order; refused
    /// when it names an id that is no such entity, or one whose rows create nothing (a module, a release).
    pub fn select(&self, all: Vec<OldEntity>) -> Result<Vec<OldEntity>, String> {
        let Scope::Only(ids) = self else {
            return Ok(all);
        };
        let mut refused = Vec::new();
        for id in ids {
            match all.iter().find(|e| &e.id == id) {
                None => refused.push(format!("'{id}' names no entity of the absorbed kinds")),
                Some(e) if table::new_kind(&e.kind).is_none() => {
                    refused.push(format!("'{id}' is a {}, which creates no entity", e.kind));
                }
                Some(_) => {}
            }
        }
        if !refused.is_empty() {
            return Err(format!(
                "--only names {} id(s) that do not convert:\n  {}",
                refused.len(),
                refused.join("\n  ")
            ));
        }
        Ok(all.into_iter().filter(|e| ids.contains(&e.id)).collect())
    }
}

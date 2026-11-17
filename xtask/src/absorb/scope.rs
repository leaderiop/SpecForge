//! Which old entities a run converts (ADR 0360 D4): every authored entity of the source, or the ids `--only ids:<file>`
//! names. A reference to an entity outside the scope is ledgered `dropped` and never stubbed (ADR 0372 D4).

use std::collections::BTreeSet;
use std::path::Path;

use super::read::OldEntity;
use super::select;

/// The ids a run converts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    All,
    Only(BTreeSet<String>),
}

impl Scope {
    /// `--only`'s value: `ids:<file>`, the file a JSON array of old ids. A sample (`sample:<name>`) is read before
    /// this, so a value of neither form is refused naming both, and every sample.
    pub fn from_arg(arg: &str) -> Result<Scope, String> {
        let file = arg.strip_prefix("ids:").ok_or_else(|| {
            format!(
                "--only takes ids:<file> or sample:<name> (the samples are: {}), not '{arg}'",
                select::sample_names()
            )
        })?;
        let text =
            std::fs::read_to_string(Path::new(file)).map_err(|e| format!("--only {file}: {e}"))?;
        let ids: Vec<String> = serde_json::from_str(&text)
            .map_err(|e| format!("--only {file}: a JSON array of ids: {e}"))?;
        Ok(Scope::Only(ids.into_iter().collect()))
    }

    /// The entities of `all` (every authored entity of the source) this scope converts, in their order; refused when
    /// it names an id that is no such entity. An entity of any kind may be named (ADR 0372 D1): one dropped whole
    /// leaves its values in the ledger, and one of a kind with no fate stops the run's pre-checks.
    pub fn select(&self, all: Vec<OldEntity>) -> Result<Vec<OldEntity>, String> {
        let Scope::Only(ids) = self else {
            return Ok(all);
        };
        let refused: Vec<String> = ids
            .iter()
            .filter(|id| !all.iter().any(|e| &e.id == *id))
            .map(|id| format!("'{id}'"))
            .collect();
        if !refused.is_empty() {
            return Err(format!(
                "--only names {} id(s) that name no entity of the source:\n  {}",
                refused.len(),
                refused.join("\n  ")
            ));
        }
        Ok(all.into_iter().filter(|e| ids.contains(&e.id)).collect())
    }
}

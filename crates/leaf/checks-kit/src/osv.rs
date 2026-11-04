//! OSV's `v1/query`: the query for one release of a package, and the advisories its answer lists, withdrawn ones
//! dropped. Every field not read is skipped unread.

use serde::Deserialize;

use crate::coord::Ecosystem;

/// The URL a query is POSTed to.
pub const QUERY_URL: &str = "https://api.osv.dev/v1/query";

/// The JSON body that asks OSV for the advisories of `name`'s release `version`.
pub fn query(ecosystem: Ecosystem, name: &str, version: &str) -> String {
    serde_json::json!({
        "version": version,
        "package": { "name": name, "ecosystem": ecosystem.osv() }
    })
    .to_string()
}

/// An advisory: its id, its aliases (a CVE, a GHSA), and the first version a range of it names as fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advisory {
    pub id: String,
    pub aliases: Vec<String>,
    pub fixed: Option<String>,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    vulns: Vec<Vuln>,
}

#[derive(Deserialize)]
struct Vuln {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
    withdrawn: Option<String>,
    #[serde(default)]
    affected: Vec<Affected>,
}

#[derive(Deserialize)]
struct Affected {
    #[serde(default)]
    ranges: Vec<Ranges>,
}

#[derive(Deserialize)]
struct Ranges {
    #[serde(default)]
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct Event {
    fixed: Option<String>,
}

/// The advisories `body`, a `v1/query` answer, lists, withdrawn ones left out; why it does not read otherwise.
pub fn advisories(body: &[u8]) -> Result<Vec<Advisory>, String> {
    let answer: Answer = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    Ok(answer
        .vulns
        .into_iter()
        .filter(|vuln| vuln.withdrawn.is_none())
        .map(|vuln| {
            let fixed = vuln
                .affected
                .iter()
                .flat_map(|a| &a.ranges)
                .flat_map(|r| &r.events)
                .find_map(|e| e.fixed.clone());
            Advisory {
                id: vuln.id,
                aliases: vuln.aliases,
                fixed,
            }
        })
        .collect())
}

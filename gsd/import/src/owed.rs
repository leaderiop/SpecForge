//! The verdicts an import owes (ADR 0391 D5): `.planning/` records results, never a verdict, so every imported entity a
//! judged rule judges is listed with the commands that record its verdict. The targets mirror the rules' own:
//!
//! - `gsd/E501` (`gsd/core/src/judged.rs`): each truth, artifact and key link without a `pattern` of a complete plan,
//!   and a plan is complete when its summary's `status` is `complete` (`passes::derive::execute::plan_statuses`);
//! - `gsd.security/W302`: each threat whose `status` is `closed` (`complete_when status closed`);
//! - `gsd.security/W303`: each security audit, owed from its creation.
//!
//! 13-10 holds this prediction equal to what `specforge check` reports on the imported fixtures.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::paths;
use crate::records::Record;
use crate::settings::shell_quote;

pub const E501: &str = "gsd/E501";
pub const W302: &str = "gsd.security/W302";
pub const W303: &str = "gsd.security/W303";

/// One verdict owed: on `entity` (its graph path), by the rule `code`, in the phase `phase` (none for a quick plan's).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Owed {
    pub phase: Option<String>,
    pub code: &'static str,
    pub entity: String,
}

impl Owed {
    /// `{"code", "entity", "phase"}`.
    pub fn json(&self) -> Value {
        json!({ "code": self.code, "entity": self.entity, "phase": self.phase })
    }
}

/// The verdicts `kept`, the records the plan creates, owe, sorted by phase, code and entity.
pub fn owed<'r>(kept: impl IntoIterator<Item = &'r Record> + Clone) -> Vec<Owed> {
    let text = |r: &Record, field: &str| {
        r.fields
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let complete: BTreeSet<String> = kept
        .clone()
        .into_iter()
        .filter(|r| r.kind == "summary" && text(r, "status").as_deref() == Some("complete"))
        .filter_map(|r| text(r, "plan"))
        .collect();
    let mut out: Vec<Owed> = kept
        .into_iter()
        .filter_map(|r| {
            let code = match r.kind {
                "truth" | "artifact" => E501,
                "key_link" if !r.fields.contains_key("pattern") => E501,
                "threat" if text(r, "status").as_deref() == Some("closed") => W302,
                "security_audit" => W303,
                _ => return None,
            };
            if code == E501 && !text(r, "plan").is_some_and(|p| complete.contains(&p)) {
                return None;
            }
            let phase = r
                .file
                .strip_prefix(&format!("{}/phases/", paths::ROOT))
                .and_then(|f| f.strip_suffix(".spec"))
                .map(str::to_string);
            Some(Owed {
                phase,
                code,
                entity: r.id.clone(),
            })
        })
        .collect();
    out.sort();
    out
}

/// The report's owed lines for `owed` (sorted): per phase the slash command that records its verdicts
/// (`/gsd-verify-work <NN>` for `gsd/E501`, `/gsd-secure-phase <NN>` for `@gsd/security`'s), then per entity the query
/// that shows what its judge reads, then the count.
pub fn lines(owed: &[Owed]) -> Vec<String> {
    let mut out = Vec::new();
    let mut group = None;
    for o in owed {
        let command = if o.code == E501 {
            "/gsd-verify-work"
        } else {
            "/gsd-secure-phase"
        };
        if group != Some((&o.phase, command)) {
            group = Some((&o.phase, command));
            out.push(match &o.phase {
                Some(phase) => format!("{command} {phase}"),
                None => "quick plans (no phase): ask the verifier to judge each".to_string(),
            });
        }
        out.push(format!(
            "  specforge query {} --inputs {} --format json",
            shell_quote(&o.entity),
            o.code
        ));
    }
    if !owed.is_empty() {
        let check = if owed.iter().any(|o| o.code == E501) {
            "exits 1"
        } else {
            "warns"
        };
        out.push(format!(
            "{} verdicts owed: `specforge check` {check} until these verdicts are recorded",
            owed.len()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::records::Source;

    fn record(kind: &'static str, id: &str, file: &str, fields: Value) -> Record {
        Record {
            kind,
            id: id.to_string(),
            title: String::new(),
            file: file.to_string(),
            fields: serde_json::from_value::<BTreeMap<String, Value>>(fields).unwrap_or_default(),
            refs: Vec::new(),
            source: Source {
                file: String::new(),
                line: 1,
            },
        }
    }

    #[test]
    fn owed_mirrors_each_rules_targets() {
        let phase = "spec/gsd/phases/01.spec";
        let records = [
            record(
                "summary",
                "v1.0/01/01-01/01-01-SUMMARY",
                phase,
                json!({"plan": "v1.0/01/01-01", "status": "complete"}),
            ),
            record(
                "summary",
                "v1.0/01/01-02/01-02-SUMMARY",
                phase,
                json!({"plan": "v1.0/01/01-02", "status": "halted"}),
            ),
            record(
                "truth",
                "v1.0/01/01-01/TR1",
                phase,
                json!({"plan": "v1.0/01/01-01"}),
            ),
            record(
                "artifact",
                "v1.0/01/01-01/AR1",
                phase,
                json!({"plan": "v1.0/01/01-01"}),
            ),
            record(
                "key_link",
                "v1.0/01/01-01/KL1",
                phase,
                json!({"plan": "v1.0/01/01-01"}),
            ),
            record(
                "key_link",
                "v1.0/01/01-01/KL2",
                phase,
                json!({"plan": "v1.0/01/01-01", "pattern": "x"}),
            ),
            record(
                "truth",
                "v1.0/01/01-02/TR1",
                phase,
                json!({"plan": "v1.0/01/01-02"}),
            ),
            record(
                "threat",
                "v1.0/01/01-01/T-01-01",
                phase,
                json!({"status": "closed"}),
            ),
            record(
                "threat",
                "v1.0/01/01-02/T-01-02",
                phase,
                json!({"status": "open"}),
            ),
            record("security_audit", "v1.0/01/01-SECURITY", phase, json!({})),
            record(
                "summary",
                "261105-tcl/261105-tcl-SUMMARY",
                "spec/gsd/quick.spec",
                json!({"plan": "261105-tcl", "status": "complete"}),
            ),
            record(
                "truth",
                "261105-tcl/TR1",
                "spec/gsd/quick.spec",
                json!({"plan": "261105-tcl"}),
            ),
        ];
        let owed = owed(&records);
        let found: Vec<(Option<&str>, &str, &str)> = owed
            .iter()
            .map(|o| (o.phase.as_deref(), o.code, o.entity.as_str()))
            .collect();
        assert_eq!(
            found,
            // Codes sort as text: `gsd.security/` before `gsd/`.
            [
                (None, E501, "261105-tcl/TR1"),
                (Some("01"), W302, "v1.0/01/01-01/T-01-01"),
                (Some("01"), W303, "v1.0/01/01-SECURITY"),
                (Some("01"), E501, "v1.0/01/01-01/AR1"),
                (Some("01"), E501, "v1.0/01/01-01/KL1"),
                (Some("01"), E501, "v1.0/01/01-01/TR1"),
            ]
        );
        assert_eq!(
            lines(&owed),
            [
                "quick plans (no phase): ask the verifier to judge each",
                "  specforge query '261105-tcl/TR1' --inputs gsd/E501 --format json",
                "/gsd-secure-phase 01",
                "  specforge query 'v1.0/01/01-01/T-01-01' --inputs gsd.security/W302 --format json",
                "  specforge query 'v1.0/01/01-SECURITY' --inputs gsd.security/W303 --format json",
                "/gsd-verify-work 01",
                "  specforge query 'v1.0/01/01-01/AR1' --inputs gsd/E501 --format json",
                "  specforge query 'v1.0/01/01-01/KL1' --inputs gsd/E501 --format json",
                "  specforge query 'v1.0/01/01-01/TR1' --inputs gsd/E501 --format json",
                "6 verdicts owed: `specforge check` exits 1 until these verdicts are recorded",
            ]
        );
        assert_eq!(
            lines(&owed[1..3]).last().map(String::as_str),
            Some("2 verdicts owed: `specforge check` warns until these verdicts are recorded")
        );
    }
}

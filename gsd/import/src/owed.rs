//! The verdicts an import owes (ADR 0391 D5): `.planning/` records results, never a verdict, so every imported entity a
//! judged rule judges is listed with the commands that record its verdict. The targets mirror the rules' own:
//!
//! - `gsd/E501` (`gsd/core/src/judged.rs`): each truth, artifact and key link without a `pattern` of a complete plan,
//!   and a plan is complete when its summary's `status` is `complete` (`passes::derive::execute::plan_statuses`);
//! - `gsd/W525`: each prohibition without a `test` of a complete plan;
//! - `gsd/W311`, `W313`, `W315`, `W316`, the plan checks: each plan that is not complete;
//! - `gsd/E518`, `gsd/E202`, `gsd/W203`, `gsd/W205`: each milestone audit, phase spec, locked requirement and package
//!   audit, owed from its creation;
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
pub const W525: &str = "gsd/W525";
pub const E518: &str = "gsd/E518";
pub const E202: &str = "gsd/E202";
pub const W203: &str = "gsd/W203";
pub const W205: &str = "gsd/W205";
/// The plan checks, owed while a plan is open (no complete summary).
pub const PLAN_CHECKS: [&str; 4] = ["gsd/W311", "gsd/W313", "gsd/W315", "gsd/W316"];
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
    let plan_done = |r: &Record| text(r, "plan").is_some_and(|p| complete.contains(&p));
    let mut out: Vec<Owed> = kept
        .into_iter()
        .flat_map(|r| {
            let codes: &[&'static str] = match r.kind {
                "truth" | "artifact" if plan_done(r) => &[E501],
                "key_link" if plan_done(r) && !r.fields.contains_key("pattern") => &[E501],
                "prohibition" if plan_done(r) && !r.fields.contains_key("test") => &[W525],
                "plan" if !complete.contains(&r.id) => &PLAN_CHECKS,
                "milestone_audit" => &[E518],
                "phase_spec" => &[E202],
                "locked_req" => &[W203],
                "package_audit" => &[W205],
                "threat" if text(r, "status").as_deref() == Some("closed") => &[W302],
                "security_audit" => &[W303],
                _ => &[],
            };
            let phase = r
                .file
                .strip_prefix(&format!("{}/phases/", paths::ROOT))
                .and_then(|f| f.strip_suffix(".spec"))
                .map(str::to_string);
            codes.iter().map(move |&code| Owed {
                phase: phase.clone(),
                code,
                entity: r.id.clone(),
            })
        })
        .collect();
    out.sort();
    out
}

/// The slash command that records a verdict of `code`: the workflow that runs its judge.
fn command(code: &str) -> &'static str {
    match code {
        E501 | W525 => "/gsd-verify-work",
        E518 => "/gsd-audit-milestone",
        E202 | W203 => "/gsd-spec-phase",
        W302 | W303 => "/gsd-secure-phase",
        // The plan checks and the package audit: the plan checker and the phase researcher.
        _ => "/gsd-plan-phase",
    }
}

/// The report's owed lines for `owed`: per phase and slash command that records its verdicts ([`command`]), then per
/// entity the query that shows what its judge reads, then the count; `check` exits 1 while an error rule is owed.
pub fn lines(owed: &[Owed]) -> Vec<String> {
    let mut sorted: Vec<(&str, &Owed)> = owed.iter().map(|o| (command(o.code), o)).collect();
    sorted.sort_by(|(a, x), (b, y)| (&x.phase, a, *x).cmp(&(&y.phase, b, *y)));
    let mut out = Vec::new();
    let mut group = None;
    for (command, o) in sorted {
        if group != Some((&o.phase, command)) {
            group = Some((&o.phase, command));
            out.push(match &o.phase {
                Some(phase) => format!("{command} {phase}"),
                None if command == "/gsd-verify-work" => {
                    "quick plans (no phase): ask the verifier to judge each".to_string()
                }
                // `/gsd-plan-phase` names a phase, and none plans a quick plan.
                None if command == "/gsd-plan-phase" => {
                    "quick plans (no phase): ask the plan checker to judge each".to_string()
                }
                // A milestone audit: its command names no phase.
                None => command.to_string(),
            });
        }
        out.push(format!(
            "  specforge query {} --inputs {} --format json",
            shell_quote(&o.entity),
            o.code
        ));
    }
    if !owed.is_empty() {
        let check = if owed.iter().any(|o| {
            o.code
                .rsplit('/')
                .next()
                .is_some_and(|c| c.starts_with('E'))
        }) {
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
            links: Vec::new(),
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

    /// Core's other judged rules: the plan checks on an open plan, W525 on a complete plan's untested prohibition, and
    /// the rules owed from creation.
    #[test]
    fn owed_mirrors_cores_other_judged_rules() {
        let phase = "spec/gsd/phases/01.spec";
        let plan = |id: &str| record("plan", id, phase, json!({}));
        let records = [
            record(
                "summary",
                "v1.0/01/01-01/01-01-SUMMARY",
                phase,
                json!({"plan": "v1.0/01/01-01", "status": "complete"}),
            ),
            plan("v1.0/01/01-01"),
            plan("v1.0/01/01-02"),
            record(
                "prohibition",
                "v1.0/01/01-01/P1",
                phase,
                json!({"plan": "v1.0/01/01-01"}),
            ),
            record(
                "prohibition",
                "v1.0/01/01-01/P2",
                phase,
                json!({"plan": "v1.0/01/01-01", "test": "t"}),
            ),
            record(
                "prohibition",
                "v1.0/01/01-02/P1",
                phase,
                json!({"plan": "v1.0/01/01-02"}),
            ),
            record("phase_spec", "v1.0/01/01-SPEC", phase, json!({})),
            record("package_audit", "v1.0/01/01-RESEARCH/PA1", phase, json!({})),
            record(
                "milestone_audit",
                "v1.0-MILESTONE-AUDIT",
                "spec/gsd/milestones.spec",
                json!({}),
            ),
        ];
        let owed = owed(&records);
        let found: Vec<(&str, &str)> = owed.iter().map(|o| (o.code, o.entity.as_str())).collect();
        assert_eq!(
            found,
            [
                (E518, "v1.0-MILESTONE-AUDIT"),
                (E202, "v1.0/01/01-SPEC"),
                (W205, "v1.0/01/01-RESEARCH/PA1"),
                ("gsd/W311", "v1.0/01/01-02"),
                ("gsd/W313", "v1.0/01/01-02"),
                ("gsd/W315", "v1.0/01/01-02"),
                ("gsd/W316", "v1.0/01/01-02"),
                (W525, "v1.0/01/01-01/P1"),
            ]
        );
        let lines = lines(&owed);
        let headers: Vec<&str> = lines
            .iter()
            .filter(|l| !l.starts_with(' '))
            .map(String::as_str)
            .collect();
        assert_eq!(
            headers,
            [
                "/gsd-audit-milestone",
                "/gsd-plan-phase 01",
                "/gsd-spec-phase 01",
                "/gsd-verify-work 01",
                "8 verdicts owed: `specforge check` exits 1 until these verdicts are recorded",
            ]
        );
    }

    #[test]
    fn an_open_quick_plans_checks_name_no_bare_plan_phase() {
        let records = [record(
            "plan",
            "261105-tcl",
            "spec/gsd/quick.spec",
            json!({}),
        )];
        let lines = lines(&owed(&records));
        assert_eq!(
            lines[0],
            "quick plans (no phase): ask the plan checker to judge each"
        );
        assert!(!lines.iter().any(|l| l == "/gsd-plan-phase"), "{lines:#?}");
    }
}

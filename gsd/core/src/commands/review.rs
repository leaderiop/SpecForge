//! The commands of a cross-AI review (`spec/plan.md`, "Reviews and revision conflicts"): `record_review` writes a round
//! and its concerns in one plan, from the JSON the review workflow assembles. Every value is checked before anything
//! is written: a plan of another phase, a reviewer the round lacks, a concern with no reviewer is refused, naming it.
//! `record_conflict` and `resolve_conflict` raise and close a revision conflict.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, next_id, node, now, plain, required, run, target, title};

/// The keys of a round's JSON; `--round` takes only the last two.
const ROUND_KEYS: &[&str] = &[
    "plans",
    "reviewers",
    "lanes",
    "coverage",
    "consensus",
    "strengths",
    "divergent",
    "unchecked",
    "concerns",
];
const MORE_KEYS: &[&str] = &["unchecked", "concerns"];
const RISKS: &[&str] = &["low", "medium", "high"];
const SEVERITIES: &[&str] = &["high", "medium", "low"];

/// The JSON of `key` in `object` read as one shape, every refusal naming where it stands (`concerns[1].text`).
struct Reader<'a> {
    object: &'a Map<String, Value>,
    at: String,
}

impl<'a> Reader<'a> {
    fn new(value: &'a Value, at: String) -> Result<Self, String> {
        match value.as_object() {
            Some(object) => Ok(Reader { object, at }),
            None => Err(format!("{at}: not a JSON object")),
        }
    }

    fn path(&self, key: &str) -> String {
        if self.at.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.at)
        }
    }

    /// Refuses a key that is not one of `keys`.
    fn only(&self, keys: &[&str]) -> Result<(), String> {
        match self.object.keys().find(|k| !keys.contains(&k.as_str())) {
            Some(key) => Err(format!("{}: no key '{key}'", self.path(key))),
            None => Ok(()),
        }
    }

    /// A text as a person wrote it; `None` when absent or `null`.
    fn text(&self, key: &str) -> Result<Option<&'a str>, String> {
        match self.object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => {
                plain(&self.path(key), text)?;
                Ok(Some(text))
            }
            Some(_) => Err(format!("{}: not a string", self.path(key))),
        }
    }

    /// A text that must be there and not blank.
    fn needed(&self, key: &str) -> Result<&'a str, String> {
        match self.text(key)? {
            Some(text) if !text.trim().is_empty() => Ok(text),
            _ => Err(format!("{}: required", self.path(key))),
        }
    }

    /// One of `values`, when given.
    fn word(&self, key: &str, values: &[&str]) -> Result<Option<&'a str>, String> {
        match self.text(key)? {
            Some(word) if !values.contains(&word) => Err(format!(
                "{}: '{word}' is not one of {}",
                self.path(key),
                values.join(", ")
            )),
            other => Ok(other),
        }
    }

    fn texts(&self, key: &str) -> Result<Vec<&'a str>, String> {
        self.list(key)?
            .iter()
            .enumerate()
            .map(|(i, item)| match item.as_str() {
                Some(text) => {
                    plain(&format!("{}[{i}]", self.path(key)), text)?;
                    Ok(text)
                }
                None => Err(format!("{}[{i}]: not a string", self.path(key))),
            })
            .collect()
    }

    fn bool(&self, key: &str) -> Result<Option<bool>, String> {
        match self.object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(format!("{}: not true or false", self.path(key))),
        }
    }

    fn count(&self, key: &str) -> Result<Option<u64>, String> {
        match self.object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("{}: not a count", self.path(key))),
        }
    }

    fn list(&self, key: &str) -> Result<&'a [Value], String> {
        match self.object.get(key) {
            None | Some(Value::Null) => Ok(&[]),
            Some(Value::Array(items)) => Ok(items),
            Some(_) => Err(format!("{}: not a JSON list", self.path(key))),
        }
    }

    /// Each object of the list `key`, as a reader.
    fn rows(&self, key: &str) -> Result<Vec<Reader<'a>>, String> {
        let at = self.path(key);
        self.list(key)?
            .iter()
            .enumerate()
            .map(|(i, row)| Reader::new(row, format!("{at}[{i}]")))
            .collect()
    }
}

/// `value` as JSON, or `null` (left out of a `create`) when it is empty.
fn kept(value: Value) -> Value {
    match &value {
        Value::Array(items) if items.is_empty() => Value::Null,
        Value::String(text) if text.is_empty() => Value::Null,
        _ => value,
    }
}

/// The plans of `phase`, by id.
fn plans_of<'a>(call: &'a CommandCall<'_>, phase: &str) -> BTreeSet<&'a str> {
    let graph = call.graph();
    graph
        .nodes_of_kind("plan")
        .filter(|p| target(graph, &p.id, "phase") == Some(phase))
        .map(|p| p.id.as_str())
        .collect()
}

/// `reviewer` names one of `names`, else the refusal names it and where it stands.
fn known(names: &BTreeSet<&str>, at: &str, reviewer: &str) -> Result<(), String> {
    if names.contains(reviewer) {
        Ok(())
    } else {
        Err(format!("{at}: '{reviewer}' is no reviewer of the round"))
    }
}

/// A concern to create: its heading and its fields (without `review`).
type Concern = (String, Map<String, Value>);

/// The concerns of a round, checked, as `create` fields (without `review`).
fn concerns(
    call: &CommandCall<'_>,
    doc: &Reader<'_>,
    plans: &BTreeSet<&str>,
    names: &BTreeSet<&str>,
) -> Result<Vec<Concern>, String> {
    let mut out = Vec::new();
    for row in doc.rows("concerns")? {
        row.only(&[
            "plan",
            "reviewers",
            "severity",
            "text",
            "actionable",
            "drift",
            "evidence",
        ])?;
        let plan = match row.text("plan")? {
            Some(id) => {
                let plan = node(call, "plan", id)?;
                if !plans.contains(plan.id.as_str()) {
                    return Err(format!(
                        "{}: plan '{id}' is not one the round reviewed",
                        row.path("plan")
                    ));
                }
                json!(plan.id)
            }
            None => Value::Null,
        };
        let reviewers = row.texts("reviewers")?;
        for reviewer in &reviewers {
            known(names, &row.path("reviewers"), reviewer)?;
        }
        let drift = row.bool("drift")?;
        if reviewers.is_empty() && drift != Some(true) {
            return Err(format!(
                "{}: a concern names its reviewers (only a drift finding has none)",
                row.path("reviewers")
            ));
        }
        let severity = row
            .word("severity", SEVERITIES)?
            .ok_or_else(|| format!("{}: required", row.path("severity")))?;
        let text = row.needed("text")?;
        let created = fields([
            ("plan", plan),
            ("reviewers", kept(json!(reviewers))),
            ("severity", json!(severity)),
            ("text", json!(text)),
            (
                "actionable",
                row.bool("actionable")?.map_or(Value::Null, Value::from),
            ),
            ("drift", drift.map_or(Value::Null, Value::from)),
            ("evidence", kept(json!(row.texts("evidence")?))),
        ]);
        out.push((title(text), created.into_iter().collect()));
    }
    Ok(out)
}

/// The `unchecked` rows of a round, checked.
fn unchecked(doc: &Reader<'_>) -> Result<Vec<Value>, String> {
    doc.rows("unchecked")?
        .iter()
        .map(|row| {
            row.only(&["symbol", "why"])?;
            Ok(json!({ "symbol": row.needed("symbol")?, "why": row.needed("why")? }))
        })
        .collect()
}

/// `record_review <phase> --review <json> [--round <review>]`: a round with its concerns, or, with `--round`, more
/// unchecked symbols and concerns of a round already recorded.
fn record_review(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = node(call, "phase", required(call, "phase")?)?;
    let raw = required(call, "review")?;
    let value: Value =
        serde_json::from_str(raw).map_err(|e| format!("review: not the JSON it takes: {e}"))?;
    let doc =
        Reader::new(&value, String::new()).map_err(|_| "review: not a JSON object".to_string())?;
    let round = call
        .str("round")
        .map(|r| node(call, "review", r))
        .transpose()?;
    let graph = call.graph();
    let mut edit = call.plan();
    let (review, file, plans, names, mut wrote) = match round {
        Some(review) => {
            if let Some(key) = doc.object.keys().find(|k| !MORE_KEYS.contains(&k.as_str())) {
                return Err(format!(
                    "--round takes only unchecked and concerns, not '{key}'"
                ));
            }
            if target(graph, &review.id, "phase") != Some(phase.id.as_str()) {
                return Err(format!(
                    "review '{}' is not a round of phase '{}'",
                    review.id, phase.id
                ));
            }
            let plans: BTreeSet<&str> = graph
                .edges_from(&review.id)
                .into_iter()
                .filter(|e| e.label == "plans")
                .map(|e| e.target.as_str())
                .collect();
            let names: BTreeSet<&str> = review
                .fields
                .get("reviewers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|r| r.get("name")?.as_str())
                .collect();
            for row in unchecked(&doc)? {
                edit.append(&review.id, "unchecked", row);
            }
            let file = review
                .file
                .clone()
                .unwrap_or_else(|| "spec/plans.spec".to_string());
            (review.id.clone(), file, plans, names, Vec::new())
        }
        None => {
            doc.only(ROUND_KEYS)?;
            let ours = plans_of(call, &phase.id);
            let mut plans = BTreeSet::new();
            for id in doc.texts("plans")? {
                let plan = node(call, "plan", id)?;
                if !ours.contains(plan.id.as_str()) {
                    return Err(format!(
                        "plans: '{id}' is not a plan of phase '{}'",
                        phase.id
                    ));
                }
                plans.insert(plan.id.as_str());
            }
            if plans.is_empty() {
                return Err("plans: a round reviews at least one plan".to_string());
            }
            let mut names = BTreeSet::new();
            let mut reviewers = Vec::new();
            for row in doc.rows("reviewers")? {
                row.only(&["name", "model", "source"])?;
                let name = row.needed("name")?;
                if !names.insert(name) {
                    return Err(format!("{}: '{name}' is named twice", row.path("name")));
                }
                reviewers.push(json!({
                    "name": name,
                    "model": row.needed("model")?,
                    "source": row.needed("source")?,
                }));
            }
            if reviewers.is_empty() {
                return Err("reviewers: a round has at least one reviewer".to_string());
            }
            let mut lanes = Vec::new();
            for row in doc.rows("lanes")? {
                row.only(&["reviewer", "summary", "strengths", "suggestions", "risk"])?;
                let reviewer = row.needed("reviewer")?;
                known(&names, &row.path("reviewer"), reviewer)?;
                let mut lane = json!({ "reviewer": reviewer, "summary": row.needed("summary")? });
                for key in ["strengths", "suggestions"] {
                    let items = row.texts(key)?;
                    if !items.is_empty() {
                        lane[key] = json!(items);
                    }
                }
                if let Some(risk) = row.word("risk", RISKS)? {
                    lane["risk"] = json!(risk);
                }
                lanes.push(lane);
            }
            let mut coverage = Vec::new();
            for row in doc.rows("coverage")? {
                row.only(&["reviewer", "total", "missing"])?;
                let reviewer = row.needed("reviewer")?;
                known(&names, &row.path("reviewer"), reviewer)?;
                let mut covered = json!({ "reviewer": reviewer });
                if let Some(total) = row.count("total")? {
                    covered["total"] = json!(total);
                }
                let missing = row.texts("missing")?;
                if !missing.is_empty() {
                    covered["missing"] = json!(missing);
                }
                coverage.push(covered);
            }
            let reviewed_at = now(call)?;
            let name = next_id(graph, "review", &format!("{}/", phase.id), "RV");
            let id = format!("{}/{name}", phase.id);
            let file = phase
                .file
                .clone()
                .unwrap_or_else(|| "spec/plans.spec".to_string());
            edit.create(
                "review",
                &name,
                &format!("Review round {name} of {}", phase.id),
                &file,
                fields([
                    ("phase", json!(phase.id)),
                    ("plans", json!(plans)),
                    ("reviewers", json!(reviewers)),
                    ("lanes", kept(json!(lanes))),
                    ("coverage", kept(json!(coverage))),
                    (
                        "consensus",
                        kept(json!(doc.text("consensus")?.unwrap_or_default())),
                    ),
                    ("strengths", kept(json!(doc.texts("strengths")?))),
                    ("divergent", kept(json!(doc.texts("divergent")?))),
                    ("unchecked", kept(json!(unchecked(&doc)?))),
                    ("reviewed_at", json!(reviewed_at)),
                ]),
                Vec::new(),
            );
            (id.clone(), file, plans, names, vec![id])
        }
    };
    let first = next_id(graph, "review_concern", &format!("{review}/"), "RC");
    let number: u64 = first[2..].parse().unwrap_or(1);
    let mut created = Vec::new();
    for (n, (heading, mut values)) in (number..).zip(concerns(call, &doc, &plans, &names)?) {
        let name = format!("RC{n}");
        values.insert("review".to_string(), json!(review));
        edit.create(
            "review_concern",
            &name,
            &heading,
            &file,
            values.into_iter().collect(),
            Vec::new(),
        );
        created.push(format!("{review}/{name}"));
    }
    if round.is_some() && created.is_empty() && doc.list("unchecked")?.is_empty() {
        return Err("--round adds nothing: give unchecked or concerns".to_string());
    }
    if round.is_some() {
        wrote.push(review.clone());
    }
    wrote.extend(created.iter().cloned());
    Ok(answer(
        call,
        json!({ "review": review, "concerns": created, "wrote": wrote }),
        format!("review {review}: {} concern(s)\n", created.len()),
        Some(&edit),
    ))
}

/// `text` with every run of line breaks and tabs collapsed to one space, trimmed (upstream's sanitize rule: one
/// conflict is one line).
fn one_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut gap = false;
    for c in text.chars() {
        if matches!(c, '\n' | '\r' | '\t') {
            gap = true;
            continue;
        }
        if gap {
            out.push(' ');
            gap = false;
        }
        out.push(c);
    }
    out.trim().to_string()
}

/// The arg `arg` on one line ([`one_line`]); refused when it holds a control character or nothing is left.
fn line_arg(call: &CommandCall<'_>, arg: &str, needed: bool) -> Result<Option<String>, String> {
    let Some(raw) = call.str(arg) else {
        return if needed {
            Err(format!("'{arg}' is required"))
        } else {
            Ok(None)
        };
    };
    let line = one_line(raw);
    plain(arg, &line)?;
    match line.is_empty() {
        true if needed => Err(format!("'{arg}' is empty")),
        true => Ok(None),
        false => Ok(Some(line)),
    }
}

/// Whether a conflict is open: a command raised it and none resolved it.
fn open(conflict: &GraphNode) -> bool {
    conflict.has_field("raised") && !conflict.has_field("resolved")
}

/// `record_conflict <plan> --dimension --property --conflicts-with [--alternatives]`: an open revision conflict in the
/// plan's phase, or the open one that already says the same.
fn record_conflict(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let plan = node(call, "plan", required(call, "plan")?)?;
    let graph = call.graph();
    let Some(phase) = target(graph, &plan.id, "phase") else {
        return Err(format!(
            "plan '{}' is a quick plan: quick has no conflict channel",
            plan.id
        ));
    };
    let dimension = line_arg(call, "dimension", true)?.unwrap_or_default();
    let property = line_arg(call, "property", true)?.unwrap_or_default();
    let conflicts_with = line_arg(call, "conflicts_with", true)?.unwrap_or_default();
    let alternatives = line_arg(call, "alternatives", false)?;
    let same = graph.nodes_of_kind("revision_conflict").find(|c| {
        open(c)
            && target(graph, &c.id, "plan") == Some(plan.id.as_str())
            && c.text("dimension") == Some(dimension.as_str())
            && c.text("property") == Some(property.as_str())
            && c.text("conflicts_with") == Some(conflicts_with.as_str())
    });
    if let Some(conflict) = same {
        return Ok(answer(
            call,
            json!({ "conflict": conflict.id, "reused": true, "wrote": [] }),
            format!("revision conflict {} is already open\n", conflict.id),
            None,
        ));
    }
    let raised = now(call)?;
    let name = next_id(graph, "revision_conflict", &format!("{phase}/"), "CF");
    let id = format!("{phase}/{name}");
    let file = graph
        .node(phase)
        .and_then(|p| p.file.clone())
        .unwrap_or_else(|| "spec/plans.spec".to_string());
    let mut edit = call.plan();
    edit.create(
        "revision_conflict",
        &name,
        &title(&format!("{dimension}/{}: {property}", plan.id)),
        &file,
        fields([
            ("phase", json!(phase)),
            ("plan", json!(plan.id)),
            ("dimension", json!(dimension)),
            ("property", json!(property)),
            ("conflicts_with", json!(conflicts_with)),
            (
                "alternatives",
                alternatives.map_or(Value::Null, Value::from),
            ),
            ("raised", json!(raised)),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "conflict": id, "reused": false, "wrote": [id] }),
        format!("revision conflict {id} raised at {raised}\n"),
        Some(&edit),
    ))
}

/// `resolve_conflict <how> (--conflict <id> | --plan <plan>)`: one open conflict, or every open one of a plan.
fn resolve_conflict(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let how = required(call, "how")?;
    let graph = call.graph();
    let targets: Vec<&GraphNode> = match (call.str("conflict"), call.str("plan")) {
        (Some(_), Some(_)) | (None, None) => {
            return Err("give one of --conflict and --plan".to_string());
        }
        (Some(id), None) => {
            let conflict = node(call, "revision_conflict", id)?;
            if conflict.has_field("resolved") {
                return Err(format!(
                    "revision conflict '{}' is already resolved",
                    conflict.id
                ));
            }
            vec![conflict]
        }
        (None, Some(id)) => {
            let plan = node(call, "plan", id)?;
            graph
                .nodes_of_kind("revision_conflict")
                .filter(|c| open(c) && target(graph, &c.id, "plan") == Some(plan.id.as_str()))
                .collect()
        }
    };
    let at = now(call)?;
    let mut edit = call.plan();
    let ids: Vec<&str> = targets.iter().map(|c| c.id.as_str()).collect();
    for id in &ids {
        edit.set(id, "resolved", json!({ "at": at, "how": how }));
    }
    Ok(answer(
        call,
        json!({ "resolved": ids, "wrote": ids }),
        format!("{} revision conflict(s) resolved at {at}\n", ids.len()),
        (!ids.is_empty()).then_some(&edit),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("record_review", |cmd| {
        cmd.description("Record a cross-AI review round of a phase's plans (GSD NN-REVIEWS.md): its reviewers, lanes, coverage and consensus, and one concern each; with --round, more unchecked symbols and concerns of a round")
            .writes()
            .arg("phase", |a| {
                a.required().description("the phase whose plans were reviewed");
            })
            .arg("review", |a| {
                a.description("required; the round as JSON: plans, reviewers [{name, model, source}], lanes [{reviewer, summary, strengths, suggestions, risk}], coverage [{reviewer, total, missing}], consensus, strengths, divergent, unchecked [{symbol, why}], concerns [{plan, reviewers, severity, text, actionable, drift, evidence}]");
            })
            .arg("round", |a| {
                a.description("a round already recorded: the JSON then holds only unchecked and concerns, which are added to it");
            })
            .handler(|call| run(call, record_review));
    });
    c.command("record_conflict", |cmd| {
        cmd.description("Record a conflict a plan revision reported (GSD REVISION_CONFLICT): open in the plan's phase; the same open conflict twice is one entity, answered reused")
            .writes()
            .arg("plan", |a| {
                a.required().description("the plan whose revision reported it");
            })
            .arg("dimension", |a| {
                a.description("required; the checker dimension or review lane that asked (one line)");
            })
            .arg("property", |a| {
                a.description("required; what the ask would change (one line)");
            })
            .arg("conflicts_with", |a| {
                a.description("required; the decision, requirement or must-have it would break (one line)");
            })
            .arg("alternatives", |a| {
                a.description("the ways out the planner named (one line)");
            })
            .handler(|call| run(call, record_conflict));
    });
    c.command("resolve_conflict", |cmd| {
        cmd.description("Resolve a revision conflict: one with --conflict, or every open one of a plan with --plan")
            .writes()
            .arg("how", |a| {
                a.required().description("how the replan resolved it");
            })
            .arg("conflict", |a| {
                a.description("the open conflict to resolve");
            })
            .arg("plan", |a| {
                a.description("the plan whose open conflicts all resolve");
            })
            .handler(|call| run(call, resolve_conflict));
    });
}

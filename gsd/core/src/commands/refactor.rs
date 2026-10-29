//! The refactor trigger's commands (D-r41-15, D-r41-1 rows 14-15). `propose_refactor` scores the files the phase's
//! plans modified with the ported complexity scorer and records a proposal for the top function that crosses the
//! threshold or jumped; `dispose_refactor` records what was decided. The baseline of a function is the score of the
//! latest *disposed* proposal for it: the anchor never advances on a plain evaluation, only a disposition moves it.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::complexity::{self, Analyzed, Candidate};
use super::support::{answer, fields, next_id, node, now, ops_file, required, run, text, title};

const OUTCOMES: &[&str] = &["accepted", "declined"];

/// The phase's plans' modified files that are served and scored, each once.
fn served<'a>(call: &'a CommandCall<'_>, plans: &BTreeSet<&str>) -> Vec<(&'a str, &'a str)> {
    let Some(files) = call.files("files") else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    files
        .facts
        .iter()
        .filter(|f| f.entity.as_deref().is_some_and(|e| plans.contains(e)))
        .filter(|f| complexity::is_analyzable_path(&f.path))
        .filter_map(|f| Some((f.path.as_str(), f.text.as_deref()?)))
        .filter(|(path, _)| seen.insert(*path))
        .collect()
}

/// The score of each `file::function` at the latest disposed proposal for it.
fn baselines(call: &CommandCall<'_>) -> BTreeMap<String, i64> {
    let mut latest: BTreeMap<String, (&str, i64)> = BTreeMap::new();
    for proposal in call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "refactor_proposal")
    {
        let (Some(file), Some(function), Some(at), Some(score)) = (
            proposal.text("target_file"),
            proposal.text("target_function"),
            proposal
                .fields
                .get("disposition")
                .and_then(|d| d["at"].as_str()),
            proposal.fields.get("score").and_then(Value::as_i64),
        ) else {
            continue;
        };
        let entry = latest
            .entry(format!("{file}::{function}"))
            .or_insert((at, score));
        if at > entry.0 {
            *entry = (at, score);
        }
    }
    latest
        .into_iter()
        .map(|(k, (_, score))| (k, score))
        .collect()
}

fn candidate_row(c: &Candidate) -> Value {
    // Integers go as decimal text: the host reads a block's integer member from text.
    let mut row = json!({
        "file": c.file,
        "function": c.name,
        "line": c.start_line.to_string(),
        "score": c.score.to_string(),
    });
    if let (Some(baseline), Some(delta)) = (c.baseline, c.delta) {
        row["baseline"] = json!(baseline.to_string());
        row["delta"] = json!(delta.to_string());
    }
    row
}

/// `propose_refactor <phase>`.
fn propose(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = node(call, "phase", required(call, "phase")?)?;
    let plans: BTreeSet<&str> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "plan" && n.text("phase") == Some(phase.id.as_str()))
        .map(|n| n.id.as_str())
        .collect();
    let settings = Settings::current();
    let threshold = settings
        .integer("refactor.complexity_threshold")
        .unwrap_or(complexity::THRESHOLD);
    let jump = settings
        .integer("refactor.complexity_jump_delta")
        .unwrap_or(complexity::JUMP_DELTA);
    let strict = settings.bool("refactor.trigger_strict").unwrap_or(false);

    let files = served(call, &plans);
    let analyzed: Vec<Analyzed> = files
        .iter()
        .map(|(path, source)| Analyzed {
            file: (*path).to_string(),
            functions: complexity::analyze_source(source),
        })
        .collect();
    let skipped: Vec<&str> = files
        .iter()
        .zip(&analyzed)
        .filter(|(_, a)| a.functions.is_err())
        .map(|((path, _), _)| *path)
        .collect();
    // One open proposal per function: while one is untriaged the function is frozen (upstream's anchor freeze), so a
    // second run, or the hook of r44, records neither another proposal nor, under strict, another window.
    let untriaged: BTreeMap<String, &str> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "refactor_proposal" && !n.has_field("disposition"))
        .filter_map(|n| {
            Some((
                format!("{}::{}", n.text("target_file")?, n.text("target_function")?),
                n.id.as_str(),
            ))
        })
        .collect();
    let mut candidates = complexity::evaluate(&analyzed, &baselines(call), threshold, jump);
    let frozen: Vec<String> = candidates
        .iter()
        .filter_map(|c| {
            untriaged
                .get(&format!("{}::{}", c.file, c.name))
                .map(|id| format!("{id} ({} in {})", c.name, c.file))
        })
        .collect();
    candidates.retain(|c| !untriaged.contains_key(&format!("{}::{}", c.file, c.name)));
    let Some(target) = candidates.first() else {
        return Ok(answer(
            call,
            json!({ "wrote": [], "files": files.len(), "skipped": skipped, "candidates": [], "untriaged": frozen }),
            if frozen.is_empty() {
                format!(
                    "nothing crosses: {} file(s) scored in phase {}, threshold {threshold}, jump {jump}\n",
                    files.len(),
                    phase.id
                )
            } else {
                format!(
                    "nothing new: untriaged proposal(s) {}: dispose them first\n",
                    frozen.join(", ")
                )
            },
            None,
        ));
    };

    let at = now(call)?;
    let name = next_id(
        call.graph(),
        "refactor_proposal",
        &format!("{}/", phase.id),
        "RP",
    );
    let id = format!("{}/{name}", phase.id);
    let (score, baseline) = (target.score, target.baseline.unwrap_or(target.score));
    let reasons: Vec<&str> = target.reasons.clone();
    let file = ops_file(call.graph(), Some(&phase.id));
    let mut plan = call.plan();
    plan.create(
        "refactor_proposal",
        &name,
        &title(&format!("Refactor {} in {}", target.name, target.file)),
        &file,
        fields([
            ("phase", json!(phase.id)),
            ("target_file", json!(target.file)),
            ("target_function", json!(target.name)),
            ("score", json!(score.to_string())),
            ("baseline", json!(baseline.to_string())),
            ("delta", json!(target.delta.unwrap_or(0).to_string())),
            ("reasons", json!(reasons)),
            (
                "candidates",
                Value::Array(candidates.iter().map(candidate_row).collect()),
            ),
        ]),
        Vec::new(),
    );
    let mut wrote = vec![id.clone()];
    if strict {
        let window = next_id(call.graph(), "broken_window", "", "BW");
        plan.create(
            "broken_window",
            &window,
            &title(&format!(
                "Refactor proposal {name}: {} in {}",
                target.name, target.file
            )),
            &file,
            fields([
                ("kind", json!("deviation")),
                ("phase", json!(phase.id)),
                (
                    "description",
                    json!(format!(
                        "Refactor proposal {name} is untriaged: {} in {} scores {score}",
                        target.name, target.file
                    )),
                ),
                ("file", json!(target.file)),
                ("line", json!(target.start_line.to_string())),
                ("refactor", json!(id)),
                ("recorded_at", json!(at)),
            ]),
            Vec::new(),
        );
        wrote.push(window);
    }
    Ok(answer(
        call,
        json!({ "wrote": wrote, "target": { "file": target.file, "function": target.name, "score": score }, "skipped": skipped }),
        format!(
            "refactor proposal {id}: {} in {} scores {score} ({})\n",
            target.name,
            target.file,
            reasons.join(", ")
        ),
        Some(&plan),
    ))
}

/// `dispose_refactor <proposal> <outcome> <by> [--reason]`.
fn dispose(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let proposal = node(call, "refactor_proposal", required(call, "proposal")?)?;
    let outcome = required(call, "outcome")?;
    let by = required(call, "by")?;
    let reason = text(call, "reason")?.filter(|r| !r.trim().is_empty());
    // `disposition` is `H`, not `I`: the host would allow a second write, so the command does not.
    if proposal.has_field("disposition") {
        return Err(format!(
            "refactor proposal '{}' is already disposed",
            proposal.id
        ));
    }
    if outcome == "declined" && reason.is_none() {
        return Err("a declined proposal needs --reason".to_string());
    }
    let at = now(call)?;
    let mut value = json!({ "outcome": outcome, "by": by, "at": at });
    if let Some(reason) = reason {
        value["reason"] = json!(reason);
    }
    let mut plan = call.plan();
    plan.set(&proposal.id, "disposition", value.clone());
    Ok(answer(
        call,
        json!({ "wrote": [proposal.id], "disposition": value }),
        format!(
            "refactor proposal {} {outcome} by {by} at {at}\n",
            proposal.id
        ),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("propose_refactor", |cmd| {
        cmd.description("Score the files a phase's plans modified with upstream's decision-point scorer and record a refactor proposal for the top function over the threshold or up by more than the jump delta; records nothing when none crosses")
            .writes()
            .reads("files", FileReadSpec::field("plan.files_modified").text())
            .arg("phase", |a| {
                a.required().description("the phase whose plans' files are scored");
            })
            .handler(|call| run(call, propose));
    });
    c.command("dispose_refactor", |cmd| {
        cmd.description("Record what was decided on a refactor proposal: accepted or declined (a declined one needs a reason); a proposal is disposed once")
            .writes()
            .arg("proposal", |a| {
                a.required().description("the refactor proposal (its path, or a short form that is not ambiguous)");
            })
            .arg("outcome", |a| {
                a.required().one_of(OUTCOMES).description("accepted or declined");
            })
            .arg("by", |a| {
                a.required().description("who decided");
            })
            .arg("reason", |a| {
                a.description("why, in the person's words (required when declined)");
            })
            .handler(|call| run(call, dispose));
    });
}

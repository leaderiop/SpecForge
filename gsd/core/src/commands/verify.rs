//! The commands of verification (D-r41-1 rows 5-6): a person's answer to a UAT test, the approval of a validation, and
//! an automated UAT result an agent or CI records. A person's UAT result is their own: the command writes what they
//! said, never a sanitised copy.
//!
//! Also ports gsd-core bin/lib modules: verify.cjs.
//!
//! gsd-core modules this file ports: verification.cjs.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run, target, text};

const RESULTS: &[&str] = &["pending", "pass", "issue", "skipped", "blocked"];
const SEVERITIES: &[&str] = &["blocker", "major", "minor", "cosmetic"];
const BLOCKED_BY: &[&str] = &[
    "server",
    "physical_device",
    "release_build",
    "third_party",
    "prior_phase",
];

/// `set_uat_result <uat_test> <result> --by <who> [--source] [--reported] [--severity] [--reason | --follow-up]
/// [--blocked-by]`.
fn set_uat_result(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    if call.str("source") == Some("automated") {
        return Err("--source automated is refused here: a person's answer is never automated. An agent or CI records an automated result with \
             `specforge gsd record-automated-uat <uat_test> <pass|issue> --by <who>`"
            .to_string());
    }
    write_result(call, false)
}

/// `record_automated_uat <uat_test> <pass|issue> --by <who> [--reported] [--severity]`: a result a test run produced,
/// which upstream's agent records without asking the person (verify-work.md, `auto_passed`). Agent-callable.
fn record_automated_uat(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    write_result(call, true)
}

/// The write both commands share; `automated` marks the result `source: automated` and takes no reason or blocker.
fn write_result(call: &CommandCall<'_>, automated: bool) -> Result<CommandOutput, String> {
    let test = node(call, "uat_test", required(call, "uat_test")?)?;
    let result = required(call, "result")?;
    // The caller's attestation of whose answer this is. No field holds it (a uat_test has no `by`): it is refused
    // empty and echoed, never stored (the command is human-attested: ADR 0301).
    let by = required(call, "by")?;
    let source = automated.then_some("automated");
    // What the predicate (gsd/E509) asks of a result: the words, the reason, the blocker, here or already there.
    let given = |arg: &str| text(call, arg);
    let kept = |arg: &str| test.has_field(arg);
    let reported = given("reported")?;
    let severity = call.str("severity");
    let (mut reason, blocked_by) = if automated {
        (None, None)
    } else {
        (given("reason")?.map(str::to_string), call.str("blocked_by"))
    };
    // A deferred follow-up is a skip whose reason is upstream's text, and an entry on the session.
    let follow_up = if automated { None } else { given("follow_up")? };
    if let Some(idea) = follow_up {
        if result != "skipped" {
            return Err(format!(
                "--follow-up belongs to a 'skipped' result, not '{result}'"
            ));
        }
        if reason.is_some() {
            return Err(
                "--follow-up writes the reason itself: give it without --reason".to_string(),
            );
        }
        if idea.trim().is_empty() {
            return Err("'follow_up' is empty".to_string());
        }
        reason = Some(format!("Deferred follow-up: {idea}"));
    }
    for (needed, result_of, name, have) in [
        (
            "issue",
            result,
            "reported",
            reported.is_some() || kept("reported"),
        ),
        (
            "skipped",
            result,
            "reason",
            reason.is_some() || kept("reason"),
        ),
        (
            "blocked",
            result,
            "blocked_by",
            blocked_by.is_some() || kept("blocked_by"),
        ),
    ] {
        if result_of == needed && !have {
            return Err(format!(
                "a '{needed}' result needs --{}",
                name.replace('_', "-")
            ));
        }
    }
    let id = &test.id;
    let mut plan = call.plan();
    plan.set(id, "result", result);
    // A re-answer by a person does not keep an earlier automated `source`.
    if let Some(source) = source.or(test.has_field("source").then_some("human")) {
        plan.set(id, "source", source);
    }
    // A re-answer takes only the members its result has; the others of an earlier answer are unset.
    for (name, value, results) in [
        ("reported", reported, "issue"),
        ("severity", severity, "issue"),
        ("reason", reason.as_deref(), "skipped"),
        ("blocked_by", blocked_by, "blocked"),
    ] {
        match value {
            Some(_) if result != results => {
                return Err(format!(
                    "--{} belongs to a '{results}' result, not '{result}'",
                    name.replace('_', "-")
                ));
            }
            Some(value) => {
                plan.set(id, name, value);
            }
            None if result != results && test.has_field(name) => {
                plan.unset(id, name);
            }
            None => {}
        }
    }
    let mut wrote = vec![id.clone()];
    if let Some(idea) = follow_up {
        let session = target(call.graph(), id, "session")
            .ok_or_else(|| format!("uat test '{id}' names no session"))?;
        let held = call
            .graph()
            .node(session)
            .and_then(|s| s.fields.get("follow_ups"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|entries| {
                entries.iter().any(|e| {
                    e.get("idea").and_then(serde_json::Value::as_str) == Some(idea)
                        && e.get("test")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|t| t == id || id.ends_with(&format!("/{t}")))
                })
            });
        if !held {
            plan.append(
                session,
                "follow_ups",
                json!({ "test": id, "idea": idea, "deferred_at": call.today() }),
            );
            wrote.push(session.to_string());
        }
    }
    Ok(answer(
        call,
        json!({ "wrote": wrote, "result": result }),
        format!("uat test {id}: {result} (told by {by})\n"),
        Some(&plan),
    ))
}

/// `approve_validation <validation> <by>`: the person who approves a phase's validation, with the host's clock.
fn approve_validation(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let validation = node(call, "validation", required(call, "validation")?)?;
    let by = required(call, "by")?;
    if validation.has_field("approval") {
        return Err(format!(
            "validation '{}' is already approved",
            validation.id
        ));
    }
    let at = now(call)?;
    let mut plan = call.plan();
    plan.set(&validation.id, "approval", json!({ "by": by, "at": at }));
    Ok(answer(
        call,
        json!({ "wrote": [validation.id], "approval": { "by": by, "at": at } }),
        format!("validation {} approved by {by} at {at}\n", validation.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("set_uat_result", |cmd| {
        cmd.description("Record a person's answer to a UAT test: pass, issue (with their words), skipped (with a reason), blocked (with what blocks it) or pending")
            .human()
            .arg("uat_test", |a| {
                a.required().description("the UAT test (its path, or a short form that is not ambiguous)");
            })
            .arg("result", |a| {
                a.required().one_of(RESULTS).description("the answer");
            })
            .arg("by", |a| {
                a.description("required; whose answer it is (typed by the person at their terminal; echoed, not stored)");
            })
            .arg("source", |a| {
                a.one_of(&["human", "automated"]).description("a person's answer is `human`; `automated` is refused here (use record_automated_uat)");
            })
            .arg("reported", |a| {
                a.description("the person's words, kept verbatim (required for an issue)");
            })
            .arg("severity", |a| {
                a.one_of(SEVERITIES).description("how bad the issue is (major when absent)");
            })
            .arg("reason", |a| {
                a.description("why the test was skipped (required for a skipped, unless --follow-up)");
            })
            .arg("follow_up", |a| {
                a.description("a skipped test's deferred follow-up, in the person's words: the reason becomes `Deferred follow-up: <words>` and the session keeps the idea");
            })
            .arg("blocked_by", |a| {
                a.one_of(BLOCKED_BY).description("what blocks the test (required for a blocked)");
            })
            .handler(|call| run(call, set_uat_result));
    });
    c.command("record_automated_uat", |cmd| {
        cmd.description("Record an automated UAT result (a pass or an issue) that a test run produced, without asking the person")
            .writes()
            .arg("uat_test", |a| {
                a.required().description("the UAT test (its path, or a short form that is not ambiguous)");
            })
            .arg("result", |a| {
                a.required().one_of(&["pass", "issue"]).description("the automated result");
            })
            .arg("by", |a| {
                a.description("required; what produced it (for example ci; echoed, not stored)");
            })
            .arg("reported", |a| {
                a.description("what the run showed, kept verbatim (required for an issue)");
            })
            .arg("severity", |a| {
                a.one_of(SEVERITIES).description("how bad the issue is (major when absent)");
            })
            .handler(|call| run(call, record_automated_uat));
    });
    c.command("approve_validation", |cmd| {
        cmd.description("Record who approved a phase's validation, with the host's clock")
            .human()
            .arg("validation", |a| {
                a.required().description(
                    "the validation (its path, or a short form that is not ambiguous)",
                );
            })
            .arg("by", |a| {
                a.required().description("who approves it");
            })
            .handler(|call| run(call, approve_validation));
    });
}

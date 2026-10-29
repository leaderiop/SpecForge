//! The commands of verification (D-r41-1 rows 5-6): a person's answer to a UAT test, and the approval of a
//! validation. A UAT result is the person's own: the command writes what they said, never a sanitised copy.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run, text};

const RESULTS: &[&str] = &["pending", "pass", "issue", "skipped", "blocked"];
const SEVERITIES: &[&str] = &["blocker", "major", "minor", "cosmetic"];
const BLOCKED_BY: &[&str] = &[
    "server",
    "physical_device",
    "release_build",
    "third_party",
    "prior_phase",
];

/// `set_uat_result <uat_test> <result> --by <who> [--source] [--reported] [--severity] [--reason] [--blocked-by]`.
fn set_uat_result(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let test = node(call, "uat_test", required(call, "uat_test")?)?;
    let result = required(call, "result")?;
    // The caller's attestation of whose answer this is. No field holds it (a uat_test has no `by`): it is refused
    // empty and echoed, never stored (K19: a human-attested command kind).
    let by = required(call, "by")?;
    let source = call.str("source");
    // What the predicate (gsd/E509) asks of a result: the words, the reason, the blocker, here or already there.
    let given = |arg: &str| text(call, arg);
    let kept = |arg: &str| test.has_field(arg);
    let (reported, reason) = (given("reported")?, given("reason")?);
    let (severity, blocked_by) = (call.str("severity"), call.str("blocked_by"));
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
    if source == Some("automated") && !["pass", "issue"].contains(&result) {
        return Err(format!(
            "--source automated is for a pass or an issue, not '{result}'"
        ));
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
        ("reason", reason, "skipped"),
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
    Ok(answer(
        call,
        json!({ "wrote": [id], "result": result }),
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
            .writes()
            .arg("uat_test", |a| {
                a.required().description("the UAT test (its path, or a short form that is not ambiguous)");
            })
            .arg("result", |a| {
                a.required().one_of(RESULTS).description("the answer");
            })
            .arg("by", |a| {
                a.description("required; whose answer it is: the caller's attestation, echoed and not stored (K19)");
            })
            .arg("source", |a| {
                a.one_of(&["human", "automated"]).description("who produced the result; only given for an automated pass or issue");
            })
            .arg("reported", |a| {
                a.description("the person's words, kept verbatim (required for an issue)");
            })
            .arg("severity", |a| {
                a.one_of(SEVERITIES).description("how bad the issue is (major when absent)");
            })
            .arg("reason", |a| {
                a.description("why the test was skipped (required for a skipped)");
            })
            .arg("blocked_by", |a| {
                a.one_of(BLOCKED_BY).description("what blocks the test (required for a blocked)");
            })
            .handler(|call| run(call, set_uat_result));
    });
    c.command("approve_validation", |cmd| {
        cmd.description("Record who approved a phase's validation, with the host's clock")
            .writes()
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

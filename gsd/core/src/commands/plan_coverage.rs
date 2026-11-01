//! `plan_coverage <phase> --review <text>`: whether a reviewer's answer names every plan of the phase, the check
//! `/gsd-review` runs on each lane's report before it trusts it.
//!
//! A port of review.md's plan-coverage script (`gsd-core/workflows/review.md`, gsd-core v1.19.0, MIT, `LICENSE-GSD`):
//! a plan counts when its id (the last segment, `01-02`) stands alone in the text, with no word character and no `-`
//! on either side (upstream's `(?<![\w-])<escaped id>(?![\w-])`, where `\w` is ASCII), so `T-04-07` does not cover
//! `04-07` and `12X6-01` does not cover `12.6-01`. The plans are the phase's own (those naming it in `phase`), not a
//! manifest file: upstream read the list its earlier block wrote, and that list is now this same selection.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};

/// Whether `c` joins an id to its neighbour: `\w` (ASCII) or `-`.
fn joins(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Whether `id` stands alone somewhere in `text`.
fn named(text: &str, id: &str) -> bool {
    text.match_indices(id).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(joins)
            && !text[at + id.len()..].chars().next().is_some_and(joins)
    })
}

/// The coverage of `ids` by `review`: `{complete, missing_ids, total}`, the missing ones in `ids`' order.
pub fn coverage(ids: &[&str], review: &str) -> Value {
    let missing: Vec<&str> = ids
        .iter()
        .copied()
        .filter(|id| !named(review, id))
        .collect();
    json!({ "complete": missing.is_empty(), "missing_ids": missing, "total": ids.len() })
}

fn plan_coverage(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = call.str("phase").ok_or("plan_coverage needs a phase")?;
    // shortcut: the text comes as an argument (a command reads no runtime path), so a review over the platform's
    // argument size (128 KiB on Linux) cannot be passed; read the lane's file through the host when one exceeds it
    let review = call
        .str("review")
        .ok_or("plan_coverage needs --review, the reviewer's text")?;
    let graph = call.graph();
    if !graph.nodes_of_kind("phase").any(|p| p.id == phase) {
        return Err(format!("no phase '{phase}'"));
    }
    let ids: Vec<&str> = graph
        .nodes_of_kind("plan")
        .filter(|p| p.text("phase") == Some(phase))
        .map(|p| p.id.rsplit('/').next().unwrap_or(&p.id))
        .collect();
    let report = coverage(&ids, review);
    let human = match report["missing_ids"].as_array() {
        Some(missing) if !missing.is_empty() => format!(
            "the review does not name: {}\n",
            missing
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!("the review names every plan ({})\n", ids.len()),
    };
    Ok(answer(call, report, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("plan_coverage", |cmd| {
        cmd.description("Check that a reviewer's text names every plan of a phase, each id standing alone (no word character or `-` beside it); answers complete, missing_ids and total")
            .arg("phase", |a| {
                a.required().description("the phase whose plans the review must cover");
            })
            .arg("review", |a| {
                a.description("required; the reviewer's text");
            })
            .handler(|call| run(call, plan_coverage));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_counts_only_where_it_stands_alone() {
        // the shell port's cases: an escaped `.`, `+`, `(`, `[`, and a longer id beside a shorter one
        let ids = ["12.6-07", "01-02", "a.b+c", "x(y)", "01-1", "[z]"];
        let review =
            "Reviewed 01-02, a.b+c and x(y); 01-10 is not 01-1, and [z]\n12.6-070 is not it\n";
        assert_eq!(
            coverage(&ids, review),
            json!({"complete": false, "missing_ids": ["12.6-07"], "total": 6})
        );
    }

    #[test]
    fn a_neighbour_that_joins_hides_the_id() {
        for review in [
            "T-04-07", "04-07x", "x04-07", "_04-07", "04-07-1", "12X6-01",
        ] {
            assert!(!named(review, "04-07") || review == "12X6-01", "{review}");
        }
        assert!(!named("12X6-01", "12.6-01"));
        // a later occurrence that stands alone counts, and so do punctuation, a path and non-ASCII neighbours
        assert!(named("T-04-07, then 04-07.", "04-07"));
        assert!(named("plans/04-07/PLAN.md", "04-07"));
        assert!(named("é04-07é", "04-07"));
    }

    #[test]
    fn no_plans_is_complete() {
        assert_eq!(
            coverage(&[], ""),
            json!({"complete": true, "missing_ids": [], "total": 0})
        );
    }
}

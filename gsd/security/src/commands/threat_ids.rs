//! `gsd.security/E101` and `threat_ids <phase>`: a threat ID claimed by more than one plan of its phase, as `init
//! execute-phase` and `init plan-phase` answer `threat_id_duplicates` (upstream #4683, `src/init.cts:960-1001`).
//!
//! The grammar is upstream's (`src/plan-document.cts:476-483`): `T-<digits>[.<digits>]-<digits>`, the whole ID. The
//! `-SC` supply-chain row every plan keeps, a lowercase `t-`, and `T-47-05A` never claim an ID. A superseded plan is
//! removed from the model rather than kept with a status, so none is excluded (`[no-superseded-plans]`).

use std::collections::BTreeMap;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, name_of, node, required, run};

/// Whether `id` is a register row ID that claims a number: `T-47-01`, `T-4.1-02`.
pub fn claims(id: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let Some(rest) = id.strip_prefix("T-") else {
        return false;
    };
    let Some((phase, number)) = rest.rsplit_once('-') else {
        return false;
    };
    let phase_ok = match phase.split_once('.') {
        Some((whole, part)) => digits(whole) && digits(part),
        None => digits(phase),
    };
    phase_ok && digits(number)
}

/// One threat that claims an ID: `(phase, id, plan)`, each by name.
pub type Claim<'a> = (&'a str, &'a str, &'a str);

/// The IDs claimed by more than one plan of one phase: `(phase, id, plans)`, sorted by phase then ID, plans sorted.
pub fn duplicates<'a>(
    claimed: impl IntoIterator<Item = Claim<'a>>,
) -> Vec<(&'a str, &'a str, Vec<&'a str>)> {
    let mut by_id: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for (phase, id, plan) in claimed {
        let plans = by_id.entry((phase, id)).or_default();
        if !plans.contains(&plan) {
            plans.push(plan);
        }
    }
    by_id
        .into_iter()
        .filter(|(_, plans)| plans.len() > 1)
        .map(|((phase, id), mut plans)| {
            plans.sort_unstable();
            (phase, id, plans)
        })
        .collect()
}

/// The target of the edge `label` out of `source`.
fn target<'a>(graph: &'a CommandGraph, source: &str, label: &str) -> Option<&'a str> {
    graph
        .edges_from(source)
        .into_iter()
        .find(|e| e.label == label)
        .map(|e| e.target.as_str())
}

/// `threat_ids <phase>`.
fn threat_ids(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = node(call, "phase", required(call, "phase")?)?;
    let graph = call.graph();
    let claimed = graph.nodes_of_kind("threat").filter_map(|t| {
        let plan = target(graph, &t.id, "plan")?;
        (target(graph, plan, "phase") == Some(phase.id.as_str()) && claims(name_of(&t.id)))
            .then(|| (name_of(&phase.id), name_of(&t.id), name_of(plan)))
    });
    let found = duplicates(claimed);
    let rows: Vec<_> = found
        .iter()
        .map(|(_, id, plans)| json!({ "id": id, "plans": plans }))
        .collect();
    let human = if found.is_empty() {
        format!(
            "phase {}: no threat ID is claimed twice\n",
            name_of(&phase.id)
        )
    } else {
        found.iter().fold(String::new(), |text, (_, id, plans)| {
            format!("{text}{id} is claimed by {}\n", plans.join(", "))
        })
    };
    Ok(answer(
        call,
        json!({ "threat_id_duplicates": rows, "threat_id_duplicate_count": rows.len() }),
        human,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("threat_ids", |cmd| {
        cmd.description(
            "The threat IDs that more than one plan of the phase claims, as init's threat_id_duplicates",
        )
        .arg("phase", |a| {
            a.required().description("the phase (its id, or its number)");
        })
        .handler(|call| run(call, threat_ids));
    });
}

#[cfg(test)]
mod tests {
    use super::{claims, duplicates};

    #[test]
    fn only_digit_suffixed_ids_claim() {
        for yes in ["T-47-01", "T-4.1-02", "T-1-1"] {
            assert!(claims(yes), "{yes}");
        }
        for no in [
            "T-47-SC", "t-47-01", "T-47-05A", "T--01", "T-47-", "T-4.-01", "X-47-01", "T-47",
        ] {
            assert!(!claims(no), "{no}");
        }
    }

    #[test]
    fn a_duplicate_needs_two_plans_of_one_phase() {
        let found = duplicates([
            ("01", "T-01-02", "01-02"),
            ("01", "T-01-02", "01-01"),
            ("01", "T-01-01", "01-01"),
            ("01", "T-01-01", "01-01"),
            ("02", "T-01-02", "02-01"),
        ]);
        assert_eq!(found, [("01", "T-01-02", vec!["01-01", "01-02"])]);
    }
}

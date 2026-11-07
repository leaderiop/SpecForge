//! The branch a phase or a milestone executes on, from `git.branching_strategy`, its template and one slug.
//!
//! D-s1-07-6: a computed field, not a command answer (`gsd branch-name <id>`): every reader already queries the phase,
//! and the value follows the settings with no call to keep in step. This is the first pass to read the settings.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::branch::{fill, slug};
use crate::passes::commits::target;
use crate::passes::plans::{name_of, of_kind, text};

/// The settings a branch name is made from.
struct Naming<'a> {
    strategy: &'a str,
    phase: &'a str,
    milestone: &'a str,
}

/// The phase strategy's name of `phase`: its local name and its slug, else the slug of its name.
fn phase_branch(naming: &Naming<'_>, phase: &PassEntity) -> String {
    let slugged = text(phase, "slug")
        .map(str::to_string)
        .unwrap_or_else(|| slug(text(phase, "name").unwrap_or_default()));
    fill(
        naming.phase,
        &[("phase", name_of(&phase.id)), ("slug", &slugged)],
    )
}

fn milestone_branch(naming: &Naming<'_>, milestone: &PassEntity) -> String {
    fill(
        naming.milestone,
        &[
            ("milestone", name_of(&milestone.id)),
            ("slug", &slug(text(milestone, "name").unwrap_or_default())),
        ],
    )
}

fn named(input: &PassInput, naming: &Naming<'_>) -> Vec<PassValue> {
    let is_backlog = |phase: &PassEntity| name_of(&phase.id).starts_with("999.");
    match naming.strategy {
        "phase" => of_kind(input, "phase")
            .filter(|phase| !is_backlog(phase))
            .map(|phase| value(&phase.id, "branch", phase_branch(naming, phase)))
            .collect(),
        "milestone" => {
            let by_milestone: BTreeMap<&str, String> = of_kind(input, "milestone")
                .map(|milestone| (milestone.id.as_str(), milestone_branch(naming, milestone)))
                .collect();
            let phases = of_kind(input, "phase")
                .filter(|phase| !is_backlog(phase))
                .filter_map(|phase| {
                    let branch = by_milestone.get(target(input, &phase.id, "milestone")?)?;
                    Some(value(&phase.id, "branch", branch.clone()))
                });
            by_milestone
                .iter()
                .map(|(id, branch)| value(id, "branch", branch.clone()))
                .chain(phases)
                .collect()
        }
        _ => Vec::new(),
    }
}

pub fn values(input: &PassInput) -> Vec<PassValue> {
    let settings = Settings::current();
    let naming = Naming {
        strategy: settings
            .string("git.branching_strategy")
            .unwrap_or_default(),
        phase: settings
            .string("git.phase_branch_template")
            .unwrap_or_default(),
        milestone: settings
            .string("git.milestone_branch_template")
            .unwrap_or_default(),
    };
    named(input, &naming)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMING: Naming<'static> = Naming {
        strategy: "phase",
        phase: "gsd/phase-{phase}-{slug}",
        milestone: "gsd/{milestone}-{slug}",
    };

    fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
        PassEntity {
            id: id.into(),
            kind: kind.into(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ..Default::default()
        }
    }

    fn input() -> PassInput {
        PassInput {
            entities: vec![
                entity("v1.0", "milestone", &[("name", "Task Board!")]),
                entity(
                    "v1.0/01",
                    "phase",
                    &[("name", "Core"), ("slug", "core-tasks")],
                ),
                entity("v1.0/02", "phase", &[("name", "User Auth & Login")]),
                entity("v1.0/999.1", "phase", &[("name", "Later")]),
            ],
            edges: ["v1.0/01", "v1.0/02", "v1.0/999.1"]
                .iter()
                .map(|phase| PassEdge {
                    source: phase.to_string(),
                    target: "v1.0".into(),
                    label: "milestone".into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn pairs(values: Vec<PassValue>) -> BTreeMap<String, String> {
        values
            .into_iter()
            .inspect(|v| assert_eq!(v.field, "branch"))
            .map(|v| (v.entity, v.value.as_str().unwrap().to_string()))
            .collect()
    }

    #[test]
    fn the_phase_strategy_names_each_phase() {
        let named = pairs(named(&input(), &NAMING));
        assert_eq!(named["v1.0/01"], "gsd/phase-01-core-tasks");
        assert_eq!(named["v1.0/02"], "gsd/phase-02-user-auth-login");
        assert_eq!(named.len(), 2, "no milestone, no backlog phase: {named:?}");
    }

    #[test]
    fn the_milestone_strategy_names_the_milestone_and_its_phases() {
        let naming = Naming {
            strategy: "milestone",
            ..NAMING
        };
        let named = pairs(named(&input(), &naming));
        for id in ["v1.0", "v1.0/01", "v1.0/02"] {
            assert_eq!(named[id], "gsd/v1.0-task-board", "{id}");
        }
        assert_eq!(named.len(), 3, "no backlog phase: {named:?}");
    }

    #[test]
    fn a_backlog_phase_or_no_strategy_has_none() {
        for strategy in ["none", ""] {
            let naming = Naming { strategy, ..NAMING };
            assert!(named(&input(), &naming).is_empty(), "{strategy:?}");
        }
        let backlog = PassInput {
            entities: vec![entity("v1.0/999.1", "phase", &[("name", "Later")])],
            ..Default::default()
        };
        assert!(named(&backlog, &NAMING).is_empty());
    }
}

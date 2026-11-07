//! The branch a phase or a milestone executes on, from `git.branching_strategy`, its template and one slug.
//!
//! D-s1-07-6: a computed field, not a command answer (`gsd branch-name <id>`): every reader already queries the phase,
//! and the value follows the settings with no call to keep in step. This is the first pass to read the settings.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::branch::{fill, refused, shortened, slug};
use crate::passes::commits::target;
use crate::passes::plans::{name_of, of_kind, text};

/// The settings a branch name is made from.
struct Naming<'a> {
    strategy: &'a str,
    phase: &'a str,
    milestone: &'a str,
}

/// `template` with its keys and the slug filled; a slug it needs and has none of, or a name git refuses, is the why.
fn checked(template: &str, key: (&str, &str), slug: &str) -> Result<String, String> {
    if slug.is_empty() && template.contains("{slug}") {
        return Err(format!("`{template}` needs a slug and its name makes none"));
    }
    let name = fill(template, &[key, ("slug", shortened(slug))]);
    match refused(&name) {
        Some(why) => Err(format!("git refuses the name `{name}`: {why}")),
        None => Ok(name),
    }
}

/// The phase strategy's name of `phase`: its local name and its slug, else the slug of its name.
fn phase_branch(naming: &Naming<'_>, phase: &PassEntity) -> Result<String, String> {
    let slugged = text(phase, "slug")
        .map(str::to_string)
        .unwrap_or_else(|| slug(text(phase, "name").unwrap_or_default()));
    checked(naming.phase, ("phase", name_of(&phase.id)), &slugged)
}

fn milestone_branch(naming: &Naming<'_>, milestone: &PassEntity) -> Result<String, String> {
    checked(
        naming.milestone,
        ("milestone", name_of(&milestone.id)),
        &slug(text(milestone, "name").unwrap_or_default()),
    )
}

/// Each entity's name, or why it has none. Under the milestone strategy a phase shares its milestone's, and the
/// milestone alone says why.
fn names(input: &PassInput, naming: &Naming<'_>) -> Vec<(String, Result<String, String>)> {
    let is_backlog = |phase: &PassEntity| name_of(&phase.id).starts_with("999.");
    match naming.strategy {
        "phase" => of_kind(input, "phase")
            .filter(|phase| !is_backlog(phase))
            .map(|phase| (phase.id.clone(), phase_branch(naming, phase)))
            .collect(),
        "milestone" => {
            let by_milestone: BTreeMap<&str, Result<String, String>> = of_kind(input, "milestone")
                .map(|milestone| (milestone.id.as_str(), milestone_branch(naming, milestone)))
                .collect();
            let phases = of_kind(input, "phase")
                .filter(|phase| !is_backlog(phase))
                .filter_map(|phase| {
                    let branch = by_milestone.get(target(input, &phase.id, "milestone")?)?;
                    Some((phase.id.clone(), Ok(branch.as_ref().ok()?.clone())))
                });
            by_milestone
                .iter()
                .map(|(id, branch)| (id.to_string(), branch.clone()))
                .chain(phases)
                .collect()
        }
        _ => Vec::new(),
    }
}

fn named(input: &PassInput, naming: &Naming<'_>) -> Vec<PassValue> {
    names(input, naming)
        .into_iter()
        .filter_map(|(id, name)| Some(value(&id, "branch", name.ok()?)))
        .collect()
}

fn refusals(input: &PassInput, naming: &Naming<'_>) -> Vec<PassDiagnostic> {
    names(input, naming)
        .into_iter()
        .filter_map(|(id, name)| {
            let why = name.err()?;
            Some(
                PassDiagnostic::new(
                    "W409",
                    PassSeverity::Warning,
                    format!("'{id}' has no branch: {why}"),
                )
                .with_suggestion(
                    "change its name or slug, or git.phase_branch_template or git.milestone_branch_template",
                )
                .with_entity(&id),
            )
        })
        .collect()
}

fn settings_naming(settings: &Settings) -> Naming<'_> {
    Naming {
        strategy: settings
            .string("git.branching_strategy")
            .unwrap_or_default(),
        phase: settings
            .string("git.phase_branch_template")
            .unwrap_or_default(),
        milestone: settings
            .string("git.milestone_branch_template")
            .unwrap_or_default(),
    }
}

/// The computed field, for each entity whose name git takes.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    named(input, &settings_naming(&Settings::current()))
}

/// `gsd/W409`, for each entity whose name it refuses.
pub fn diagnostics(input: &PassInput) -> Vec<PassDiagnostic> {
    refusals(input, &settings_naming(&Settings::current()))
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
    fn a_name_git_refuses_leaves_the_field_absent_with_a_diagnostic() {
        let naming = Naming {
            phase: "-{phase}-{slug}",
            ..NAMING
        };
        assert!(named(&input(), &naming).is_empty());
        let said = refusals(&input(), &naming);
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said.iter().all(|d| d.code == "W409"), "{said:?}");
        assert!(said[0].message.contains("-01-core-tasks"), "{said:?}");
    }

    #[test]
    fn an_empty_slug_names_no_branch() {
        let empty = PassInput {
            entities: vec![entity("v1.0/01", "phase", &[("name", "!!!")])],
            ..Default::default()
        };
        assert!(named(&empty, &NAMING).is_empty());
        let said = refusals(&empty, &NAMING);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].message.contains("slug"), "{said:?}");
        let no_slug = Naming {
            phase: "gsd/phase-{phase}",
            ..NAMING
        };
        assert_eq!(pairs(named(&empty, &no_slug))["v1.0/01"], "gsd/phase-01");
        assert!(refusals(&empty, &no_slug).is_empty());
    }

    #[test]
    fn a_long_title_shortens() {
        let title = "word ".repeat(80);
        let long = PassInput {
            entities: vec![entity("v1.0/01", "phase", &[("name", title.as_str())])],
            ..Default::default()
        };
        let named = pairs(named(&long, &NAMING));
        let name = &named["v1.0/01"];
        assert!(name.len() < 90, "{name}");
        assert!(name.starts_with("gsd/phase-01-word-word"), "{name}");
        assert!(!name.ends_with('-'), "{name}");
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

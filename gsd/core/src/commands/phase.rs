//! `phase` (D-r41-8): the roadmap's phase CRUD behind `/gsd-phase`. The action is chosen by flag, as upstream's
//! `[--insert X|--remove N|--edit N]`: none adds, `--insert` makes a decimal phase, `--remove` deletes a phase and
//! renumbers the later ones in one plan, `--edit` sets fields.

mod remove;

use std::cmp::Ordering;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, json as json_arg, required, run, text, title};

const TO_BE_PLANNED: &str = "[To be planned]";
const URGENT: &str = "[Urgent work - to be planned]";

/// A phase name: `05`, `05.1` or `999.1`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Number {
    pub whole: u32,
    pub decimal: Option<u32>,
    /// The digits `whole` was written with: `05` stays two wide.
    pub width: usize,
}

impl Number {
    pub fn parse(name: &str) -> Option<Number> {
        let (whole, decimal) = match name.split_once('.') {
            Some((w, d)) => (w, Some(d.parse().ok()?)),
            None => (name, None),
        };
        Some(Number {
            whole: whole.parse().ok()?,
            decimal,
            width: whole.len(),
        })
    }

    pub fn backlog(self) -> bool {
        self.whole == 999
    }

    pub fn name(self) -> String {
        let whole = format!("{:0width$}", self.whole, width = self.width);
        match self.decimal {
            Some(d) => format!("{whole}.{d}"),
            None => whole,
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.whole, self.decimal).cmp(&(other.whole, other.decimal))
    }
}

/// A phase of the milestone being edited.
pub(super) struct Phase<'a> {
    pub node: &'a GraphNode,
    pub number: Number,
}

/// The milestone named by `--milestone`, else the only one that is neither closed nor abandoned.
fn milestone<'a>(call: &CommandCall<'a>) -> Result<&'a GraphNode, String> {
    if let Some(name) = text(call, "milestone")? {
        return super::support::node(call, "milestone", name);
    }
    let open: Vec<&GraphNode> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| {
            n.kind == "milestone" && !n.has_field("completion") && !n.has_field("abandoned")
        })
        .collect();
    match open.as_slice() {
        [one] => Ok(one),
        [] => Err("the project has no open milestone: name the milestone with --milestone".into()),
        many => Err(format!(
            "name the milestone with --milestone: {}",
            many.iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The milestone's phases, in order.
pub(super) fn phases<'a>(call: &CommandCall<'a>, milestone: &GraphNode) -> Vec<Phase<'a>> {
    let mut found: Vec<Phase<'a>> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "phase" && n.text("milestone") == Some(&milestone.id))
        .filter_map(|node| {
            let name = node.id.rsplit('/').next()?;
            Some(Phase {
                node,
                number: Number::parse(name)?,
            })
        })
        .collect();
    found.sort_by_key(|p| p.number);
    found
}

/// The phase `name` names (`05`, `05.1`) among `phases`; a backlog phase is never a target.
pub(super) fn target<'a, 'p>(phases: &'p [Phase<'a>], name: &str) -> Result<&'p Phase<'a>, String> {
    let number = Number::parse(name).ok_or_else(|| format!("'{name}' is not a phase number"))?;
    if number.backlog() {
        return Err(format!(
            "phase '{name}' is a backlog phase: /gsd-phase leaves 999.x alone"
        ));
    }
    phases
        .iter()
        .find(|p| p.number == number)
        .ok_or_else(|| format!("no phase '{name}' in this milestone"))
}

/// `--name` as the directory slug: lower case, runs of other characters as one dash.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

fn list(call: &CommandCall<'_>, arg: &str) -> Result<Option<Vec<String>>, String> {
    match call.str(arg) {
        None => Ok(None),
        Some(_) => json_arg(call, arg, Vec::new(), |t| serde_json::from_str(t)).map(Some),
    }
}

/// Creates the phase `number` after `previous`, in the milestone's own file.
fn create(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    number: Number,
    previous: Option<&str>,
    goal_default: &str,
) -> Result<CommandOutput, String> {
    let name = required(call, "name")?;
    let goal = text(call, "goal")?.unwrap_or(goal_default);
    let criteria = list(call, "criteria")?.unwrap_or_else(|| vec![TO_BE_PLANNED.to_string()]);
    let id = format!("{}/{}", ms.id, number.name());
    let mut plan = call.plan();
    plan.create(
        "phase",
        &number.name(),
        &title(name),
        ms.file.as_deref().unwrap_or_default(),
        fields([
            ("milestone", json!(ms.id)),
            ("name", json!(name)),
            ("slug", json!(slug(name))),
            ("goal", json!(goal)),
            ("success_criteria", json!(criteria)),
            ("depends_on", previous.map_or(Value::Null, |p| json!([p]))),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [id], "phase": number.name() }),
        format!("phase {} \"{name}\" added to {}\n", number.name(), ms.id),
        Some(&plan),
    ))
}

fn add(call: &CommandCall<'_>, ms: &GraphNode, all: &[Phase<'_>]) -> Result<CommandOutput, String> {
    let last = all
        .iter()
        .filter(|p| !p.number.backlog() && p.number.decimal.is_none())
        .max_by_key(|p| p.number);
    let number = Number {
        whole: last.map_or(1, |p| p.number.whole + 1),
        decimal: None,
        width: last.map_or(2, |p| p.number.width),
    };
    create(
        call,
        ms,
        number,
        last.map(|p| p.node.id.as_str()),
        TO_BE_PLANNED,
    )
}

fn insert(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    after: &str,
) -> Result<CommandOutput, String> {
    let after = target(all, after)?;
    let taken = |k: u32| {
        all.iter()
            .any(|p| p.number.whole == after.number.whole && p.number.decimal == Some(k))
    };
    let k = (1..).find(|k| !taken(*k)).unwrap_or(1);
    let number = Number {
        decimal: Some(k),
        ..after.number
    };
    create(call, ms, number, Some(&after.node.id), URGENT)
}

fn edit(
    call: &CommandCall<'_>,
    ms: &GraphNode,
    all: &[Phase<'_>],
    name: &str,
) -> Result<CommandOutput, String> {
    let phase = target(all, name)?;
    let mut plan = call.plan();
    let mut set = Vec::new();
    if let Some(v) = text(call, "name")? {
        plan.set(&phase.node.id, "name", v);
        set.push("name");
    }
    if let Some(v) = text(call, "goal")? {
        plan.set(&phase.node.id, "goal", v);
        set.push("goal");
    }
    if let Some(v) = list(call, "criteria")? {
        plan.set(&phase.node.id, "success_criteria", json!(v));
        set.push("success_criteria");
    }
    if let Some(names) = list(call, "depends_on")? {
        let ids = names
            .iter()
            .map(|n| target(all, n).map(|p| p.node.id.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        plan.set(&phase.node.id, "depends_on", json!(ids));
        set.push("depends_on");
    }
    if set.is_empty() {
        return Err("--edit sets nothing: give --name, --goal, --criteria or --depends-on".into());
    }
    Ok(answer(
        call,
        json!({ "wrote": [phase.node.id], "set": set }),
        format!(
            "phase {} of {}: set {}\n",
            phase.number.name(),
            ms.id,
            set.join(", ")
        ),
        Some(&plan),
    ))
}

fn phase(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let ms = milestone(call)?;
    let all = phases(call, ms);
    let chosen: Vec<&str> = ["insert", "remove", "edit"]
        .into_iter()
        .filter(|a| call.str(a).is_some())
        .collect();
    match chosen.as_slice() {
        [] => add(call, ms, &all),
        ["insert"] => insert(call, ms, &all, required(call, "insert")?),
        ["remove"] => remove::remove(call, ms, &all, required(call, "remove")?),
        ["edit"] => edit(call, ms, &all, required(call, "edit")?),
        many => Err(format!("one action at a time: {}", many.join(", "))),
    }
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("phase", |cmd| {
        cmd.description("Add, insert, remove or edit a phase of the roadmap; a removal renumbers the later phases in one edit plan")
            .writes()
            .arg("milestone", |a| {
                a.description("the milestone (default: the only one still open)");
            })
            .arg("insert", |a| {
                a.description("insert a decimal phase after this phase (02 makes 02.1, or the first free decimal)");
            })
            .arg("remove", |a| {
                a.description("remove this phase that has not started, and renumber the later ones");
            })
            .arg("edit", |a| {
                a.description("edit this phase: --name, --goal, --criteria, --depends-on");
            })
            .arg("name", |a| {
                a.description("the phase's name (required to add or insert)");
            })
            .arg("goal", |a| {
                a.description("the phase's goal");
            })
            .arg("criteria", |a| {
                a.description("the success criteria, as a JSON list of strings");
            })
            .arg("depends_on", |a| {
                a.description("what it depends on, as a JSON list of phase numbers (--edit)");
            })
            .arg("force", |a| {
                a.flag().description("remove a phase that has plans with summaries or commits");
            })
            .handler(|call| run(call, phase));
    });
}

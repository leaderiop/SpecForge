//! Planning a list of steps (ADR 0289): typed edits and renames, in order, each over the texts the ones before it
//! left. A run of edits is planned over one project; a rename compiles the texts so far, plans K2's rename over
//! that compile and merges its texts, and the edits after it are planned over a compile of the merged texts.

use super::declared::Declared;
use super::{Edit, EditField, EditPlan, EditReport, Planner, describe, validate};
use crate::Refusal;
use crate::authoring::format::{Configs, Forms};
use crate::authoring::planned::PlannedChange;
use crate::authoring::rename::{self, RenameField};
use crate::read::navigate::Navigator;
use crate::view::ProjectView;
use specforge_common::DiagnosticList;
use specforge_parser::LineBreak;
use specforge_project::harness::{Harness, LedgerChange};
use std::collections::BTreeMap;

/// One step of a plan: a typed edit, or a rename of an entity (its declaration and every reference).
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Edit(Edit),
    Rename {
        entity: String,
        to: String,
    },
    /// A judged rule's verdict, checked by the apply step: the entity's block gains or replaces its verdict member.
    Verdict(Box<crate::authoring::apply::VerdictStep>),
}

impl Step {
    fn describe(&self) -> String {
        match self {
            Step::Edit(edit) => describe(edit),
            Step::Rename { entity, to } => format!("rename {entity} {to}"),
            Step::Verdict(v) => format!("set_verdict {} {}", v.entity, v.member.rule),
        }
    }
}

/// What the steps so far have made.
#[derive(Default)]
struct Carry {
    /// The text of each file the steps changed, LF.
    texts: BTreeMap<String, String>,
    declared: Declared,
    /// The entities moved so far, composed.
    moves: Vec<(String, String)>,
    /// Every move, as it was made.
    raw_moves: Vec<(String, String)>,
    /// The harness values set: (moves made before, path then, field).
    sets: Vec<(usize, String, String)>,
}

/// A move `old → new` after the moves of `moves`: a chain composes, `a → b` then `b → c` is `a → c`.
pub(super) fn compose(moves: &mut Vec<(String, String)>, old: &str, new: &str) {
    match moves.iter_mut().find(|(_, to)| to == old) {
        Some((_, to)) => *to = new.to_string(),
        None => moves.push((old.to_string(), new.to_string())),
    }
}

/// `path` once `moves`, made in order, are made.
fn relocated(path: &str, moves: &[(String, String)]) -> String {
    let mut path = path.to_string();
    for (old, new) in moves {
        if path == *old {
            path = new.clone();
        } else if let Some(rest) = path.strip_prefix(&format!("{old}/")) {
            path = format!("{new}/{rest}");
        }
    }
    path
}

/// Plan `edits`, in order, over the project `view` reads: each edit reads the text the ones before it left.
/// Refused with the first edit that cannot be made (with several edits its message starts with
/// `edit <n> (<op> <entity>): `). Each file is laid out with its format configuration, read once per plan
/// through the view's disk (ADR 0169).
pub fn plan(view: &ProjectView<'_>, edits: &[Edit]) -> Result<EditPlan, Refusal<EditField>> {
    let steps: Vec<Step> = edits.iter().cloned().map(Step::Edit).collect();
    plan_steps(view, &steps)
}

/// [`plan`] over steps, renames among them.
pub fn plan_steps(view: &ProjectView<'_>, steps: &[Step]) -> Result<EditPlan, Refusal<EditField>> {
    crate::config::refuse_rewrite(view.env().config_problems())?;
    view.project_root()?;
    let harness = Harness::of(view.registries());
    let mut carry = Carry::default();
    let mut at = 0;
    while at < steps.len() {
        let end = match &steps[at] {
            Step::Rename { .. } => at + 1,
            Step::Edit(_) | Step::Verdict(_) => steps[at..]
                .iter()
                .position(|s| matches!(s, Step::Rename { .. }))
                .map_or(steps.len(), |p| at + p),
        };
        let done = match &steps[at] {
            Step::Rename { entity, to } => {
                rename_step(view, &mut carry, entity, to).map_err(|r| (0, r))
            }
            Step::Edit(_) | Step::Verdict(_) => edits(view, &harness, &mut carry, &steps[at..end]),
        };
        done.map_err(|(offset, mut refusal)| {
            if steps.len() > 1 {
                let n = at + offset;
                refusal.error =
                    refusal
                        .error
                        .prefixed(format!("edit {} ({}): ", n + 1, steps[n].describe()));
            }
            refusal
        })?;
        at = end;
    }
    conclude(view, carry)
}

/// A run of edits, planned over the project as the steps before left it; the refusal names the edit's place in the run.
fn edits(
    view: &ProjectView<'_>,
    harness: &Harness,
    carry: &mut Carry,
    run: &[Step],
) -> Result<(), (usize, Refusal<EditField>)> {
    let rebased;
    let local;
    let at: &ProjectView<'_> = if carry.texts.is_empty() {
        view
    } else {
        rebased = view.project().edited(&carry.texts);
        local = ProjectView::of(&rebased).bounded_by(view.containment());
        &local
    };
    let disk = at.containment().disk();
    let mut planner = Planner {
        view: at,
        configs: Configs::new(Forms::none(), &disk),
        texts: BTreeMap::new(),
        scopes: at.graph().scopes().clone(),
        files: BTreeMap::new(),
        declared: std::mem::take(&mut carry.declared),
        moves: std::mem::take(&mut carry.moves),
        raw_moves: std::mem::take(&mut carry.raw_moves),
        sets: std::mem::take(&mut carry.sets),
        harness,
    };
    let mut failed = None;
    for (offset, step) in run.iter().enumerate() {
        let applied = match step {
            Step::Edit(edit) => planner.apply(edit),
            Step::Verdict(verdict) => planner.verdict(verdict),
            Step::Rename { .. } => continue,
        };
        if let Err(refusal) = applied {
            failed = Some((offset, refusal));
            break;
        }
    }
    carry.texts.extend(planner.texts);
    carry.declared = planner.declared;
    carry.moves = planner.moves;
    carry.raw_moves = planner.raw_moves;
    carry.sets = planner.sets;
    failed.map_or(Ok(()), Err)
}

/// Rename `entity` to `to` over the project as the steps before left it, and merge the texts it makes.
fn rename_step(
    view: &ProjectView<'_>,
    carry: &mut Carry,
    entity: &str,
    to: &str,
) -> Result<(), Refusal<EditField>> {
    let rebased;
    let project = if carry.texts.is_empty() {
        view.project()
    } else {
        rebased = view.project().edited(&carry.texts);
        &rebased
    };
    let nav = Navigator::new(ProjectView::of(project).bounded_by(view.containment()));
    let plan = rename::plan(&nav, entity, to).map_err(|refusal| {
        refusal.map_subject(|field| {
            Some(match field {
                RenameField::OldId => EditField::Entity,
                RenameField::NewId => EditField::Id,
            })
        })
    })?;
    for (key, after) in plan.change().after() {
        carry
            .texts
            .insert(key, LineBreak::of(&after).lf(&after).into_owned());
    }
    for (old, new) in plan.moved() {
        carry.declared.moved(old, new);
        compose(&mut carry.moves, old, new);
        carry.raw_moves.push((old.clone(), new.clone()));
    }
    carry.declared.renamed(plan.change().affected_files());
    Ok(())
}

/// The plan the steps made: the change, the ledger it keeps, and the report, judged.
fn conclude(view: &ProjectView<'_>, carry: Carry) -> Result<EditPlan, Refusal<EditField>> {
    let project = view.project();
    let Carry {
        texts,
        declared,
        moves,
        raw_moves,
        sets,
    } = carry;
    let mut set: Vec<(String, String)> = sets
        .into_iter()
        .map(|(made, path, field)| (relocated(&path, &raw_moves[made..]), field))
        .collect();
    set.sort();
    set.dedup();
    let change = PlannedChange::new(
        view.env().sources().clone(),
        "cannot apply the edits",
        texts.into_iter().map(|(key, after)| {
            let before = project.source_text(&key);
            let line_break = LineBreak::of(before.as_deref().unwrap_or(""));
            (key, before, line_break.write(&after).into_owned())
        }),
    );
    let change = change.keeping_ledger(project, LedgerChange { set, moves });
    let added = if change.is_empty() {
        Vec::new()
    } else {
        validate::check(view, &change.after(), &declared, change.ledger_after())?
    };
    let report = EditReport {
        entities: change.entities(),
        affected_files: change.affected_files().map(String::from).collect(),
        diff: change.diff(),
        added: DiagnosticList::new(added, view.registries().codes()),
    };
    Ok(EditPlan::made(change, report))
}

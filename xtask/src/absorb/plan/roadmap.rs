//! A release's milestones as phases (ADR 0372 D4, D7): the three ways that write a phase's references through the id
//! map, the phase's fills, and the tier rule that keeps `gsd/E101` quiet once a phase is written.
//!
//! A reference the map does not hold (an entity outside the selection, one the run drops whole, one that converts to
//! another kind) and a dependency on a phase that is not an earlier one of the same version milestone are ledgered
//! `dropped` with a reason that says which, never stubbed (ADR 0360 D4).

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;

use super::super::how;
use super::super::ids::New;
use super::super::ledger::Why;
use super::super::read::OldEntity;
use super::Planner;

/// Why a dependency on a later phase, or on this phase, is dropped.
const NOT_EARLIER: &str = "it names a phase that is not an earlier one of the same version milestone: a phase \
                           depends on earlier phases of its milestone only (ADR 0372 D4)";
/// Why a dependency on another version milestone's phase is dropped.
const OTHER_MILESTONE: &str = "it names a phase of another version milestone: GSD resolves a phase's depends_on \
                               within its own milestone (ADR 0372 D4)";
/// Why a reference to an entity outside the selection is dropped.
const OUTSIDE: &str =
    "it names an entity outside the selection: a reference is never stubbed (ADR 0360 D4)";

impl Planner {
    /// The tier `e`'s requirement is written with when its tier would be `tier`: `v2` for a `v1` no written phase
    /// lists, when the run writes a phase (ADR 0372 D7); else `tier`.
    pub(super) fn unlisted_tier(&self, e: &OldEntity, tier: &'static str) -> &'static str {
        if tier == "v1" && self.links.writes_phase() && !self.links.listed(&e.id) {
            "v2"
        } else {
            tier
        }
    }

    /// A phase's required values its old milestone lacks: its slug, from its name; its one success criterion, its
    /// description, when it has no exit criteria (none when it has no description either).
    pub(super) fn phase_fills(&mut self, e: &OldEntity) -> Vec<(&'static str, Input)> {
        let Some(draft) = self.own(e) else {
            return Vec::new();
        };
        let mut fills = Vec::new();
        if let Some(Input::One(name)) = draft.fields.get("name") {
            fills.push(("slug", Input::One(how::slug(name))));
        }
        let description = e.fields.get("description").and_then(Value::as_str);
        if !draft.fields.contains_key("success_criteria")
            && let Some(description) = description.filter(|d| !d.trim().is_empty())
        {
            fills.push((
                "success_criteria",
                Input::Many(vec![description.to_string()]),
            ));
        }
        fills
    }

    /// A release's `milestones`: each listed milestone's phase sits under this release's milestone (its scope field,
    /// which the draft holds from the map); one the run writes no phase for is ledgered `dropped`.
    pub(super) fn parent(&mut self, e: &OldEntity, old_field: &str, field: &str, value: &Value) {
        let Some(parent) = self.ids.path(&e.kind, &e.id) else {
            return;
        };
        for item in how::items(value) {
            match self.named(&item, "phase") {
                Ok(new) if new.scope.as_ref().map(|(_, scope)| scope) == Some(&parent) => {
                    let path = new.path();
                    if let Some(draft) = self.drafts.get_mut(&path) {
                        draft.set(field, Input::One(parent.clone()));
                    }
                }
                Ok(_) => self.dropped(e, old_field, &item, OTHER_MILESTONE.to_string()),
                Err(why) => self.dropped(e, old_field, &item, why),
            }
        }
    }

    /// A milestone's `features`: each as the requirement it becomes, through the map.
    pub(super) fn requirements(
        &mut self,
        e: &OldEntity,
        old_field: &str,
        field: &str,
        value: &Value,
    ) {
        let mut paths = Vec::new();
        for item in how::items(value) {
            match self.named(&item, "requirement") {
                Ok(new) => paths.push(new.path()),
                Err(why) => self.dropped(e, old_field, &item, why),
            }
        }
        if let Some(draft) = self.own(e) {
            draft.append(field, paths);
        }
    }

    /// A milestone's `depends_on`: each as its phase's path, when that phase is an earlier one of the same version
    /// milestone.
    pub(super) fn earlier(&mut self, e: &OldEntity, old_field: &str, field: &str, value: &Value) {
        let Some(own) = self.ids.get(&e.kind, &e.id).cloned() else {
            return;
        };
        let mut paths = Vec::new();
        for item in how::items(value) {
            match self.named(&item, "phase") {
                Ok(on) if on.scope != own.scope => {
                    self.dropped(e, old_field, &item, OTHER_MILESTONE.to_string());
                }
                Ok(on) if on.name >= own.name => {
                    self.dropped(e, old_field, &item, NOT_EARLIER.to_string());
                }
                Ok(on) => paths.push(on.path()),
                Err(why) => self.dropped(e, old_field, &item, why),
            }
        }
        if let Some(draft) = self.own(e) {
            draft.append(field, paths);
        }
    }

    /// The entity the old id `id` names, when it converts to a `kind` entity; else why not: it is outside the
    /// selection, the run drops it whole, or it becomes another kind.
    fn named(&self, id: &str, kind: &str) -> Result<New, String> {
        let held = self.known.get(id).map(Vec::as_slice).unwrap_or_default();
        if let Some(new) = held
            .iter()
            .filter_map(|(old, _)| self.ids.get(old, id))
            .find(|new| new.kind == kind)
        {
            return Ok(new.clone());
        }
        match held.first() {
            None => Err(OUTSIDE.to_string()),
            Some((old, true)) => Err(format!(
                "it names a {old} the run drops whole: a reference is never stubbed (ADR 0360 D4)"
            )),
            Some((old, false)) => Err(format!("it names a {old}, which converts to no {kind}")),
        }
    }

    /// The listed `item` of `e`'s `old_field`, ledgered `dropped` with `why`.
    fn dropped(&mut self, e: &OldEntity, old_field: &str, item: &str, why: String) {
        self.ledger
            .record(&e.kind, &e.id, old_field, json!(item), Why::Dropped(why));
    }
}

//! A release's milestones as phases (ADR 0372 D4, D7): the three ways that write a phase's references through the id
//! map, the phase's fills, and the listings and the tier rule that keep `gsd/E101` quiet once a phase is written.
//!
//! A reference names the entity of the old kind its field references (a milestone's `features` a feature, its
//! `depends_on` and a release's `milestones` a milestone), so two kinds sharing an id resolve to one. A reference the
//! map does not hold (an entity outside the selection, one the run drops whole, one that converts to another kind) and
//! a dependency on a phase that is not an earlier one of the same version milestone are ledgered `dropped` with a
//! reason that says which, never stubbed (ADR 0360 D4).
//!
//! `gsd/E101` wants every `v1` requirement in exactly one phase. A requirement no written phase lists is `v2`; one
//! several list is kept by the first in source order (file, line) that lists it, and each later listing is ledgered
//! `dropped`, naming that phase (ADR 0372 D7). Both are keyed by the requirement's new path, never by an old id.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;

use super::super::how;
use super::super::ids::New;
use super::super::ledger::Why;
use super::super::links::MILESTONE;
use super::super::read::OldEntity;
use super::Planner;

/// The old kind a milestone's `features` references.
const FEATURE: &str = "feature";

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
    /// The listings: each requirement a written phase's `features` names, by its new path, kept by the first phase in
    /// source order (file, line, then old id) that lists it.
    pub(super) fn listings_of(&mut self, entities: &[OldEntity]) {
        let mut milestones: Vec<&OldEntity> =
            entities.iter().filter(|e| e.kind == MILESTONE).collect();
        milestones.sort_by(|a, b| (&a.file, a.line, &a.id).cmp(&(&b.file, b.line, &b.id)));
        for e in milestones {
            let Some(phase) = self
                .ids
                .get(&e.kind, &e.id)
                .filter(|new| new.kind == "phase")
            else {
                continue;
            };
            let phase = phase.path();
            for item in e.fields.get("features").map(how::items).unwrap_or_default() {
                if let Ok(new) = self.named(&item, "requirement", FEATURE) {
                    self.listings
                        .entry(new.path())
                        .or_insert_with(|| phase.clone());
                }
            }
        }
    }

    /// The tier `e`'s requirement is written with when its tier would be `tier`: `v2` for a `v1` no written phase
    /// lists, when the run writes a phase (ADR 0372 D7); else `tier`.
    pub(super) fn unlisted_tier(&self, e: &OldEntity, tier: &'static str) -> &'static str {
        let listed = self
            .ids
            .path(&e.kind, &e.id)
            .is_some_and(|path| self.listings.contains_key(&path));
        if tier == "v1" && self.links.writes_phase() && !listed {
            "v2"
        } else {
            tier
        }
    }

    /// A phase's required values its old milestone lacks: its slug, from its title (none when the title holds no ASCII
    /// letter or digit, so the missing-value stop names the phase); its one success criterion, its description, when it
    /// has no exit criteria (none when it has no description either).
    pub(super) fn phase_fills(&mut self, e: &OldEntity) -> Vec<(&'static str, Input)> {
        let Some(draft) = self.own(e) else {
            return Vec::new();
        };
        let mut fills = Vec::new();
        let slug = how::slug(&draft.title);
        if !slug.is_empty() {
            fills.push(("slug", Input::One(slug)));
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

    /// A release's `milestones`: each listed milestone's phase sits under this release's milestone already, its scope
    /// field written by its draft from the map; one the run writes no phase for is ledgered `dropped`. A listed phase
    /// under another version milestone cannot be: `Links::of` stops a milestone two releases list before any plan.
    pub(super) fn parent(&mut self, e: &OldEntity, old_field: &str, value: &Value) {
        for item in how::items(value) {
            if let Err(why) = self.named(&item, "phase", MILESTONE) {
                self.dropped(e, old_field, &item, why);
            }
        }
    }

    /// A milestone's `features`: each as the requirement it becomes, through the map, when this phase keeps it; one an
    /// earlier phase keeps is ledgered `dropped`, naming that phase.
    pub(super) fn requirements(
        &mut self,
        e: &OldEntity,
        old_field: &str,
        field: &str,
        value: &Value,
    ) {
        let own = self.ids.path(&e.kind, &e.id).unwrap_or_default();
        let mut paths = Vec::new();
        for item in how::items(value) {
            match self.named(&item, "requirement", FEATURE) {
                Ok(new) => match self.listings.get(&new.path()) {
                    Some(keeper) if *keeper != own => {
                        let why = format!(
                            "it names a requirement the phase {keeper} lists first: a requirement is in one phase, \
                             the first in source order that lists it (gsd/E101, ADR 0372 D7)"
                        );
                        self.dropped(e, old_field, &item, why);
                    }
                    _ => paths.push(new.path()),
                },
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
            match self.named(&item, "phase", MILESTONE) {
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

    /// The entity the old id `id` names in a field that references the old kind `references`, when it converts to a
    /// `kind` entity; else why not: it is outside the selection, the run drops it whole, or it becomes another kind.
    /// The old kind `references` comes first, so two kinds sharing an id resolve to the one the field names (ADR 0372
    /// D3); another kind that holds the id answers only when that one does not convert to a `kind`.
    fn named(&self, id: &str, kind: &str, references: &str) -> Result<New, String> {
        let mut held: Vec<&(String, bool)> = self.known.get(id).into_iter().flatten().collect();
        // A stable sort: the referenced kind first, the others in the order the run holds them.
        held.sort_by_key(|(old, _)| old != references);
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

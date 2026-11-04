//! Declaring value checkers (ADR 0336), with the handlers one export serves: `__check_values` routes each item to its
//! checker by name. A checker plans the host's probes and judges their outcomes; the host performs them.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value;
use specforge_protocol_types::{
    CHECK_VALUES_EXPORT, CheckFinding, CheckItem, CheckPhase, CheckPlan, CheckRef,
    CheckValuesInput, CheckerDescriptor, FieldDescriptor, PlannedItem, Root, ValueForm,
};

use crate::{ContributionsBuilder, FieldBuilder};

type Plan = Box<dyn Fn(&CheckItem) -> PlannedItem>;
type Judge = Box<dyn Fn(&CheckItem) -> Vec<CheckFinding>>;

pub(crate) struct Handlers {
    plan: Plan,
    judge: Option<Judge>,
}

/// The handlers of the checkers declared, shared with the one export's route.
pub(crate) type Checkers = Rc<RefCell<BTreeMap<String, Handlers>>>;

impl ContributionsBuilder {
    /// Declare a value checker: what it reads, and its handlers.
    ///
    /// ```
    /// # use specforge_extension_sdk::prelude::*;
    /// # let mut c = ContributionsBuilder::default();
    /// c.checker("no_todo", |k| {
    ///     k.applies_to(&["string"]).codes(&["W001"]).plan(|item| {
    ///         let found = item.text.contains("TODO");
    ///         let finding = CheckFinding::new("W001", "this value still says TODO");
    ///         PlannedItem::findings(found.then_some(finding).into_iter().collect())
    ///     });
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// When it declares no plan: a declaration cannot lack its code.
    pub fn checker(&mut self, name: &str, f: impl FnOnce(&mut CheckerBuilder)) -> &mut Self {
        let mut builder = CheckerBuilder {
            descriptor: CheckerDescriptor {
                name: name.to_string(),
                ..Default::default()
            },
            plan: None,
            judge: None,
        };
        f(&mut builder);
        let Some(plan) = builder.plan else {
            panic!("checker '{name}' declares no plan: answer it with `k.plan(...)`");
        };
        if self.decl.checkers.is_empty() {
            let checkers = self.checkers.clone();
            self.add_operation(
                CHECK_VALUES_EXPORT.to_string(),
                "the value checkers".to_string(),
                crate::operations::wire("check_values", move |input: &CheckValuesInput| {
                    answer(&checkers.borrow(), input)
                }),
            );
        }
        self.checkers.borrow_mut().insert(
            name.to_string(),
            Handlers {
                plan,
                judge: builder.judge,
            },
        );
        self.decl.checkers.push(builder.descriptor);
        self
    }
}

/// Each item's answer, by its checker and the call's phase. A checker the extension does not have fails the call.
fn answer(
    checkers: &BTreeMap<String, Handlers>,
    input: &CheckValuesInput,
) -> Result<CheckPlan, String> {
    let items = input.items.iter().map(|item| {
        let Some(handlers) = checkers.get(&item.checker) else {
            return Err(format!("no checker named '{}'", item.checker));
        };
        let planned = match input.phase {
            CheckPhase::Plan => (handlers.plan)(item),
            CheckPhase::Judge => PlannedItem::findings(
                handlers
                    .judge
                    .as_ref()
                    .map_or_else(Vec::new, |judge| judge(item)),
            ),
        };
        Ok(PlannedItem {
            id: item.id,
            ..planned
        })
    });
    Ok(CheckPlan {
        items: items.collect::<Result<_, String>>()?,
    })
}

/// Builder for a [`CheckerDescriptor`] and its handlers.
pub struct CheckerBuilder {
    descriptor: CheckerDescriptor,
    plan: Option<Plan>,
    judge: Option<Judge>,
}

impl CheckerBuilder {
    pub fn description(&mut self, description: &str) -> &mut Self {
        self.descriptor.description = Some(description.to_string());
        self
    }

    /// The field types it reads: `string`, `string_list`, `identifier`, `date`, `integer`.
    pub fn applies_to(&mut self, types: &[&str]) -> &mut Self {
        self.descriptor.applies_to = types.iter().map(|t| t.to_string()).collect();
        self
    }

    /// An argument it takes, in the forms a setting declares. A gentler reading is an enum whose values select codes.
    pub fn arg(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut field = FieldBuilder(FieldDescriptor {
            name: name.to_string(),
            field_type: "string".to_string(),
            ..Default::default()
        });
        f(&mut field);
        self.descriptor.args.push(field.0);
        self
    }

    /// A capability its probes use, by the scope name the extension declares it under.
    pub fn scope(&mut self, scope: &str) -> &mut Self {
        self.descriptor.scopes.push(scope.to_string());
        self
    }

    /// Every code a finding of it may carry.
    pub fn codes(&mut self, codes: &[&str]) -> &mut Self {
        self.descriptor.codes = codes.iter().map(|c| c.to_string()).collect();
        self
    }

    /// What its values are, for the editor: path completion and document links read it, no check does.
    pub fn value(&mut self, form: ValueForm) -> &mut Self {
        self.descriptor.value = form;
        self
    }

    /// A [`ValueForm::Path`] checker's argument that names a bare path's default root.
    pub fn root_arg(&mut self, arg: &str) -> &mut Self {
        self.descriptor.root_arg = Some(arg.to_string());
        self
    }

    /// Its findings from the text, or the probes it needs performed.
    pub fn plan(&mut self, plan: impl Fn(&CheckItem) -> PlannedItem + 'static) -> &mut Self {
        self.plan = Some(Box::new(plan));
        self
    }

    /// Its findings from `item.outcomes`, what its probes answered; none when it declares no judge.
    pub fn judge(
        &mut self,
        judge: impl Fn(&CheckItem) -> Vec<CheckFinding> + 'static,
    ) -> &mut Self {
        self.judge = Some(Box::new(judge));
        self
    }
}

impl FieldBuilder {
    /// A check each of the field's values passes: a checker of this extension by its name, or another's as
    /// `<extension>:<checker>` (which needs a peer).
    pub fn check(&mut self, checker: &str) -> &mut Self {
        self.checked(CheckRef::named(checker))
    }

    /// [`Self::check`] with arguments.
    pub fn check_with(&mut self, checker: &str, args: Value) -> &mut Self {
        self.checked(CheckRef::with(checker, args))
    }

    /// A value passes when one of these checks does.
    pub fn any_of(&mut self, checks: impl IntoIterator<Item = CheckRef>) -> &mut Self {
        self.checked(CheckRef::AnyOf {
            any_of: checks.into_iter().collect(),
        })
    }

    /// The value names a file that exists, a bare path starting at `root`.
    pub fn file(&mut self, root: Root) -> &mut Self {
        self.checked(CheckRef::with(
            "@specforge/checks:file",
            serde_json::json!({ "root": root.as_str() }),
        ))
    }

    fn checked(&mut self, check: CheckRef) -> &mut Self {
        self.0.checks.push(check);
        self
    }
}

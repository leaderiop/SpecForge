//! The `@gsd/core` twin the kernel tests share declares the plan check and the plan checker's route as the shipped
//! `@gsd/core` does (its pinned declaration), so what a test proves on the twin holds for the builtin.

use specforge_component::committed::pinned_builtin_declarations;
use specforge_project::testing::{PLAN_CHECK_INPUTS, gsd_context_builder};
use specforge_protocol_types::{ContextTier, ExtensionDeclaration};

/// Named selectors, in order: `(name, selector)`.
type Named = Vec<(String, String)>;

/// Each `(name, selector)` of the plan check `declaration` declares, and its plan checker's route: the allowance and
/// each section's `(name, selector)`, every one core.
fn plan_check(declaration: &ExtensionDeclaration) -> (Named, u32, Named) {
    let judged = declaration
        .validation_rules
        .iter()
        .filter_map(|rule| rule.judged.as_ref())
        .find(|judged| judged.judge == "gsd-plan-checker")
        .expect("the plan check");
    let inputs = judged.targets[0]
        .inputs
        .iter()
        .map(|input| (input.name.clone(), input.selector.clone()))
        .collect();
    let route = declaration
        .contexts
        .iter()
        .find(|route| route.name == "plan-checker")
        .expect("the plan checker's route");
    assert!(
        route.sections.iter().all(|s| s.tier == ContextTier::Core),
        "{route:?}"
    );
    let sections = route
        .sections
        .iter()
        .map(|s| (s.name.clone(), s.selector.clone()))
        .collect();
    (inputs, route.budget, sections)
}

#[test]
fn the_twin_declares_the_shipped_plan_check() {
    let shipped = pinned_builtin_declarations()
        .expect("the pinned declarations")
        .into_iter()
        .find(|declaration| declaration.name() == "@gsd/core")
        .expect("@gsd/core is a builtin");
    let twin = plan_check(&gsd_context_builder().declaration());
    let expected: Named = PLAN_CHECK_INPUTS
        .iter()
        .map(|(name, selector)| (name.to_string(), selector.to_string()))
        .collect();
    assert_eq!(twin.0, expected);
    assert_eq!(plan_check(&shipped), twin);
    assert_eq!(
        twin.1, 0,
        "a route of core sections only allows nothing beyond them"
    );
    assert_eq!(twin.2, expected, "the route serves what the check reads");
}

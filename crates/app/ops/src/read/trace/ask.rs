//! What a trace request asks for (ADR 0101): a trace of one entity or of every entity, following what it is asked to
//! follow (ADR 0330), or an agent plan's check.

use super::{Follow, Target};
use crate::read::plan::AgentPlan;
use crate::{OpError, OpErrorKind, Refusal};

/// The code of the refusal of an entity and a plan in one request: they are two traces.
pub const ENTITY_WITH_PLAN: &str = "entity_with_plan";

/// The code of the refusal of a plan with a route or the reach: a plan is checked, never traced.
pub const PLAN_WITH_ROUTE: &str = "plan_with_route";

/// The code of the refusal of a route and the reach in one request: a trace follows one.
pub const ROUTE_WITH_REACH: &str = "route_with_reach";

/// The code of the refusal of a route no loaded extension declares.
pub const UNKNOWN_ROUTE: &str = "unknown_route";

/// The code of the refusal of a route that starts at another kind than the traced entity's.
pub const ROUTE_OF_ANOTHER_KIND: &str = "route_of_another_kind";

/// The field of a trace request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceField {
    /// The entity traced: one the graph lacks.
    EntityId,
    /// The plan given beside an entity, or beside a route or the reach.
    Plan,
    /// The route asked for (or the reach).
    Route,
}

/// The read a trace request asks for.
#[derive(Debug, Clone, Copy)]
pub enum Ask<'a> {
    /// The trace operation over `Target`, following `Follow`: one entity's trace when an entity is given, every
    /// entity's when neither an entity nor a plan is. A surface may decline to offer `Target::Every` (MCP does, ADR
    /// 0101 D4).
    Trace(Target<'a>, Follow<'a>),
    /// The check of an agent plan against the graph (`plan::check`).
    Plan(&'a AgentPlan),
}

impl<'a> Ask<'a> {
    /// The one reading of a trace request's entity, plan, route and reach, for every surface: an entity, a plan,
    /// or neither, following the named route, the reach, or (neither) what its kind declares. An entity and a plan
    /// are refused, invalid input about the plan, and so is a plan with a route or the reach; a route and the reach
    /// are refused about the route.
    pub fn of(
        entity: Option<&'a str>,
        plan: Option<&'a AgentPlan>,
        route: Option<&'a str>,
        reach: bool,
    ) -> Result<Self, Refusal<TraceField>> {
        let follow = match (route, reach) {
            (Some(_), true) => {
                return Err(OpError::new(
                    OpErrorKind::InvalidInput,
                    ROUTE_WITH_REACH,
                    "a trace follows a route or the reach: give one",
                )
                .about(TraceField::Route));
            }
            (Some(name), false) => Follow::Route(name),
            (None, true) => Follow::Reach,
            (None, false) => Follow::Declared,
        };
        match (entity, plan) {
            (Some(_), Some(_)) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                ENTITY_WITH_PLAN,
                "an entity and a plan are two traces: give one",
            )
            .about(TraceField::Plan)),
            (None, Some(_)) if follow != Follow::Declared => Err(OpError::new(
                OpErrorKind::InvalidInput,
                PLAN_WITH_ROUTE,
                "a plan is checked against the graph, not traced: give a route or the reach with an entity",
            )
            .about(TraceField::Plan)),
            (Some(entity), None) => Ok(Ask::Trace(Target::Entity(entity), follow)),
            (None, Some(plan)) => Ok(Ask::Plan(plan)),
            (None, None) => Ok(Ask::Trace(Target::Every, follow)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> AgentPlan {
        AgentPlan::parse(
            r#"{"plan_id":"p","entries":[{"entity_id":"login","action":"implement"}]}"#,
        )
        .unwrap()
    }

    #[specforge_test_macros::test(
        behavior = "validate_agent_plan",
        verify = "an entity and a plan together are invalid input about the plan, on every surface"
    )]
    fn an_entity_with_a_plan_is_refused_about_the_plan() {
        let refusal = Ask::of(Some("login"), Some(&plan()), None, false).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, ENTITY_WITH_PLAN);
        assert_eq!(refusal.subject, Some(TraceField::Plan));
    }

    #[test]
    fn either_or_neither_is_one_trace() {
        let plan = plan();
        assert!(matches!(
            Ask::of(Some("login"), None, None, false),
            Ok(Ask::Trace(Target::Entity("login"), Follow::Declared))
        ));
        assert!(matches!(
            Ask::of(None, Some(&plan), None, false),
            Ok(Ask::Plan(_))
        ));
        assert!(matches!(
            Ask::of(None, None, None, true),
            Ok(Ask::Trace(Target::Every, Follow::Reach))
        ));
    }

    #[specforge_test_macros::test(
        behavior = "follow_trace_route",
        verify = "a named route, the reach, and every entity are traced as asked, and a wrong route is refused"
    )]
    fn a_plan_with_a_route_or_the_reach_is_refused_about_the_plan() {
        for (route, reach) in [(Some("shop/order-thread"), false), (None, true)] {
            let refusal = Ask::of(None, Some(&plan()), route, reach).unwrap_err();
            assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
            assert_eq!(refusal.error.code, PLAN_WITH_ROUTE);
            assert_eq!(refusal.subject, Some(TraceField::Plan));
        }
        let refusal = Ask::of(Some("login"), None, Some("shop/order-thread"), true).unwrap_err();
        assert_eq!(refusal.error.code, ROUTE_WITH_REACH);
        assert_eq!(refusal.subject, Some(TraceField::Route));
        assert!(matches!(
            Ask::of(Some("login"), None, Some("shop/order-thread"), false),
            Ok(Ask::Trace(
                Target::Entity("login"),
                Follow::Route("shop/order-thread")
            ))
        ));
    }
}

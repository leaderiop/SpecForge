//! What a trace request asks for (ADR 0101): a trace of one entity, of every entity within one (ADR 0355) or of every
//! entity, following what it is asked to follow (ADR 0330), or an agent plan's check.

use super::{Follow, Target};
use crate::read::plan::AgentPlan;
use crate::{OpError, OpErrorKind, Refusal};

/// The code of the refusal of an entity and a plan in one request: they are two traces.
pub const ENTITY_WITH_PLAN: &str = "entity_with_plan";

/// The code of the refusal of an entity and `in` in one request: they are two traces.
pub const ENTITY_WITH_IN: &str = "entity_with_in";

/// The code of the refusal of a plan and `in` in one request: a plan is checked, never traced within an entity.
pub const PLAN_WITH_IN: &str = "plan_with_in";

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
    /// The entity traced within (`in`): one the graph lacks, or given beside an entity or a plan.
    In,
}

/// A trace request as every surface reads it: at most one of an entity, an entity to trace within and a plan, and
/// what to follow.
#[derive(Debug, Clone, Copy, Default)]
pub struct TraceRequest<'a> {
    /// The entity to trace.
    pub entity: Option<&'a str>,
    /// The entity to trace within: it, its scope at any depth, and the entities one reference away (`in`).
    pub within: Option<&'a str>,
    /// The agent plan to check instead of tracing.
    pub plan: Option<&'a AgentPlan>,
    /// The route to follow (its qualified name).
    pub route: Option<&'a str>,
    /// Follow the reach both ways, whatever route starts at the kind.
    pub reach: bool,
}

/// The read a trace request asks for.
#[derive(Debug, Clone, Copy)]
pub enum Ask<'a> {
    /// The trace operation over `Target`, following `Follow`: one entity's trace when an entity is given, the traces
    /// within an entity when `in` is, every entity's when none of an entity, `in` and a plan is. A surface may decline
    /// to offer `Target::Every` (MCP does, ADR 0101 D4).
    Trace(Target<'a>, Follow<'a>),
    /// The check of an agent plan against the graph (`plan::check`).
    Plan(&'a AgentPlan),
}

impl<'a> Ask<'a> {
    /// The one reading of a trace request, for every surface: an entity, an entity to trace within, a plan, or
    /// none, following the named route, the reach, or (neither) what its kind declares. An entity and a plan are
    /// refused, invalid input about the plan, and so is a plan with a route or the reach; `in` beside an entity or a
    /// plan is refused about `in`; a route and the reach are refused about the route.
    pub fn of(request: TraceRequest<'a>) -> Result<Self, Refusal<TraceField>> {
        let TraceRequest {
            entity,
            within,
            plan,
            route,
            reach,
        } = request;
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
        match (entity, within, plan) {
            (Some(_), _, Some(_)) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                ENTITY_WITH_PLAN,
                "an entity and a plan are two traces: give one",
            )
            .about(TraceField::Plan)),
            (Some(_), Some(_), None) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                ENTITY_WITH_IN,
                "an entity and in are two traces: give one",
            )
            .about(TraceField::In)),
            (None, Some(_), Some(_)) => Err(OpError::new(
                OpErrorKind::InvalidInput,
                PLAN_WITH_IN,
                "a plan is checked against the graph, not traced within an entity",
            )
            .about(TraceField::In)),
            (None, None, Some(_)) if follow != Follow::Declared => Err(OpError::new(
                OpErrorKind::InvalidInput,
                PLAN_WITH_ROUTE,
                "a plan is checked against the graph, not traced: give a route or the reach with an entity",
            )
            .about(TraceField::Plan)),
            (Some(entity), None, None) => Ok(Ask::Trace(Target::Entity(entity), follow)),
            (None, Some(within), None) => Ok(Ask::Trace(Target::Within(within), follow)),
            (None, None, Some(plan)) => Ok(Ask::Plan(plan)),
            (None, None, None) => Ok(Ask::Trace(Target::Every, follow)),
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

    /// A request for `entity`, `within` and `plan`, following what each kind declares.
    fn request<'a>(
        entity: Option<&'a str>,
        within: Option<&'a str>,
        plan: Option<&'a AgentPlan>,
    ) -> TraceRequest<'a> {
        TraceRequest {
            entity,
            within,
            plan,
            ..TraceRequest::default()
        }
    }

    #[specforge_test_macros::test(
        behavior = "validate_agent_plan",
        verify = "an entity and a plan together are invalid input about the plan, on every surface"
    )]
    fn an_entity_with_a_plan_is_refused_about_the_plan() {
        let plan = plan();
        let refusal = Ask::of(request(Some("login"), None, Some(&plan))).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, ENTITY_WITH_PLAN);
        assert_eq!(refusal.subject, Some(TraceField::Plan));
    }

    #[test]
    fn either_or_neither_is_one_trace() {
        let plan = plan();
        assert!(matches!(
            Ask::of(request(Some("login"), None, None)),
            Ok(Ask::Trace(Target::Entity("login"), Follow::Declared))
        ));
        assert!(matches!(
            Ask::of(request(None, Some("v1.0"), None)),
            Ok(Ask::Trace(Target::Within("v1.0"), Follow::Declared))
        ));
        assert!(matches!(
            Ask::of(request(None, None, Some(&plan))),
            Ok(Ask::Plan(_))
        ));
        let reach = TraceRequest {
            reach: true,
            ..TraceRequest::default()
        };
        assert!(matches!(
            Ask::of(reach),
            Ok(Ask::Trace(Target::Every, Follow::Reach))
        ));
    }

    #[specforge_test_macros::test(
        behavior = "trace_within_entity",
        verify = "in beside an entity or a plan is refused about in"
    )]
    fn in_beside_an_entity_or_a_plan_is_refused_about_in() {
        let plan = plan();
        let refusal = Ask::of(request(Some("login"), Some("v1.0"), None)).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, ENTITY_WITH_IN);
        assert_eq!(refusal.subject, Some(TraceField::In));
        let refusal = Ask::of(request(None, Some("v1.0"), Some(&plan))).unwrap_err();
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
        assert_eq!(refusal.error.code, PLAN_WITH_IN);
        assert_eq!(refusal.subject, Some(TraceField::In));
        let routed = TraceRequest {
            route: Some("gsd/requirement-thread"),
            ..request(None, Some("v1.0"), None)
        };
        assert!(matches!(
            Ask::of(routed),
            Ok(Ask::Trace(
                Target::Within("v1.0"),
                Follow::Route("gsd/requirement-thread")
            ))
        ));
    }

    #[specforge_test_macros::test(
        behavior = "follow_trace_route",
        verify = "a named route, the reach, and every entity are traced as asked, and a wrong route is refused"
    )]
    fn a_plan_with_a_route_or_the_reach_is_refused_about_the_plan() {
        let plan = plan();
        for (route, reach) in [(Some("shop/order-thread"), false), (None, true)] {
            let asked = TraceRequest {
                route,
                reach,
                ..request(None, None, Some(&plan))
            };
            let refusal = Ask::of(asked).unwrap_err();
            assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
            assert_eq!(refusal.error.code, PLAN_WITH_ROUTE);
            assert_eq!(refusal.subject, Some(TraceField::Plan));
        }
        let routed = TraceRequest {
            route: Some("shop/order-thread"),
            ..request(Some("login"), None, None)
        };
        let refusal = Ask::of(TraceRequest {
            reach: true,
            ..routed
        })
        .unwrap_err();
        assert_eq!(refusal.error.code, ROUTE_WITH_REACH);
        assert_eq!(refusal.subject, Some(TraceField::Route));
        assert!(matches!(
            Ask::of(routed),
            Ok(Ask::Trace(
                Target::Entity("login"),
                Follow::Route("shop/order-thread")
            ))
        ));
    }
}

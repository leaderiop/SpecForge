//! The routes an extension declares (protocol 1.15.0, ADR 0330): from an
//! entity of a start kind, the steps `specforge trace` follows, in order, and
//! what it prints of each entity it reaches.

use serde::{Deserialize, Serialize};

/// The word a step's `from` names the start entity by; no step may take it as its name.
pub const ROUTE_START: &str = "start";

/// A route: from an entity of `start`, the steps `specforge trace` follows and
/// what it prints of each entity it reaches.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteDescriptor {
    /// Kebab case, unique among the extension's routes; qualified by the
    /// extension's short name (`shop/order-thread`).
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The kind whose entities the route starts from.
    pub start: String,
    /// The route `trace` follows from `start` when several start there (W195
    /// otherwise).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
    /// What the thread prints of the start entity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<RouteAnnotationDescriptor>,
    pub steps: Vec<RouteStepDescriptor>,
}

/// One step of a route: the entities `via` selects from each entity of `from`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteStepDescriptor {
    /// A word, unique in the route, never [`ROUTE_START`].
    pub name: String,
    /// [`ROUTE_START`], or an earlier step's name; absent: the previous step (the
    /// start for the first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Selector steps (ADR 0290's grammar without a head: `<invoice.order`,
    /// `.items`, `<shipment.order.items`) from each entity of `from`.
    pub via: String,
    /// The word printed before each match's name (`invoice` → `invoice INV-7`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<RouteAnnotationDescriptor>,
}

/// One annotation: exactly one of `field` and `judgement`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteAnnotationDescriptor {
    /// A field (or `block.member`) of the entity, or of the entities `of`
    /// picks from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Selector steps from the entity to the entities whose `field` is read
    /// (`<shipment.order`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    /// A judged rule's code: its own (`E501`), qualified by the extension's
    /// namespace, or qualified (`shop/E501`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judgement: Option<String>,
    /// What prints when there is nothing to print (`not shipped`); absent:
    /// nothing prints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent: Option<String>,
}

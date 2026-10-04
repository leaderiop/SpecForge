//! Product queries: pure reads of the graph the host passes a command.
//!
//! Each one answers a planning question over this extension's own kinds and
//! the fields they declare (`status`, `priority`, `features`, ...). The
//! host knows none of them; it runs the `cmd__product_*` exports in
//! `commands.rs`, which render these results.

use serde::Serialize;
use specforge_extension_sdk::prelude::{CommandError, CommandGraph, GraphNode};
use std::collections::BTreeMap;

/// The kinds this extension declares, in the order `health` reports them.
pub const PRODUCT_KINDS: &[&str] = &[
    "feature",
    "journey",
    "deliverable",
    "milestone",
    "module",
    "term",
    "persona",
    "channel",
    "release",
];

/// The kinds with a lifecycle `status`, in the order `bulk-status` reports
/// them.
const STATUS_KINDS: &[&str] = &[
    "feature",
    "milestone",
    "deliverable",
    "persona",
    "channel",
    "release",
];

// ── Listing ────────────────────────────────────────────────────────────────

/// `Priority`, most important first.
pub const PRIORITY: &[&str] = &["critical", "high", "medium", "low"];
const FEATURE_STATUS: &[&str] = &[
    "proposed",
    "accepted",
    "in_progress",
    "done",
    "deferred",
    "deprecated",
];
const DELIVERABLE_STATUS: &[&str] = &["draft", "in_progress", "shipped", "deprecated"];
const MILESTONE_STATUS: &[&str] = &["planned", "in_progress", "completed", "blocked"];
const RELEASE_STATUS: &[&str] = &["planned", "in_progress", "released", "recalled"];
const ACTIVE_STATUS: &[&str] = &["active", "deprecated"];
const ARTIFACT_TYPE: &[&str] = &[
    "cli",
    "service",
    "library",
    "web_app",
    "mobile_app",
    "api",
    "extension",
    "documentation",
    "package",
];
const TECHNICAL_LEVEL: &[&str] = &[
    "expert",
    "advanced",
    "intermediate",
    "beginner",
    "non_technical",
];
const INTERACTION_MODEL: &[&str] = &[
    "request_response",
    "event_driven",
    "batch",
    "streaming",
    "bidirectional",
    "manual",
];

/// `ProductListSortOrder`.
pub const SORT_ORDER: &[&str] = &["asc", "desc"];

/// A list's page size when the caller sets none, and the bounds a set one
/// is clamped to.
pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 1000;

/// An arg a list command filters by: the entities whose `arg` field equals
/// its value (a reference field matches the id it names).
#[derive(Debug)]
pub struct FilterArg {
    pub arg: &'static str,
    /// The values a closed enum takes, in its order (which a sort follows);
    /// `None` for an open value (an id, an open enum like `family`).
    pub values: Option<&'static [&'static str]>,
    /// What an entity without the field counts as (`FeatureStatus`: absent
    /// is `proposed`).
    pub absent_as: Option<&'static str>,
}

const fn closed(arg: &'static str, values: &'static [&'static str]) -> FilterArg {
    FilterArg {
        arg,
        values: Some(values),
        absent_as: None,
    }
}

const fn lifecycle(values: &'static [&'static str]) -> FilterArg {
    FilterArg {
        arg: "status",
        values: Some(values),
        absent_as: Some(values[0]),
    }
}

const fn open(arg: &'static str) -> FilterArg {
    FilterArg {
        arg,
        values: None,
        absent_as: None,
    }
}

/// One list command's kind: the payload key its entries are under, and
/// the args beside `--tags` that filter it.
#[derive(Debug)]
pub struct ListKind {
    pub kind: &'static str,
    pub plural: &'static str,
    pub filters: &'static [FilterArg],
}

impl ListKind {
    /// The filter on `field`, if the kind has one.
    fn filter(&self, field: &str) -> Option<&FilterArg> {
        self.filters.iter().find(|f| f.arg == field)
    }
}

pub const FEATURES: ListKind = ListKind {
    kind: "feature",
    plural: "features",
    filters: &[lifecycle(FEATURE_STATUS), closed("priority", PRIORITY)],
};
pub const JOURNEYS: ListKind = ListKind {
    kind: "journey",
    plural: "journeys",
    filters: &[closed("priority", PRIORITY), open("persona")],
};
pub const DELIVERABLES: ListKind = ListKind {
    kind: "deliverable",
    plural: "deliverables",
    filters: &[
        lifecycle(DELIVERABLE_STATUS),
        closed("artifact_type", ARTIFACT_TYPE),
    ],
};
pub const MILESTONES: ListKind = ListKind {
    kind: "milestone",
    plural: "milestones",
    filters: &[lifecycle(MILESTONE_STATUS), closed("priority", PRIORITY)],
};
pub const MODULES: ListKind = ListKind {
    kind: "module",
    plural: "modules",
    // `ModuleFamily` is open: a family outside the standard set is I062,
    // not an error, so it is a value to match, not one to refuse.
    filters: &[open("family")],
};
pub const TERMS: ListKind = ListKind {
    kind: "term",
    plural: "terms",
    filters: &[],
};
pub const PERSONAS: ListKind = ListKind {
    kind: "persona",
    plural: "personas",
    filters: &[
        closed("status", ACTIVE_STATUS),
        closed("technical_level", TECHNICAL_LEVEL),
    ],
};
pub const CHANNELS: ListKind = ListKind {
    kind: "channel",
    plural: "channels",
    filters: &[
        closed("status", ACTIVE_STATUS),
        closed("interaction_model", INTERACTION_MODEL),
    ],
};
pub const RELEASES: ListKind = ListKind {
    kind: "release",
    plural: "releases",
    filters: &[lifecycle(RELEASE_STATUS)],
};

/// Which entities a list command returns (`ProductListFilter`, validated):
/// one kind, narrowed by every filter set (AND), sorted, then paged.
#[derive(Debug)]
pub struct ListFilter<'a> {
    pub kind: &'a ListKind,
    /// `(field, value)`: the entity's field equals the value.
    pub equals: Vec<(&'a str, &'a str)>,
    /// The entity has one of these tags; empty matches every entity.
    pub tags: Vec<&'a str>,
    pub sort_by: &'a str,
    pub descending: bool,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl<'a> ListFilter<'a> {
    /// Every entity of `kind`, by id, on the first page.
    pub fn all(kind: &'a ListKind) -> Self {
        ListFilter {
            kind,
            equals: Vec::new(),
            tags: Vec::new(),
            sort_by: "id",
            descending: false,
            offset: None,
            limit: None,
        }
    }
}

/// Whether `kind` entities have `field` to sort by: `id`, `title`, the
/// shared `tags`, or a field the kind declares.
pub fn sortable(kind: &str, field: &str) -> bool {
    matches!(field, "id" | "title" | "tags") || declared_fields(kind).contains(&field)
}

/// The fields `kind` declares (`describe_entities.json`).
fn declared_fields(kind: &str) -> Vec<&'static str> {
    static KINDS: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let kinds = KINDS.get_or_init(|| {
        serde_json::from_slice(crate::DESCRIBE_ENTITIES).unwrap_or(serde_json::Value::Null)
    });
    kinds["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|k| k["keyword"] == kind)
        .flat_map(|k| k["fields"].as_array().into_iter().flatten())
        .filter_map(|f| f["name"].as_str())
        .collect()
}

/// The `kind` entities `filter` matches, sorted as it asks: by the field's
/// enum order when it is a closed enum, else by its text; an entity without
/// the field last; ties by id ascending.
pub fn list_nodes<'g>(graph: &'g CommandGraph, filter: &ListFilter) -> Vec<&'g GraphNode> {
    let kind = filter.kind;
    let value = |node: &GraphNode, field: &str| -> Option<String> {
        let absent_as = kind.filter(field).and_then(|f| f.absent_as);
        match node.fields.get(field) {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Array(items)) => Some(
                items
                    .iter()
                    .filter_map(|i| i.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            _ => absent_as.map(str::to_string),
        }
    };
    let mut nodes: Vec<&GraphNode> = graph
        .nodes_of_kind(kind.kind)
        .filter(|n| {
            filter.equals.iter().all(|(field, wanted)| {
                value(n, field).as_deref() == Some(*wanted)
                    || graph
                        .edges_from(&n.id)
                        .iter()
                        .any(|e| e.label == *field && e.target == *wanted)
            })
        })
        .filter(|n| {
            filter.tags.is_empty() || n.list("tags").iter().any(|t| filter.tags.contains(t))
        })
        .collect();

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Key {
        Rank(usize),
        Text(String),
    }
    let key = |node: &GraphNode| -> Option<Key> {
        match filter.sort_by {
            "id" => Some(Key::Text(node.id.clone())),
            "title" => node.title.clone().map(Key::Text),
            field => {
                let v = value(node, field)?;
                let order = kind.filter(field).and_then(|f| f.values).or(match field {
                    "priority" => Some(PRIORITY),
                    _ => None,
                });
                Some(match order {
                    Some(order) => {
                        Key::Rank(order.iter().position(|o| *o == v).unwrap_or(order.len()))
                    }
                    None => Key::Text(v),
                })
            }
        }
    };
    nodes.sort_by(|a, b| {
        let by_key = match (key(a), key(b)) {
            (Some(x), Some(y)) if filter.descending => y.cmp(&x),
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        };
        by_key.then_with(|| a.id.cmp(&b.id))
    });
    nodes
}

/// One page of `items` (`PaginationMetadata`): `limit` defaults to
/// [`DEFAULT_LIMIT`] and is clamped to [1, [`MAX_LIMIT`]]; an offset past
/// the end is an empty page. `total` counts every item.
#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

pub fn paginate<T>(items: Vec<T>, offset: Option<usize>, limit: Option<usize>) -> Page<T> {
    let total = items.len();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let items: Vec<T> = items.into_iter().skip(offset).take(limit).collect();
    let has_more = offset.saturating_add(items.len()) < total;
    Page {
        items,
        total,
        offset,
        limit,
        has_more,
    }
}

impl<T: Serialize> Page<T> {
    /// The page as its payload: the items under `key`, then the
    /// pagination metadata.
    pub fn payload(&self, key: &str) -> serde_json::Value {
        serde_json::json!({
            key: self.items,
            "total": self.total,
            "offset": self.offset,
            "limit": self.limit,
            "has_more": self.has_more,
        })
    }
}

/// An entry of a list command's payload (`FeatureListEntry`, ...), and its
/// row in the human table.
pub trait ListEntry: Serialize + Sized {
    const KIND: &'static ListKind;
    const HEADERS: &'static [&'static str];
    fn of(graph: &CommandGraph, node: &GraphNode) -> Self;
    fn row(&self) -> Vec<String>;
}

/// The page of `T` entries `filter` selects.
pub fn list<T: ListEntry>(graph: &CommandGraph, filter: &ListFilter) -> Page<T> {
    let entries = list_nodes(graph, filter)
        .into_iter()
        .map(|n| T::of(graph, n))
        .collect();
    paginate(entries, filter.offset, filter.limit)
}

fn tags(node: &GraphNode) -> Option<Vec<String>> {
    let tags = node.list("tags");
    (!tags.is_empty()).then(|| tags.into_iter().map(str::to_string).collect())
}

fn title(node: &GraphNode) -> String {
    node.title.clone().unwrap_or_default()
}

fn cell(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "-".to_string())
}

#[derive(Debug, Serialize)]
pub struct FeatureListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for FeatureListEntry {
    const KIND: &'static ListKind = &FEATURES;
    const HEADERS: &'static [&'static str] = &["id", "title", "status", "priority"];
    fn of(_: &CommandGraph, n: &GraphNode) -> Self {
        FeatureListEntry {
            id: n.id.clone(),
            title: title(n),
            status: text(n, "status"),
            priority: text(n, "priority"),
            problem: text(n, "problem"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.status),
            cell(&self.priority),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct JourneyListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    pub channel_count: usize,
    pub feature_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for JourneyListEntry {
    const KIND: &'static ListKind = &JOURNEYS;
    const HEADERS: &'static [&'static str] =
        &["id", "title", "persona", "channels", "features", "priority"];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        JourneyListEntry {
            id: n.id.clone(),
            title: title(n),
            persona: text(n, "persona"),
            channel_count: count_out(graph, &n.id, "channels"),
            feature_count: count_out(graph, &n.id, "features"),
            priority: text(n, "priority"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.persona),
            self.channel_count.to_string(),
            self.feature_count.to_string(),
            cell(&self.priority),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct DeliverableListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub journey_count: usize,
    pub module_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for DeliverableListEntry {
    const KIND: &'static ListKind = &DELIVERABLES;
    const HEADERS: &'static [&'static str] = &[
        "id",
        "title",
        "artifact_type",
        "status",
        "journeys",
        "modules",
    ];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        DeliverableListEntry {
            id: n.id.clone(),
            title: title(n),
            artifact_type: text(n, "artifact_type"),
            status: text(n, "status"),
            journey_count: count_out(graph, &n.id, "journeys"),
            module_count: count_out(graph, &n.id, "modules"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.artifact_type),
            cell(&self.status),
            self.journey_count.to_string(),
            self.module_count.to_string(),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct MilestoneListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    pub feature_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for MilestoneListEntry {
    const KIND: &'static ListKind = &MILESTONES;
    const HEADERS: &'static [&'static str] = &[
        "id",
        "title",
        "status",
        "target_date",
        "features",
        "priority",
    ];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        MilestoneListEntry {
            id: n.id.clone(),
            title: title(n),
            status: text(n, "status"),
            target_date: text(n, "target_date"),
            feature_count: count_out(graph, &n.id, "features"),
            priority: text(n, "priority"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.status),
            cell(&self.target_date),
            self.feature_count.to_string(),
            cell(&self.priority),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct ModuleListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub feature_count: usize,
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for ModuleListEntry {
    const KIND: &'static ListKind = &MODULES;
    const HEADERS: &'static [&'static str] = &["id", "title", "family", "features", "depends_on"];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        ModuleListEntry {
            id: n.id.clone(),
            title: title(n),
            family: text(n, "family"),
            feature_count: count_out(graph, &n.id, "features"),
            depends_on: sorted_dedup(targets(graph, &n.id, "depends_on")),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.family),
            self.feature_count.to_string(),
            if self.depends_on.is_empty() {
                "-".to_string()
            } else {
                self.depends_on.join(",")
            },
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct TermListEntry {
    pub id: String,
    pub title: String,
    pub definition: String,
    pub alias_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for TermListEntry {
    const KIND: &'static ListKind = &TERMS;
    const HEADERS: &'static [&'static str] = &["id", "title", "aliases", "definition"];
    fn of(_: &CommandGraph, n: &GraphNode) -> Self {
        TermListEntry {
            id: n.id.clone(),
            title: title(n),
            definition: text(n, "definition").unwrap_or_default(),
            alias_count: n.list("aliases").len(),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            self.alias_count.to_string(),
            self.definition.clone(),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct PersonaListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub journey_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for PersonaListEntry {
    const KIND: &'static ListKind = &PERSONAS;
    const HEADERS: &'static [&'static str] =
        &["id", "title", "technical_level", "status", "journeys"];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        PersonaListEntry {
            id: n.id.clone(),
            title: title(n),
            technical_level: text(n, "technical_level"),
            status: text(n, "status"),
            journey_count: count_in(graph, &n.id, "persona", "journey"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.technical_level),
            cell(&self.status),
            self.journey_count.to_string(),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct ChannelListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub journey_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for ChannelListEntry {
    const KIND: &'static ListKind = &CHANNELS;
    const HEADERS: &'static [&'static str] =
        &["id", "title", "interaction_model", "status", "journeys"];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        ChannelListEntry {
            id: n.id.clone(),
            title: title(n),
            interaction_model: text(n, "interaction_model"),
            status: text(n, "status"),
            journey_count: count_in(graph, &n.id, "channels", "journey"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.interaction_model),
            cell(&self.status),
            self.journey_count.to_string(),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct ReleaseListEntry {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub deliverable_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ListEntry for ReleaseListEntry {
    const KIND: &'static ListKind = &RELEASES;
    const HEADERS: &'static [&'static str] = &[
        "id",
        "title",
        "version",
        "status",
        "deliverables",
        "release_date",
    ];
    fn of(graph: &CommandGraph, n: &GraphNode) -> Self {
        ReleaseListEntry {
            id: n.id.clone(),
            title: title(n),
            version: text(n, "version"),
            status: text(n, "status"),
            deliverable_count: count_out(graph, &n.id, "deliverables"),
            release_date: text(n, "release_date"),
            tags: tags(n),
        }
    }
    fn row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.title.clone(),
            cell(&self.version),
            cell(&self.status),
            self.deliverable_count.to_string(),
            cell(&self.release_date),
        ]
    }
}

// ── Milestones and journeys ────────────────────────────────────────────────

/// `MilestoneCompletionPayload`: how many of the milestone's features are
/// done, as a count, a ratio in [0, 1] and their ids.
#[derive(Debug, Serialize)]
pub struct MilestoneCompletion {
    pub milestone_id: String,
    pub total_features: usize,
    pub done_count: usize,
    pub completion_ratio: f64,
    pub done_features: Vec<String>,
    /// The milestone's status and its features, for the human layout.
    #[serde(skip)]
    pub status: Option<String>,
    #[serde(skip)]
    pub features: Vec<FeatureStatus>,
}

#[derive(Debug)]
pub struct FeatureStatus {
    pub id: String,
    pub status: Option<String>,
}

/// The milestone's features (in declaration order) and which are done;
/// `None` when `milestone_id` is not a milestone.
pub fn milestone_completion(
    graph: &CommandGraph,
    milestone_id: &str,
) -> Option<MilestoneCompletion> {
    let node = of_kind(graph, milestone_id, "milestone")?;
    // The referenced features, in the order the milestone lists them.
    let mut referenced = targets(graph, milestone_id, "features");
    let declared = node.list("features");
    referenced.sort_by_key(|id| declared.iter().position(|d| d == id).unwrap_or(usize::MAX));
    let features: Vec<FeatureStatus> = referenced
        .iter()
        .filter_map(|id| graph.node(id))
        .map(|f| FeatureStatus {
            id: f.id.clone(),
            status: text(f, "status"),
        })
        .collect();
    let done_features: Vec<String> = features
        .iter()
        .filter(|f| f.status.as_deref() == Some("done"))
        .map(|f| f.id.clone())
        .collect();
    Some(MilestoneCompletion {
        milestone_id: milestone_id.to_string(),
        total_features: features.len(),
        done_count: done_features.len(),
        completion_ratio: ratio(done_features.len(), features.len()),
        done_features,
        status: text(node, "status"),
        features,
    })
}

/// `JourneyCoveragePayload`: how many of the journey's features are done,
/// and the ones that are not.
#[derive(Debug, Serialize)]
pub struct JourneyCoverage {
    pub journey_id: String,
    pub total_features: usize,
    pub covered_count: usize,
    pub uncovered_features: Vec<String>,
    /// The journey's persona, for the human layout.
    #[serde(skip)]
    pub persona: Option<String>,
}

/// How many of the journey's features have status `done` (a feature
/// without a status is uncovered); `None` when `journey_id` is not a
/// journey.
pub fn journey_coverage(graph: &CommandGraph, journey_id: &str) -> Option<JourneyCoverage> {
    let node = of_kind(graph, journey_id, "journey")?;
    let features = sorted_dedup(targets(graph, journey_id, "features"));
    let (covered, uncovered): (Vec<String>, Vec<String>) = features
        .into_iter()
        .partition(|f| graph.node(f).and_then(|n| n.text("status")) == Some("done"));
    Some(JourneyCoverage {
        journey_id: journey_id.to_string(),
        total_features: covered.len() + uncovered.len(),
        covered_count: covered.len(),
        uncovered_features: uncovered,
        persona: text(node, "persona"),
    })
}

// ── Features ───────────────────────────────────────────────────────────────

/// `FeatureImpactPayload`: what deferring or removing the feature touches.
#[derive(Debug, Serialize)]
pub struct FeatureImpact {
    pub feature_id: String,
    pub affected_journeys: Vec<String>,
    pub affected_milestones: Vec<String>,
    pub affected_deliverables: Vec<String>,
    pub affected_modules: Vec<String>,
    pub dependent_features: Vec<String>,
    pub total_affected_entities: usize,
}

/// The journeys, milestones and modules that list the feature, the
/// deliverables holding those journeys or modules, and the features that
/// depend on it, directly or through one another (only `depends_on`: a
/// feature listing it under `features` relates to it). Each sorted by id,
/// once; `None` when `feature_id` is not a feature.
pub fn feature_impact(graph: &CommandGraph, feature_id: &str) -> Option<FeatureImpact> {
    of_kind(graph, feature_id, "feature")?;
    let (via_journeys, via_modules) = deliverables_of_feature(graph, feature_id);
    let mut dependents: Vec<String> = Vec::new();
    let mut frontier = vec![feature_id.to_string()];
    while let Some(next) = frontier.pop() {
        for dependent in into(graph, &next, "depends_on", "feature") {
            if dependent != feature_id && !dependents.contains(&dependent) {
                dependents.push(dependent.clone());
                frontier.push(dependent);
            }
        }
    }
    let mut impact = FeatureImpact {
        feature_id: feature_id.to_string(),
        affected_journeys: into(graph, feature_id, "features", "journey"),
        affected_milestones: into(graph, feature_id, "features", "milestone"),
        affected_deliverables: sorted_dedup([via_journeys, via_modules].concat()),
        affected_modules: into(graph, feature_id, "features", "module"),
        dependent_features: sorted_dedup(dependents),
        total_affected_entities: 0,
    };
    // The kinds are disjoint, so the union is the sum.
    impact.total_affected_entities = impact.affected_journeys.len()
        + impact.affected_milestones.len()
        + impact.affected_deliverables.len()
        + impact.affected_modules.len()
        + impact.dependent_features.len();
    Some(impact)
}

/// `FeatureDependentPayload`: the features that declare `depends_on` the
/// feature, sorted by id.
#[derive(Debug, Serialize)]
pub struct FeatureDependents {
    pub feature_id: String,
    pub dependents: Vec<String>,
    pub count: usize,
}

/// The features that declare `depends_on` the feature; `None` when
/// `feature_id` is not a feature.
pub fn feature_dependents(graph: &CommandGraph, feature_id: &str) -> Option<FeatureDependents> {
    of_kind(graph, feature_id, "feature")?;
    let dependents = into(graph, feature_id, "depends_on", "feature");
    Some(FeatureDependents {
        feature_id: feature_id.to_string(),
        count: dependents.len(),
        dependents,
    })
}

/// `PersonaFeaturePayload`: the features of every journey the persona
/// undertakes, and those journeys.
#[derive(Debug, Serialize)]
pub struct PersonaFeatures {
    pub persona_id: String,
    pub features: Vec<String>,
    pub via_journey_ids: Vec<String>,
    pub count: usize,
}

/// The features of every journey the persona undertakes, sorted; `None`
/// when `persona_id` is not a persona.
pub fn persona_features(graph: &CommandGraph, persona_id: &str) -> Option<PersonaFeatures> {
    of_kind(graph, persona_id, "persona")?;
    let journeys = journeys_of_persona(graph, persona_id);
    let (features, via_journey_ids) = through_journeys(graph, journeys);
    Some(PersonaFeatures {
        persona_id: persona_id.to_string(),
        count: features.len(),
        features,
        via_journey_ids,
    })
}

/// `ChannelFeaturePayload`: the features of every journey that uses the
/// channel, and those journeys.
#[derive(Debug, Serialize)]
pub struct ChannelFeatures {
    pub channel_id: String,
    pub features: Vec<String>,
    pub via_journey_ids: Vec<String>,
    pub count: usize,
}

/// The features of every journey that uses the channel, sorted; `None`
/// when `channel_id` is not a channel.
pub fn channel_features(graph: &CommandGraph, channel_id: &str) -> Option<ChannelFeatures> {
    of_kind(graph, channel_id, "channel")?;
    let journeys = journeys_of_channel(graph, channel_id);
    let (features, via_journey_ids) = through_journeys(graph, journeys);
    Some(ChannelFeatures {
        channel_id: channel_id.to_string(),
        count: features.len(),
        features,
        via_journey_ids,
    })
}

/// The features `journeys` exercise, and the journeys, each sorted and
/// listed once.
fn through_journeys(graph: &CommandGraph, journeys: Vec<String>) -> (Vec<String>, Vec<String>) {
    let journeys = sorted_dedup(journeys);
    let features = journeys
        .iter()
        .flat_map(|j| targets(graph, j, "features"))
        .filter(|f| kind_of(graph, f) == Some("feature"))
        .collect();
    (sorted_dedup(features), journeys)
}

// ── Traceability ───────────────────────────────────────────────────────────

/// `DeliverableTraceabilityPayload`: every feature the deliverable reaches
/// through its journeys or its modules, and how many each path reaches.
#[derive(Debug, Serialize)]
pub struct DeliverableTraceability {
    pub deliverable_id: String,
    pub transitive_features: Vec<String>,
    pub journey_path_count: usize,
    pub module_path_count: usize,
}

/// The features `deliverable -> journeys -> features` and `deliverable ->
/// modules -> features` reach, once each; `None` when `deliverable_id` is
/// not a deliverable.
pub fn deliverable_traceability(
    graph: &CommandGraph,
    deliverable_id: &str,
) -> Option<DeliverableTraceability> {
    of_kind(graph, deliverable_id, "deliverable")?;
    let (via_journeys, via_modules) = features_of_deliverable(graph, deliverable_id);
    Some(DeliverableTraceability {
        deliverable_id: deliverable_id.to_string(),
        journey_path_count: via_journeys.len(),
        module_path_count: via_modules.len(),
        transitive_features: sorted_dedup([via_journeys, via_modules].concat()),
    })
}

/// `FeatureDeliverablePayload`: every deliverable that holds the feature
/// through a journey or a module, and how many each path reaches.
#[derive(Debug, Serialize)]
pub struct FeatureDeliverables {
    pub feature_id: String,
    pub deliverables: Vec<String>,
    pub via_journey_count: usize,
    pub via_module_count: usize,
}

/// The deliverables the reverse paths `feature <- journey <- deliverable`
/// and `feature <- module <- deliverable` reach, once each, sorted; `None`
/// when `feature_id` is not a feature.
pub fn feature_deliverables(graph: &CommandGraph, feature_id: &str) -> Option<FeatureDeliverables> {
    of_kind(graph, feature_id, "feature")?;
    let (via_journeys, via_modules) = deliverables_of_feature(graph, feature_id);
    Some(FeatureDeliverables {
        feature_id: feature_id.to_string(),
        via_journey_count: via_journeys.len(),
        via_module_count: via_modules.len(),
        deliverables: sorted_dedup([via_journeys, via_modules].concat()),
    })
}

/// `PersonaChannelPayload`: the channels of every journey the persona
/// undertakes.
#[derive(Debug, Serialize)]
pub struct PersonaChannels {
    pub persona_id: String,
    pub channels: Vec<String>,
    pub count: usize,
}

/// The channels `persona <- journey -> channels` reaches, sorted, once
/// each; `None` when `persona_id` is not a persona.
pub fn persona_channels(graph: &CommandGraph, persona_id: &str) -> Option<PersonaChannels> {
    of_kind(graph, persona_id, "persona")?;
    let channels = sorted_dedup(
        journeys_of_persona(graph, persona_id)
            .iter()
            .flat_map(|j| out(graph, j, "channels", "channel"))
            .collect(),
    );
    Some(PersonaChannels {
        persona_id: persona_id.to_string(),
        count: channels.len(),
        channels,
    })
}

/// `DeliverablePersonaPayload`: the personas the deliverable's journeys
/// target, and those journeys.
#[derive(Debug, Serialize)]
pub struct DeliverablePersonas {
    pub deliverable_id: String,
    pub personas: Vec<String>,
    pub via_journey_ids: Vec<String>,
    pub count: usize,
}

/// The personas `deliverable -> journeys -> persona` reaches, sorted, once
/// each, and the journeys on those paths (a journey without a persona is on
/// none); `None` when `deliverable_id` is not a deliverable.
pub fn deliverable_personas(
    graph: &CommandGraph,
    deliverable_id: &str,
) -> Option<DeliverablePersonas> {
    of_kind(graph, deliverable_id, "deliverable")?;
    let mut personas = Vec::new();
    let mut via_journey_ids = Vec::new();
    for journey in out(graph, deliverable_id, "journeys", "journey") {
        let targeted = out(graph, &journey, "persona", "persona");
        if !targeted.is_empty() {
            personas.extend(targeted);
            via_journey_ids.push(journey);
        }
    }
    let personas = sorted_dedup(personas);
    Some(DeliverablePersonas {
        deliverable_id: deliverable_id.to_string(),
        count: personas.len(),
        personas,
        via_journey_ids,
    })
}

/// The journeys that target `persona`: by the reference, or by the field
/// naming it.
fn journeys_of_persona(graph: &CommandGraph, persona: &str) -> Vec<String> {
    let by_field = graph
        .nodes_of_kind("journey")
        .filter(|j| j.text("persona") == Some(persona))
        .map(|j| j.id.clone());
    sorted_dedup(
        into(graph, persona, "persona", "journey")
            .into_iter()
            .chain(by_field)
            .collect(),
    )
}

/// The journeys that use `channel`.
fn journeys_of_channel(graph: &CommandGraph, channel: &str) -> Vec<String> {
    into(graph, channel, "channels", "journey")
}

/// The features `deliverable` reaches through its journeys, and through
/// its modules, each sorted, once each.
fn features_of_deliverable(graph: &CommandGraph, deliverable: &str) -> (Vec<String>, Vec<String>) {
    let through = |label: &str, kind: &str| {
        sorted_dedup(
            out(graph, deliverable, label, kind)
                .iter()
                .flat_map(|hop| out(graph, hop, "features", "feature"))
                .collect(),
        )
    };
    (through("journeys", "journey"), through("modules", "module"))
}

/// The deliverables that reach `feature` through a journey, and through a
/// module, each sorted, once each.
pub fn deliverables_of_feature(graph: &CommandGraph, feature: &str) -> (Vec<String>, Vec<String>) {
    let through = |kind: &str, label: &str| {
        sorted_dedup(
            into(graph, feature, "features", kind)
                .iter()
                .flat_map(|hop| into(graph, hop, label, "deliverable"))
                .collect(),
        )
    };
    (through("journey", "journeys"), through("module", "modules"))
}

// ── Project-wide ───────────────────────────────────────────────────────────

/// `BulkStatusPayload`: one entry per lifecycle kind with entities.
#[derive(Debug, Serialize)]
pub struct BulkStatus {
    pub kinds: Vec<KindStatusCounts>,
}

#[derive(Debug, Serialize)]
pub struct KindStatusCounts {
    pub kind: String,
    pub total: usize,
    pub by_status: Vec<StatusCount>,
}

#[derive(Debug, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: usize,
}

/// Per lifecycle kind present in the graph, how many entities have each
/// status (`(none)` for those without one).
pub fn bulk_status(graph: &CommandGraph) -> BulkStatus {
    let kinds = STATUS_KINDS
        .iter()
        .filter_map(|&kind| {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut total = 0;
            for node in graph.nodes_of_kind(kind) {
                total += 1;
                let status = text(node, "status").unwrap_or_else(|| "(none)".to_string());
                *counts.entry(status).or_insert(0) += 1;
            }
            (total > 0).then(|| KindStatusCounts {
                kind: kind.to_string(),
                total,
                by_status: counts
                    .into_iter()
                    .map(|(status, count)| StatusCount { status, count })
                    .collect(),
            })
        })
        .collect();
    BulkStatus { kinds }
}

/// `HealthPayload`: the 0 to 100 project health score and its counts.
#[derive(Debug, Serialize)]
pub struct HealthPayload {
    pub score: HealthScore,
    pub entity_counts: Vec<KindCount>,
    pub orphan_counts: Vec<KindOrphanCount>,
    pub completeness: HealthCompleteness,
}

#[derive(Debug, Serialize)]
pub struct HealthScore {
    pub overall: f64,
    pub coverage: f64,
    pub connectivity: f64,
    pub completeness: f64,
}

#[derive(Debug, Serialize)]
pub struct KindCount {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct KindOrphanCount {
    pub kind: String,
    pub orphans: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthCompleteness {
    pub features_with_status: usize,
    pub features_total: usize,
    pub milestones_with_features: usize,
    pub milestones_total: usize,
}

/// A 0-100 score averaging coverage (product entities something references),
/// connectivity (edge density) and completeness (features with a status,
/// milestones with references).
pub fn project_health(graph: &CommandGraph) -> HealthPayload {
    let mut entity_counts = Vec::new();
    let mut orphan_counts = Vec::new();
    let (mut total_entities, mut total_orphans) = (0usize, 0usize);
    for &kind in PRODUCT_KINDS {
        let count = graph.nodes_of_kind(kind).count();
        let orphans = graph
            .nodes_of_kind(kind)
            .filter(|n| graph.edges_to(&n.id).is_empty())
            .count();
        entity_counts.push(KindCount {
            kind: kind.to_string(),
            count,
        });
        if count > 0 {
            orphan_counts.push(KindOrphanCount {
                kind: kind.to_string(),
                orphans,
                total: count,
            });
        }
        total_entities += count;
        total_orphans += orphans;
    }

    let coverage = if total_entities > 0 {
        pct(total_entities - total_orphans, total_entities)
    } else {
        100.0
    };
    let connectivity = if total_entities > 1 {
        let max_edges = total_entities * (total_entities - 1);
        (graph.edges().len() as f64 / max_edges as f64).min(1.0) * 100.0
    } else {
        100.0
    };

    let features_total = graph.nodes_of_kind("feature").count();
    let features_with_status = graph
        .nodes_of_kind("feature")
        .filter(|n| n.has_field("status"))
        .count();
    let milestones_total = graph.nodes_of_kind("milestone").count();
    let milestones_with_features = graph
        .nodes_of_kind("milestone")
        .filter(|n| !graph.edges_from(&n.id).is_empty())
        .count();
    let completeness = if features_total + milestones_total > 0 {
        pct(
            features_with_status + milestones_with_features,
            features_total + milestones_total,
        )
    } else {
        100.0
    };

    HealthPayload {
        score: HealthScore {
            overall: (coverage + connectivity + completeness) / 3.0,
            coverage,
            connectivity,
            completeness,
        },
        entity_counts,
        orphan_counts,
        completeness: HealthCompleteness {
            features_with_status,
            features_total,
            milestones_with_features,
            milestones_total,
        },
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn text(node: &GraphNode, field: &str) -> Option<String> {
    node.text(field).map(str::to_string)
}

fn of_kind<'a>(graph: &'a CommandGraph, id: &str, kind: &str) -> Option<&'a GraphNode> {
    graph.node(id).filter(|n| n.kind == kind)
}

fn kind_of<'a>(graph: &'a CommandGraph, id: &str) -> Option<&'a str> {
    graph.node(id).map(|n| n.kind.as_str())
}

/// The targets of `id`'s references declared in `label`.
fn targets(graph: &CommandGraph, id: &str, label: &str) -> Vec<String> {
    graph
        .edges_from(id)
        .iter()
        .filter(|e| e.label == label)
        .map(|e| e.target.clone())
        .collect()
}

/// The `kind` entities `id` references in `label`, sorted, once each.
fn out(graph: &CommandGraph, id: &str, label: &str, kind: &str) -> Vec<String> {
    sorted_dedup(
        targets(graph, id, label)
            .into_iter()
            .filter(|t| kind_of(graph, t) == Some(kind))
            .collect(),
    )
}

/// The `source_kind` entities that reference `id` in `label`, sorted,
/// once each.
fn into(graph: &CommandGraph, id: &str, label: &str, source_kind: &str) -> Vec<String> {
    sorted_dedup(
        graph
            .edges_to(id)
            .iter()
            .filter(|e| e.label == label && kind_of(graph, &e.source) == Some(source_kind))
            .map(|e| e.source.clone())
            .collect(),
    )
}

/// How many of `id`'s references are declared in `label`.
fn count_out(graph: &CommandGraph, id: &str, label: &str) -> usize {
    graph
        .edges_from(id)
        .iter()
        .filter(|e| e.label == label)
        .count()
}

/// How many `source_kind` entities reference `id` in `label`.
fn count_in(graph: &CommandGraph, id: &str, label: &str, source_kind: &str) -> usize {
    graph
        .edges_to(id)
        .iter()
        .filter(|e| e.label == label && kind_of(graph, &e.source) == Some(source_kind))
        .count()
}

/// `ids` sorted, each once.
fn sorted_dedup(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}

/// `part / whole` in [0, 1]; 0 when `whole` is 0.
fn ratio(part: usize, whole: usize) -> f64 {
    if whole > 0 {
        part as f64 / whole as f64
    } else {
        0.0
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// The `ENTITY_NOT_FOUND` exit code.
pub const NOT_FOUND_EXIT: i32 = 1;

/// The `INVALID_INPUT` exit code: the one the host gives the usage errors
/// it catches itself.
pub const INVALID_INPUT_EXIT: i32 = 2;

/// `ProductSurfaceError` for an id that names no `kind`: the message names
/// both, and the suggestion is the nearest id of that kind, if any is
/// within edit distance 2.
pub fn not_found(graph: &CommandGraph, kind: &str, id: &str) -> CommandError {
    CommandError {
        entity_id: Some(id.to_string()),
        suggestion: suggest(graph, kind, id),
        ..CommandError::new("ENTITY_NOT_FOUND", format!("{kind} '{id}' not found"))
    }
}

/// `ProductSurfaceError` for an arg the command cannot use.
pub fn invalid_input(message: impl Into<String>) -> CommandError {
    CommandError::new("INVALID_INPUT", message)
}

/// The id of a `kind` entity nearest `id`, within Levenshtein distance 2;
/// the first by id among equally near ones.
pub fn suggest(graph: &CommandGraph, kind: &str, id: &str) -> Option<String> {
    graph
        .nodes_of_kind(kind)
        .map(|n| (levenshtein(&n.id, id), &n.id))
        .filter(|(distance, _)| *distance <= 2)
        .min()
        .map(|(_, near)| near.clone())
}

/// The edit distance between `a` and `b`, by character.
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

fn pct(part: usize, whole: usize) -> f64 {
    if whole > 0 {
        (part as f64 / whole as f64) * 100.0
    } else {
        0.0
    }
}

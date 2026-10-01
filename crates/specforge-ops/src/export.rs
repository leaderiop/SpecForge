//! Graph export: the one function behind `specforge export`, the MCP
//! `specforge.export` tool, `specforge.render` and the `specforge://graph`
//! resource (ADR 0004 D3-a).
//!
//! Every caller gets the same Graph Protocol document for the same request,
//! under the CLI's schema policy: a full `graph` export embeds the schema
//! (format 2.0), a scoped one references it (`schema_ref`), and `context`,
//! `brief` and any export under a token budget leave it out unless asked
//! for it. `dot` never carries a schema.

use crate::OpError;
use specforge_emitter::{EmitFormat, EmitOptions, GraphProtocolSchema, SchemaVersion, emit};
use specforge_graph::Graph;
use specforge_registry::{FieldRegistry, KindRegistry};
use std::str::FromStr;

/// An export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Graph,
    Context,
    Brief,
    Dot,
}

impl FromStr for Format {
    type Err = OpError;

    /// `graph` (alias `json`), `context`, `brief` or `dot`.
    fn from_str(name: &str) -> Result<Self, OpError> {
        match name {
            "graph" | "json" => Ok(Self::Graph),
            "context" => Ok(Self::Context),
            "brief" => Ok(Self::Brief),
            "dot" => Ok(Self::Dot),
            other => Err(OpError::new(
                "unknown_format",
                format!("Unknown format: {other}"),
            )),
        }
    }
}

impl Format {
    fn emit_format(self) -> EmitFormat {
        match self {
            Self::Graph => EmitFormat::Json,
            Self::Context => EmitFormat::Context,
            Self::Brief => EmitFormat::Brief,
            Self::Dot => EmitFormat::Dot,
        }
    }
}

/// Whether the export carries the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Schema {
    /// The policy: embed for a full `graph` export (a scoped one references
    /// it), leave it out for `context`, `brief` and under a token budget.
    #[default]
    Default,
    /// Attach it to every format but `dot` (`--with-schema`). Under a
    /// budget it counts toward the budget, and a budget it doesn't fit in
    /// fails with E062.
    With,
    /// Never attach it (`--no-schema`): Graph Protocol 1.0.
    Without,
}

/// The project an export reads.
#[derive(Clone, Copy)]
pub struct Project<'a> {
    pub graph: &'a Graph,
    pub kinds: &'a KindRegistry,
    pub fields: &'a FieldRegistry,
    /// The schema the project's extensions produce.
    pub schema: &'a GraphProtocolSchema,
}

/// What to export.
#[derive(Debug, Clone, Default)]
pub struct Request<'a> {
    pub format: Option<Format>,
    /// Restrict to the subgraph reachable from this entity.
    pub scope: Option<&'a str>,
    /// With `scope`, how far to traverse.
    pub depth: Option<usize>,
    /// Keep only nodes of these kinds (the scoped root always stays).
    pub kinds: Vec<&'a str>,
    /// Token budget: keep the most central entities that fit.
    pub max_tokens: Option<usize>,
    pub schema: Schema,
    /// A schema version to negotiate (same major as the produced schema).
    pub schema_version: Option<&'a str>,
}

impl Request<'_> {
    fn format(&self) -> Format {
        self.format.unwrap_or(Format::Graph)
    }

    /// Whether the export carries the schema, under the policy.
    pub fn attaches_schema(&self) -> bool {
        let format = self.format();
        if format == Format::Dot {
            return false;
        }
        match self.schema {
            Schema::Without => false,
            Schema::With => true,
            Schema::Default => format == Format::Graph && self.max_tokens.is_none(),
        }
    }
}

/// The export text for `request`.
pub fn export(project: &Project, request: &Request) -> Result<String, OpError> {
    let schema = if request.attaches_schema() {
        Some(negotiated(project.schema, request.schema_version)?)
    } else {
        None
    };
    let options = EmitOptions {
        format: request.format().emit_format(),
        scope: request.scope,
        depth: request.depth,
        kind_filter: request.kinds.clone(),
        schema: schema.as_ref(),
        token_budget: request.max_tokens,
        kind_registry: Some(project.kinds),
        field_registry: Some(project.fields),
    };
    emit(project.graph, &options).map_err(|e| {
        let message = e.to_string();
        OpError::new(leading_code(&message).unwrap_or("export_failed"), message)
    })
}

/// `schema`, at `requested` when one is asked for: the same major as the
/// produced schema, minor and patch from 0 up to the produced version.
fn negotiated(
    schema: &GraphProtocolSchema,
    requested: Option<&str>,
) -> Result<GraphProtocolSchema, OpError> {
    let mut schema = schema.clone();
    let Some(requested) = requested else {
        return Ok(schema);
    };
    let requested = requested.parse::<SchemaVersion>().map_err(|e| {
        OpError::new(
            "invalid_schema_version",
            format!("invalid schema version: {e}"),
        )
    })?;
    let max = schema.schema_version.clone();
    let min = SchemaVersion::new(max.major, 0, 0);
    specforge_emitter::negotiate_version(&requested, &min, &max)
        .map_err(|e| OpError::new("E027", e.to_string()))?;
    schema.schema_version = requested;
    Ok(schema)
}

/// The `E###`/`W###` code a message starts with (`"E062: ..."`).
fn leading_code(message: &str) -> Option<&'static str> {
    const CODES: [&str; 3] = ["E003", "E062", "E027"];
    CODES.into_iter().find(|code| message.starts_with(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(format: Format) -> Request<'static> {
        Request {
            format: Some(format),
            ..Request::default()
        }
    }

    #[test]
    fn format_names_accept_json_for_graph() {
        assert_eq!("graph".parse::<Format>(), Ok(Format::Graph));
        assert_eq!("json".parse::<Format>(), Ok(Format::Graph));
        assert_eq!("dot".parse::<Format>(), Ok(Format::Dot));
        assert_eq!(
            "yaml".parse::<Format>().unwrap_err().message,
            "Unknown format: yaml"
        );
    }

    #[test]
    fn the_policy_embeds_only_for_an_unbudgeted_graph() {
        assert!(request(Format::Graph).attaches_schema());
        assert!(!request(Format::Context).attaches_schema());
        assert!(!request(Format::Brief).attaches_schema());
        assert!(!request(Format::Dot).attaches_schema());
        let budgeted = Request {
            max_tokens: Some(100),
            ..request(Format::Graph)
        };
        assert!(!budgeted.attaches_schema());
    }

    #[test]
    fn with_and_without_override_the_policy_but_not_for_dot() {
        let with = |format| Request {
            schema: Schema::With,
            max_tokens: Some(100),
            ..request(format)
        };
        assert!(with(Format::Graph).attaches_schema());
        assert!(with(Format::Brief).attaches_schema());
        assert!(!with(Format::Dot).attaches_schema());
        let without = Request {
            schema: Schema::Without,
            ..request(Format::Graph)
        };
        assert!(!without.attaches_schema());
    }
}

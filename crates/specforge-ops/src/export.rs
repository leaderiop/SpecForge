//! Graph export: the one function behind `specforge export`, the MCP
//! `specforge.export` tool, `specforge.render` and the `specforge://graph`,
//! `context` and `brief` resources with their scoped forms and
//! `specforge://graph/{entity_id}` (ADR 0004 D3-a, ADR 0024 D4).
//!
//! Every caller gets the same Graph Protocol document for the same request,
//! under the CLI's schema policy: a full `graph` export embeds the schema
//! (format 2.0), a scoped one references it (`schema_ref`), and `context`,
//! `brief` and any export under a token budget leave it out unless asked
//! for it. `dot` never carries a schema.

use crate::options::{Choice, OptionTable};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};
use specforge_common::{Code, codes};
use specforge_emitter::{
    EmitFormat, EmitOptions, EmitterError, GraphProtocolSchema, SchemaVersion, emit,
};

/// An export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Graph,
    Context,
    Brief,
    Dot,
}

impl Format {
    /// The emitter format this export renders with.
    pub fn emit_format(self) -> EmitFormat {
        match self {
            Self::Graph => EmitFormat::Json,
            Self::Context => EmitFormat::Context,
            Self::Brief => EmitFormat::Brief,
            Self::Dot => EmitFormat::Dot,
        }
    }
}

const GRAPH: Choice<Format> = Choice {
    name: "graph",
    aliases: &["json"],
    help: "every entity with its fields, file and line (Graph Protocol)",
    value: Format::Graph,
};
const CONTEXT: Choice<Format> = Choice {
    name: "context",
    aliases: &[],
    help: "headline and normative fields and verify, for an agent",
    value: Format::Context,
};
const BRIEF: Choice<Format> = Choice {
    name: "brief",
    aliases: &[],
    help: "id, kind and title",
    value: Format::Brief,
};
const DOT: Choice<Format> = Choice {
    name: "dot",
    aliases: &[],
    help: "Graphviz",
    value: Format::Dot,
};

/// `specforge export --format`, `specforge.render`'s `format`.
pub const FORMAT: OptionTable<Format> = OptionTable {
    argument: "format",
    choices: &[GRAPH, CONTEXT, BRIEF, DOT],
    default: Some(Format::Graph),
};

/// The formats an agent reads and a published schema describes:
/// `specforge.export`, `specforge.query`, `specforge schema --publish
/// --format`.
pub const AGENT_FORMAT: OptionTable<Format> = OptionTable {
    argument: "format",
    choices: &[GRAPH, CONTEXT, BRIEF],
    default: Some(Format::Graph),
};

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
        self.format
            .or(FORMAT.default)
            .expect("the export format has a default")
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

/// The export text of the view's project for `request`. A schema it
/// carries is the view's versioned schema (`specforge export`'s version,
/// computed against the root's schema cache, which this only reads).
pub fn export(view: &ProjectView, request: &Request) -> Result<String, OpError> {
    let schema = if request.attaches_schema() {
        Some(negotiated(view.versioned_schema(), request.schema_version)?)
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
        kind_registry: Some(&view.registries().kinds),
        field_registry: Some(&view.registries().fields),
    };
    emit(view.graph(), &options).map_err(|error| failure(error, request.scope))
}

/// The emitter's failure as the operation's: what kind it is is decided by
/// the variant, and a diagnostic code its message leads with (`E003`,
/// `E062`) is the failure's code, not text of its message.
fn failure(error: EmitterError, scope: Option<&str>) -> OpError {
    match error {
        EmitterError::EntityNotFound(message) => {
            let error = OpError::coded(
                OpErrorKind::EntityNotFound,
                codes::E003,
                without_code(&message, codes::E003),
            );
            match scope {
                Some(scope) => error.with_entity(scope),
                None => error,
            }
        }
        EmitterError::Other(message) | EmitterError::InvalidScope(message) => {
            if let Some(rest) = message.strip_prefix(&format!("{}: ", codes::E062)) {
                OpError::coded(OpErrorKind::InvalidInput, codes::E062, rest)
            } else {
                OpError::new(OpErrorKind::InvalidInput, "export_failed", message)
            }
        }
        EmitterError::SerializationError(message) => {
            OpError::new(OpErrorKind::Internal, "export_failed", message)
        }
    }
}

/// `message` without the `"{code}: "` it leads with.
fn without_code(message: &str, code: Code) -> String {
    message
        .strip_prefix(code.id())
        .and_then(|rest| rest.strip_prefix(": "))
        .unwrap_or(message)
        .to_string()
}

/// `schema`, at `requested` when one is asked for: the same major as the
/// produced schema, minor and patch from 0 up to the produced version.
fn negotiated(
    mut schema: GraphProtocolSchema,
    requested: Option<&str>,
) -> Result<GraphProtocolSchema, OpError> {
    let Some(requested) = requested else {
        return Ok(schema);
    };
    let requested = requested.parse::<SchemaVersion>().map_err(|e| {
        OpError::new(
            OpErrorKind::SchemaMismatch,
            "invalid_schema_version",
            format!("invalid schema version: {e}"),
        )
    })?;
    let max = schema.schema_version.clone();
    let min = SchemaVersion::new(max.major, 0, 0);
    specforge_emitter::negotiate_version(&requested, &min, &max).map_err(|e| {
        OpError::coded(
            OpErrorKind::Conflict,
            codes::E027,
            without_code(&e.to_string(), codes::E027),
        )
    })?;
    schema.schema_version = requested;
    Ok(schema)
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
        assert_eq!(FORMAT.parse("graph"), Ok(Format::Graph));
        assert_eq!(FORMAT.parse("json"), Ok(Format::Graph));
        assert_eq!(FORMAT.parse("dot"), Ok(Format::Dot));
        let error = FORMAT.parse("yaml").unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert_eq!(
            error.message,
            "Unknown format: yaml. Expected: graph, context, brief, dot"
        );
        assert_eq!(
            AGENT_FORMAT.parse("dot").unwrap_err().message,
            "Unknown format: dot. Expected: graph, context, brief"
        );
    }

    fn failed(request: &Request) -> OpError {
        let fixture = crate::view::testing::Fixture::new();
        export(&fixture.view(), request).unwrap_err()
    }

    #[test]
    fn export_failures_carry_their_kind() {
        let error = failed(&Request {
            scope: Some("ghost"),
            ..request(Format::Graph)
        });
        assert_eq!(error.kind, OpErrorKind::EntityNotFound);
        assert_eq!(error.code, "E003");
        assert_eq!(error.entity.as_deref(), Some("ghost"));
        assert!(!error.message.starts_with("E003"), "{}", error.message);
        assert!(error.message.contains("'ghost'"), "{}", error.message);

        let error = failed(&Request {
            max_tokens: Some(1),
            ..request(Format::Graph)
        });
        assert_eq!(error.kind, OpErrorKind::InvalidInput);
        assert_eq!(error.code, "E062");
        assert!(
            error.message.starts_with("the token budget"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_message_loses_the_code_it_leads_with() {
        assert_eq!(
            without_code("E003: no such entity", codes::E003),
            "no such entity"
        );
        assert_eq!(
            without_code("no such entity", codes::E003),
            "no such entity"
        );
        assert_eq!(without_code("E0031: odd", codes::E003), "E0031: odd");
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

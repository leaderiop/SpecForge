//! The core tools: one entry per tool holds its name, description, output
//! schema and effect (what it does, with its handler and its arguments; a
//! mutation's handler says what it wrote, ADR 0022). A tool's category,
//! annotations, call target and input schema derive from it (ADR 0024, ADR
//! 0033). The listing, dispatch and events all derive from it.

use serde_json::json;

use super::*;
use crate::args::NoArgs;
use crate::target::ProjectTarget;
use crate::tool::{Effect, Handler, MutationHandler, ToolGroup, ToolSpec, WriteHints};

/// A handler that reads no project, given its typed arguments only: refused
/// when they don't parse. `Args => Reply` answers a typed `Answered<Reply>`
/// (its outputSchema is the reply's); `text` answers `Answered<Text>`;
/// without either the handler returns an outcome, or `Handled` to use `?`
/// (a tool not yet typed).
macro_rules! unscoped {
    (text $handler:path, $args:ty) => {
        Handler::Unscoped {
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |_, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::reply::text($handler(args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty => $reply:ty) => {
        Handler::Unscoped {
            arguments: <$args as crate::args::Arguments>::declared,
            reply: Some(crate::reply::output_schema::<$reply>),
            run: |_, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::reply::structured::<$reply>($handler(args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty) => {
        Handler::Unscoped {
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |_, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A handler given the project view of its target (the empty session's with
/// nothing served) and its typed arguments: refused when they don't parse.
/// `Args => Reply`, `text` and the untyped form are as for `unscoped!`.
macro_rules! view {
    (text $handler:path, $args:ty, $target:expr) => {
        Handler::View {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::reply::text($handler(call.view(), args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty => $reply:ty, $target:expr) => {
        Handler::View {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: Some(crate::reply::output_schema::<$reply>),
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::reply::structured::<$reply>($handler(call.view(), args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty, $target:expr) => {
        Handler::View {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => crate::tool::IntoOutcome::into_outcome($handler(call.view(), args)),
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A handler given the project it acts on and its typed arguments: refused
/// when they don't parse. `Args => Reply`, `text` and the untyped form are
/// as for `unscoped!`.
macro_rules! project {
    (text $handler:path, $args:ty, $target:expr) => {
        Handler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                // The call target refused a call with no project before
                // this runs.
                Ok(args) => match call.project() {
                    Ok(project) => crate::reply::text($handler(&project, args)),
                    Err(refused) => refused.into(),
                },
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty => $reply:ty, $target:expr) => {
        Handler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: Some(crate::reply::output_schema::<$reply>),
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => match call.project() {
                    Ok(project) => crate::reply::structured::<$reply>($handler(&project, args)),
                    Err(refused) => refused.into(),
                },
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
    ($handler:path, $args:ty, $target:expr) => {
        Handler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: None,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                // The call target refused a call with no project before
                // this runs.
                Ok(args) => match call.project() {
                    Ok(project) => crate::tool::IntoOutcome::into_outcome($handler(&project, args)),
                    Err(refused) => refused.into(),
                },
                Err(refused) => crate::tool::ToolOutcome::Refused(refused),
            },
        }
    };
}

/// A mutation handler given the project it writes and its typed arguments:
/// a failed mutation when they don't parse (arguments that do not parse
/// cannot say they asked for a preview). The handler answers a typed
/// `Mutation<Reply>`.
macro_rules! mutation {
    ($handler:path, $args:ty => $reply:ty, $target:expr) => {
        MutationHandler::Project {
            target: $target,
            arguments: <$args as crate::args::Arguments>::declared,
            reply: crate::mutation::output_schema::<$reply>,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => match call.project() {
                    Ok(project) => crate::mutation::replied::<$reply>($handler(&project, args)),
                    Err(refused) => crate::mutation::Replied::refused(refused),
                },
                Err(refused) => {
                    crate::mutation::Replied::refused(crate::tool::ToolOutcome::Refused(refused))
                }
            },
        }
    };
}

/// A mutation handler given the directory it creates a project in, the
/// runtime its extensions' declarations are read in, and its typed
/// arguments. It answers a typed `Mutation<Reply>`, like `mutation!`.
macro_rules! create {
    ($handler:path, $args:ty => $reply:ty) => {
        MutationHandler::New {
            arguments: <$args as crate::args::Arguments>::declared,
            reply: crate::mutation::output_schema::<$reply>,
            run: |call, arguments| match crate::args::read::<$args>(&arguments) {
                Ok(args) => match call.new_project_dir() {
                    Some(dir) => {
                        crate::mutation::replied::<$reply>($handler(dir, &call.runtime(), args))
                    }
                    // The call target refused a call without a path.
                    None => crate::mutation::Replied::refused(crate::tool::McpError::from(
                        crate::target::TargetError::PathRequired,
                    )),
                },
                Err(refused) => {
                    crate::mutation::Replied::refused(crate::tool::ToolOutcome::Refused(refused))
                }
            },
        }
    };
}

pub static CORE_TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "specforge.query",
        description: "Query the graph at multiple resolutions",
        output: Some(
            || json!({ "type": "object", "properties": { "nodes": { "type": "array" }, "edges": { "type": "array" }, "schema_version": { "type": "string" }, "format_version": { "type": "string" } }, "required": ["nodes", "edges"] }),
        ),
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(query::call, query::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.validate",
        description: "Recompile and validate the spec project",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(text
                validate::call,
                validate::Args,
                ProjectTarget::ANY_UNLESS_CACHED
            ),
        },
    },
    ToolSpec {
        name: "specforge.analyze",
        description: "Run analysis passes (coverage: proof obligations and discharge funnel; contracts: clause symmetry) over the compiled project",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(
                analyze::call,
                analyze::Args => analyze::Reply,
                ProjectTarget::ANY_UNLESS_CACHED
            ),
        },
    },
    ToolSpec {
        name: "specforge.export",
        description: "Export the graph in various formats",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(text export::call, export::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.trace",
        description: "Show traceability chain for an entity, or check an agent plan for gaps (entity_id or plan)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(trace::call, trace::Args => trace::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.search",
        description: "Find entities by id, title or string field text, ranked as the LSP ranks them (exact, prefix, substring, field text, then fuzzy)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(search::call, search::Args => search::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.explain",
        description: "Explain a diagnostic code: its title, owner, level, what triggers it and how to fix it, and its docs link",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: unscoped!(explain::call, explain::Args => explain::Reply),
        },
    },
    ToolSpec {
        name: "specforge.schema",
        description: "Get the GraphProtocolSchema: entity kinds with their typed fields, edge types and the loaded extensions",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(schema::call, schema::Args => schema::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.model",
        description: "Render the logical data model (entity kinds, fields, relationships)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(text model::call, model::Args, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.outline_extensions",
        description: "Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes.",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(
                text outline_extensions::call,
                outline_extensions::Args,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.coverage",
        description: "Get coverage status per entity",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(coverage::call, coverage::Args => coverage::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.stats",
        description: "Get project statistics",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(stats::call, NoArgs => stats::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.list",
        description: "List entities sorted by id, optionally filtered by kind and field values, and paged",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: view!(list::call, list::Args => list::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.inspect",
        description: "Get full detail for a specific entity",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(inspect::call, inspect::Args => inspect::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.find_definition",
        description: "Find the source location of an entity definition",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                find_definition::call,
                find_definition::Args => find_definition::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_references",
        description: "Find the references to an entity: each place another entity's field names it, as the identifier token (what an IDE's find-references shows)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                find_references::call,
                find_references::Args => find_references::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.outline",
        description: "Get entity outline for a file",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(outline::call, outline::Args => outline::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.suggest_fixes",
        description: "The fixes the LSP offers as code actions for the project's diagnostics and entities, each with its edits",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: view!(
                suggest_fixes::call,
                suggest_fixes::Args => suggest_fixes::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.format",
        description: "Format spec files",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(format::call, format::Args => format::Reply, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.rename",
        description: "Rename an entity across all files",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(rename::call, rename::Args => rename::Reply, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.init",
        description: "Initialize a new SpecForge project",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: false,
                idempotent: true,
                open_world: false,
            },
            handler: create!(init::call, init::Args => init::Reply),
        },
    },
    ToolSpec {
        name: "specforge.add_extension",
        description: "Install an extension",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: true,
            },
            handler: mutation!(add_extension::call, add_extension::Args => add_extension::Reply, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.remove_extension",
        description: "Remove an installed extension",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(
                remove_extension::call,
                remove_extension::Args => remove_extension::Reply,
                ProjectTarget::ANY
            ),
        },
    },
    ToolSpec {
        name: "specforge.migrate",
        description: "Run migration pipeline",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: mutation!(migrate::call, migrate::Args => migrate::Reply, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.extensions",
        description: "List installed extensions",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(extensions::call, NoArgs => extensions::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.providers",
        description: "List configured providers",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(providers::call, NoArgs => providers::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.doctor",
        description: "Run health checks",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Management,
            handler: project!(doctor::call, NoArgs => doctor::Reply, ProjectTarget::SERVED_UNLESS_CACHED),
        },
    },
    ToolSpec {
        name: "specforge.collect",
        description: "Record which entities the project's tests prove, from the test runner's report (runs the runner only with run: true and prior approval)",
        output: None,
        effect: Effect::WritesOutput {
            group: ToolGroup::Management,
            hints: WriteHints {
                destructive: true,
                idempotent: false,
                open_world: true,
            },
            handler: project!(collect::call, collect::Args => collect::Reply, ProjectTarget::ANY),
        },
    },
    ToolSpec {
        name: "specforge.render",
        description: "Render output in a specified format",
        output: None,
        effect: Effect::WritesOutput {
            group: ToolGroup::Management,
            hints: WriteHints {
                destructive: true,
                idempotent: true,
                open_world: false,
            },
            handler: view!(render::call, render::Args => render::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_progress",
        description: "Check inference progress: summary of analyzed vs unanalyzed source files, stale entries, and entity counts",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(infer_progress::call, NoArgs => infer_progress::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_gaps",
        description: "Analyze inference gaps: public Rust items not yet covered by spec entities (approximate)",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Core,
            handler: project!(infer_gaps::call, NoArgs => infer_gaps::Reply, ProjectTarget::SERVED),
        },
    },
    ToolSpec {
        name: "specforge.infer_session",
        description: "Manage inference sessions: start a new session, mark files as analyzed, or end a session",
        output: None,
        effect: Effect::Mutates {
            hints: WriteHints {
                destructive: true,
                idempotent: false,
                open_world: false,
            },
            handler: mutation!(
                infer_session::call,
                infer_session::Args => infer_session::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_implementation",
        description: "Find source code locations that implement a specforge entity",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: project!(
                find_implementation::call,
                find_implementation::Args => find_implementation::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
    ToolSpec {
        name: "specforge.find_spec_for_source",
        description: "Find specforge entities anchored to a source file",
        output: None,
        effect: Effect::Reads {
            group: ToolGroup::Navigation,
            handler: project!(
                find_spec_for_source::call,
                find_spec_for_source::Args => find_spec_for_source::Reply,
                ProjectTarget::SERVED
            ),
        },
    },
];

//! `specforge model`/`specforge.model` and `specforge outline`/
//! `specforge.outline_extensions`: the logical data model and the extension
//! architecture diagrams, each one operation over the project view (ADR
//! 0015). Each enumerated argument is an option table here (ADR 0027), so
//! both surfaces list, accept, default and refuse the same names.

use specforge_common::{Diagnostic, codes};
use specforge_emitter::GraphProtocolSchema;
use specforge_emitter::model::{ModelIntermediate_from_schema, filter_entities, filter_fields};
use specforge_emitter::outline::OutlineIntermediate_from_declarations;
use specforge_protocol_types::ExtensionDeclaration;

// The value types, so surfaces name ops rather than the emitter.
pub use specforge_emitter::model::{FieldLevel, GroupBy, ModelFormat, ModelOptions};
pub use specforge_emitter::outline::{
    DependencyDepth, OutlineDetail, OutlineFormat, OutlineOptions,
};

use crate::options::{Choice, OptionTable};
use crate::view::ProjectView;

/// A rendered model diagram and what rendering it reported.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelOutcome {
    pub rendered: String,
    /// W146: a field type the model does not know, rendered as a string.
    pub warnings: Vec<Diagnostic>,
}

/// The logical data model of the view's extensions, as `options` asks.
pub fn model(view: &ProjectView, options: &ModelOptions) -> ModelOutcome {
    render_schema(&view.schema(), view.registries().declarations(), options)
}

/// The model of `schema`, themed by `declarations`: what [`model`] renders
/// for a view, for any Graph Protocol schema.
pub fn render_schema(
    schema: &GraphProtocolSchema,
    declarations: &[ExtensionDeclaration],
    options: &ModelOptions,
) -> ModelOutcome {
    let model = ModelIntermediate_from_schema(schema).with_theme_colors(declarations);
    let warnings = model
        .warnings
        .iter()
        .map(|warning| Diagnostic::new(codes::W146, format!("model: {warning}")))
        .collect();
    let model = filter_entities(&model, options);
    let model = filter_fields(&model, options.fields);
    ModelOutcome {
        rendered: specforge_emitter::model::render(&model, options),
        warnings,
    }
}

/// The architecture of the view's extensions (dependencies, enhancements,
/// contributions), as `options` asks.
pub fn outline(view: &ProjectView, options: &OutlineOptions) -> String {
    let outline = OutlineIntermediate_from_declarations(view.registries().declarations());
    specforge_emitter::outline::render(&outline, options)
}

/// A choice whose name says what it selects.
const fn plain<T>(name: &'static str, value: T) -> Choice<T> {
    Choice {
        name,
        aliases: &[],
        help: "",
        value,
    }
}

/// A choice with a one-line help.
const fn helped<T>(name: &'static str, help: &'static str, value: T) -> Choice<T> {
    Choice {
        name,
        aliases: &[],
        help,
        value,
    }
}

/// `specforge model --format`, `specforge.model`'s `format`.
pub const MODEL_FORMAT: OptionTable<ModelFormat> = OptionTable {
    argument: "format",
    code: "unknown_format",
    choices: &[
        plain("markdown", ModelFormat::Markdown),
        helped("mermaid", "ER diagram", ModelFormat::Mermaid),
        helped("dot", "Graphviz", ModelFormat::Dot),
        plain("json", ModelFormat::Json),
        helped("dbml", "dbdiagram.io", ModelFormat::Dbml),
    ],
    default: Some(ModelFormat::Markdown),
};

/// `specforge model --group-by`, `specforge.model`'s `group_by`.
pub const GROUP_BY: OptionTable<GroupBy> = OptionTable {
    argument: "group_by",
    code: "invalid_input",
    choices: &[
        helped(
            "extension",
            "under a header per extension",
            GroupBy::Extension,
        ),
        helped("none", "one flat list", GroupBy::None),
    ],
    default: Some(GroupBy::Extension),
};

/// `specforge model --fields`, `specforge.model`'s `fields`.
pub const MODEL_FIELDS: OptionTable<FieldLevel> = OptionTable {
    argument: "fields",
    code: "invalid_input",
    choices: &[
        plain("none", FieldLevel::None),
        helped("keys", "key fields", FieldLevel::Keys),
        plain("all", FieldLevel::All),
    ],
    default: Some(FieldLevel::Keys),
};

/// `specforge outline --format`, `specforge.outline_extensions`' `format`.
pub const OUTLINE_FORMAT: OptionTable<OutlineFormat> = OptionTable {
    argument: "format",
    code: "unknown_format",
    choices: &[
        plain("markdown", OutlineFormat::Markdown),
        helped("mermaid", "flowchart", OutlineFormat::Mermaid),
        helped("dot", "Graphviz", OutlineFormat::Dot),
        plain("json", OutlineFormat::Json),
    ],
    default: Some(OutlineFormat::Markdown),
};

/// `specforge outline --fields`, `specforge.outline_extensions`' `fields`.
pub const OUTLINE_FIELDS: OptionTable<OutlineDetail> = OptionTable {
    argument: "fields",
    code: "invalid_input",
    choices: &[
        helped("none", "counts only", OutlineDetail::None),
        helped("keys", "names and rule codes", OutlineDetail::Keys),
        helped("all", "full field attribution", OutlineDetail::All),
    ],
    default: Some(OutlineDetail::Keys),
};

/// `specforge outline --deps`, `specforge.outline_extensions`' `deps`.
pub const DEPS: OptionTable<DependencyDepth> = OptionTable {
    argument: "deps",
    code: "invalid_input",
    choices: &[
        helped("direct", "declared only", DependencyDepth::Direct),
        helped(
            "effective",
            "direct and used transitive",
            DependencyDepth::Effective,
        ),
        helped("full", "all transitive", DependencyDepth::Full),
    ],
    default: Some(DependencyDepth::Direct),
};

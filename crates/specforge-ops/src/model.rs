//! `specforge model`/`specforge.model` and `specforge outline`/
//! `specforge.outline_extensions`: the logical data model and the extension
//! architecture diagrams, each one operation over the project view (ADR
//! 0015). Argument names parse here ([`Named`]), so both surfaces accept
//! and refuse the same values with the same messages.

use std::str::FromStr;

use specforge_common::Diagnostic;
use specforge_emitter::GraphProtocolSchema;
use specforge_emitter::model::{
    FieldLevel, GroupBy, ModelFormat, ModelIntermediate_from_schema, ModelOptions, filter_entities,
    filter_fields,
};
use specforge_emitter::outline::{
    DependencyDepth, OutlineDetail, OutlineFormat, OutlineIntermediate_from_declarations,
    OutlineOptions,
};
use specforge_protocol_types::ExtensionDeclaration;

use crate::OpError;
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
    render_schema(&view.schema(), view.registries.declarations(), options)
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
        .map(|warning| Diagnostic::warning("W146", format!("model: {warning}")))
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
    let outline = OutlineIntermediate_from_declarations(view.registries.declarations());
    specforge_emitter::outline::render(&outline, options)
}

/// A value named by an argument (`"mermaid"`, `"keys"`, ...), parsed with
/// `str::parse`: `Named<ModelFormat>`, `Named<GroupBy>`, `Named<FieldLevel>`,
/// `Named<OutlineFormat>`, `Named<OutlineDetail>`, `Named<DependencyDepth>`.
/// Any other name is an error listing the accepted ones (`unknown_format`
/// for a format, `invalid_input` otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Named<T>(pub T);

/// `impl FromStr for Named<$type>`: `$argument` names the argument in the
/// refusal, `$code` is its code, then each accepted name and its value.
macro_rules! named {
    ($type:ty, $argument:literal, $code:literal, [$(($name:literal, $value:expr)),+ $(,)?]) => {
        impl FromStr for Named<$type> {
            type Err = OpError;

            fn from_str(name: &str) -> Result<Self, OpError> {
                match name {
                    $($name => Ok(Named($value)),)+
                    other => Err(OpError::new(
                        $code,
                        format!(
                            concat!("Unknown ", $argument, ": {}. Expected: {}"),
                            other,
                            [$($name),+].join(", ")
                        ),
                    )),
                }
            }
        }
    };
}

named!(
    ModelFormat,
    "format",
    "unknown_format",
    [
        ("markdown", ModelFormat::Markdown),
        ("mermaid", ModelFormat::Mermaid),
        ("dot", ModelFormat::Dot),
        ("json", ModelFormat::Json),
        ("dbml", ModelFormat::Dbml),
    ]
);
named!(
    GroupBy,
    "group_by",
    "invalid_input",
    [("extension", GroupBy::Extension), ("none", GroupBy::None),]
);
named!(
    FieldLevel,
    "fields",
    "invalid_input",
    [
        ("none", FieldLevel::None),
        ("keys", FieldLevel::Keys),
        ("all", FieldLevel::All),
    ]
);
named!(
    OutlineFormat,
    "format",
    "unknown_format",
    [
        ("markdown", OutlineFormat::Markdown),
        ("mermaid", OutlineFormat::Mermaid),
        ("dot", OutlineFormat::Dot),
        ("json", OutlineFormat::Json),
    ]
);
named!(
    OutlineDetail,
    "fields",
    "invalid_input",
    [
        ("none", OutlineDetail::None),
        ("keys", OutlineDetail::Keys),
        ("all", OutlineDetail::All),
    ]
);
named!(
    DependencyDepth,
    "deps",
    "invalid_input",
    [
        ("direct", DependencyDepth::Direct),
        ("effective", DependencyDepth::Effective),
        ("full", DependencyDepth::Full),
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_and_others_list_the_accepted_ones() {
        assert_eq!(
            "dbml".parse::<Named<ModelFormat>>(),
            Ok(Named(ModelFormat::Dbml))
        );
        assert_eq!(
            "full".parse::<Named<DependencyDepth>>(),
            Ok(Named(DependencyDepth::Full))
        );
        let error = "svg".parse::<Named<ModelFormat>>().unwrap_err();
        assert_eq!(error.code, "unknown_format");
        assert_eq!(
            error.message,
            "Unknown format: svg. Expected: markdown, mermaid, dot, json, dbml"
        );
        let error = "both".parse::<Named<GroupBy>>().unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert_eq!(
            error.message,
            "Unknown group_by: both. Expected: extension, none"
        );
        assert_eq!(
            "x".parse::<Named<DependencyDepth>>().unwrap_err().message,
            "Unknown deps: x. Expected: direct, effective, full"
        );
    }
}

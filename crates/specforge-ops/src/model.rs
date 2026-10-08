//! `specforge model`/`specforge.model` and `specforge outline`/
//! `specforge.outline_extensions`: the logical data model and the extension
//! architecture diagrams, each one operation over the project view (ADR
//! 0015). Each enumerated argument is an option table here (ADR 0027), so
//! both surfaces list, accept, default and refuse the same names.

// The value types, so surfaces name ops rather than the emitter.
pub use specforge_emitter::model::{FieldLevel, GroupBy, ModelFormat, ModelOptions};
pub use specforge_emitter::outline::{
    DependencyDepth, OutlineDetail, OutlineFormat, OutlineOptions,
};

use crate::options::{Choice, OptionTable};
use crate::view::ProjectView;

/// The logical data model of the view's extensions, as `options` asks.
pub fn model(view: &ProjectView, options: &ModelOptions) -> String {
    specforge_emitter::model::export(&view.schema(), view.registries().declarations(), options)
}

/// The architecture of the view's extensions (dependencies, enhancements,
/// contributions), as `options` asks.
pub fn outline(view: &ProjectView, options: &OutlineOptions) -> String {
    specforge_emitter::outline::export(view.registries().declarations(), options)
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

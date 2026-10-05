//! The core prompts: one entry per prompt holds its name, description,
//! typed arguments and renderer. The listing and the dispatch derive from
//! it, as the tool table's do.

use super::{context, explore, infer, review, trace};
use crate::prompt::{Layout, PromptArgs, PromptSpec};
use crate::target::TargetSpec;

/// The Prompt spec of the prompt `$module` renders: its listing derived
/// from `$module::Args`, its renderer reading them (refused when they
/// don't parse).
macro_rules! prompt {
    ($name:literal, $description:literal, $module:ident, $layout:expr) => {
        PromptSpec {
            name: $name,
            description: $description,
            arguments: crate::prompt::arguments::<$module::Args>,
            fields: crate::args::fields::<$module::Args>,
            descriptions: <$module::Args as PromptArgs>::DESCRIPTIONS,
            target: TargetSpec::SERVED,
            layout: $layout,
            render: |call, arguments| {
                $module::render(call, crate::args::parse_args::<$module::Args>(arguments)?)
            },
        }
    };
}

/// The core prompts, in listing order.
pub static CORE_PROMPTS: &[PromptSpec] = &[
    prompt!(
        "specforge://prompts/context",
        "Get structured context for implementing an entity",
        context,
        Layout::Turns
    ),
    prompt!(
        "specforge://prompts/review",
        "Analyze coverage gaps for an entity or the whole graph",
        review,
        Layout::Turns
    ),
    prompt!(
        "specforge://prompts/trace",
        "Identify traceability gaps for a plan",
        trace,
        Layout::Turns
    ),
    prompt!(
        "specforge://prompts/explore",
        "Discover exploration starting points in the graph",
        explore,
        Layout::Turns
    ),
    prompt!(
        "specforge://prompts/infer",
        "Get inference guidance for discovering spec entities from code",
        infer,
        Layout::Inline
    ),
];

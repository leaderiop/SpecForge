//! The core prompts: one entry per prompt holds its name, description,
//! typed arguments and renderer. The listing and the dispatch derive from
//! it, as the tool table's do.

use super::{context, explore, infer, review, trace};
use crate::prompt::PromptSpec;
use crate::target::TargetSpec;

/// The Prompt spec of the prompt `$module` renders: its listing and its
/// renderer's reading both derived from `$module::Args` (refused when they
/// don't read).
macro_rules! prompt {
    ($name:literal, $description:literal, $module:ident) => {
        PromptSpec {
            name: $name,
            description: $description,
            arguments: <$module::Args as crate::args::Arguments>::declared,
            target: TargetSpec::SERVED_VIEW,
            render: |call, arguments| {
                $module::render(call, crate::args::read::<$module::Args>(&arguments)?)
            },
        }
    };
}

/// The core prompts, in listing order.
pub static CORE_PROMPTS: &[PromptSpec] = &[
    prompt!(
        "specforge://prompts/context",
        "Get structured context for implementing an entity",
        context
    ),
    prompt!(
        "specforge://prompts/review",
        "Analyze coverage gaps for an entity or the whole graph",
        review
    ),
    prompt!(
        "specforge://prompts/trace",
        "Identify traceability gaps for a plan",
        trace
    ),
    prompt!(
        "specforge://prompts/explore",
        "Discover exploration starting points in the graph",
        explore
    ),
    prompt!(
        "specforge://prompts/infer",
        "Get inference guidance for discovering spec entities from code",
        infer
    ),
];

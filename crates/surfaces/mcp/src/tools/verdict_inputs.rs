//! `specforge.verdict_inputs`: a judged rule's inputs for one entity, the answer of
//! `specforge query <entity> --inputs <rule>` (ADR 0290).

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::Answered;
use crate::target::View;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};
use specforge_ops::read::judged::{InputsField, InputsRequest, JudgedInputs, judged_inputs};
use specforge_ops::view::ProjectView;
use specforge_protocol_types::Cite;

/// `specforge.verdict_inputs`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// The entity the rule judges
    entity: String,
    /// The judged rule's code, qualified (`gsd/W311`) or local when unique
    rule: String,
    /// Entities the verdict will cite
    cite_entities: Vec<String>,
    /// Files the verdict will cite, from the project root
    cite_files: Vec<String>,
    /// Tests the verdict will cite, by name
    cite_tests: Vec<String>,
}

pub(crate) struct Tool;

impl ArgumentOf<InputsField> for Tool {
    fn argument(field: InputsField) -> Option<&'static str> {
        match field {
            InputsField::Entity => Some("entity"),
            InputsField::Rule => Some("rule"),
            InputsField::Cited => Some("cite_entities"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.verdict_inputs";
    const DESCRIPTION: &'static str = "Read what a judged rule judges for one entity: each input with its digest, the verdict's state, and the basis set_verdict checks";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpVerdictInputsResult");
    /// The basis maps each input's name to its digest.
    const OPEN_VALUES: &'static [&'static str] = &["$.basis"];
    type Args = Args;
    type Input = View;
    type Returns = Answered<JudgedInputs>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<JudgedInputs> {
        let cited: Vec<Cite> = args
            .cite_entities
            .into_iter()
            .map(|entity| Cite::Entity { entity })
            .chain(args.cite_files.into_iter().map(|file| Cite::File { file }))
            .chain(args.cite_tests.into_iter().map(|test| Cite::Test { test }))
            .collect();
        let request = InputsRequest {
            entity: &args.entity,
            rule: &args.rule,
            cited: &cited,
        };
        Ok(judged_inputs(&view, &request)
            .map_err(Self::refused)?
            .into())
    }
}

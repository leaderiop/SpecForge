use crate::target::View;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};
use specforge_ops::read::navigate::{Direction, Navigator, ReferenceLocations, ReferenceQuery};

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::Answered;
use specforge_ops::view::EntityField;
use specforge_ops::view::ProjectView;

/// `specforge.find_references`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID
    entity_id: String,
    /// Which references
    #[arg(choice = specforge_ops::read::navigate::DIRECTION)]
    direction: Direction,
    /// Also return the entity's own declaration (its name)
    include_declaration: bool,
}

/// `specforge.find_references`: each occurrence of the entity's ID, held by
/// each block that holds it, as the LSP's references answer it (ADR 0016, ADR 0176). Incoming by default, the
/// declaration only when asked for.
pub(crate) struct Tool;

impl ArgumentOf<EntityField> for Tool {
    fn argument(field: EntityField) -> Option<&'static str> {
        match field {
            EntityField::EntityId => Some("entity_id"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.find_references";
    const DESCRIPTION: &'static str = "Find the references to an entity: each place another entity's field names it, as the identifier token (what an IDE's find-references shows)";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Navigation,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpReferenceResult");
    type Args = Args;
    type Input = View;
    type Returns = Answered<ReferenceLocations>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<ReferenceLocations> {
        let query = ReferenceQuery {
            direction: args.direction,
            include_declaration: args.include_declaration,
        };
        Ok(Navigator::new(view)
            .reference_locations(&args.entity_id, query)
            .map_err(Self::refused)?
            .into())
    }
}

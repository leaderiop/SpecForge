//! The hover on a string value a checker read (ADR 0336): the facts for [`crate::hover::checked_value`].

use specforge_ops::authoring::format::Place;
use tower_lsp::lsp_types::{Position, Url};

use crate::state::LspState;

/// What the checkers said of the string value under the cursor (ADR 0336).
pub(crate) fn hover(state: &LspState, uri: &Url, position: Position) -> Option<String> {
    let doc = state.document(uri)?;
    let cursor = doc.at(position)?;
    let value = cursor.string_value()?;
    let view = state.view();
    let file = crate::uri::file_path(uri);
    let place = file.as_deref().map_or(Place::Detached, Place::File);
    let id = crate::completion::holder(&cursor, &view, place)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64);
    crate::hover::checked_value(
        view.project().value_checks().of(id.as_str()),
        &value.field,
        value.text,
        view.env().effects().mode().name(),
        now,
    )
}

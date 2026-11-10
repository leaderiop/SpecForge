//! Source documents (ADR 0349 D7, D8): a Rust, TypeScript or JavaScript file under the project root, where a
//! collector placed facts. The server holds its buffer, to hash and place in, and never hands it to the session:
//! typing in a source file rebuilds nothing. A span in one is placed through its text (the open buffer, else the file
//! read through the session's containment) only while that text hashes to the digest its fact store recorded at
//! collect (`CompiledProject::fact_sources`); a stale file places nothing.

use std::path::Path;
use std::sync::Arc;

use tower_lsp::lsp_types::Url;

use crate::LspState;
use crate::document::LineIndex;

/// The extensions of the files the server answers as source documents.
const EXTENSIONS: [&str; 5] = ["rs", "ts", "tsx", "js", "jsx"];

/// Whether `uri` is a source document: a `file:` URI of a `.rs`, `.ts`, `.tsx`, `.js` or `.jsx` file under the
/// served project's root. Never a spec source nor `specforge.json`, by extension.
pub fn is_source(state: &LspState, uri: &Url) -> bool {
    crate::uri::file_path(uri).is_some() && is_source_key(state, &state.key_of(uri))
}

/// [`is_source`] of the file session key `key` names.
pub(crate) fn is_source_key(state: &LspState, key: &str) -> bool {
    let source = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e));
    source && state.view().root().is_some() && is_under_root(key)
}

/// Whether `key`, a session key, names a file under the project root: relative and not climbing out of it.
fn is_under_root(key: &str) -> bool {
    let key = Path::new(key);
    !key.is_absolute()
        && key
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// The line index of `key`'s text, where `key` is a source file the session's facts are placed in: its open buffer,
/// else the file read through the session's containment, when that text hashes to the digest `recorded` (the one its
/// store recorded at collect). `None` for any other file, and for a text that changed since or cannot be read.
pub(crate) fn placed_index(state: &LspState, key: &str, recorded: &str) -> Option<Arc<LineIndex>> {
    if let Some(document) = state.files().document(key) {
        return (specforge_ops::revision::of_text(document.text()) == recorded)
            .then(|| Arc::clone(document.index()));
    }
    let view = state.view();
    let (_, bytes) = view.containment().read(&view.root()?.join(key)).ok()?;
    if specforge_ops::revision::of_bytes(&bytes) != recorded {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    Some(Arc::new(LineIndex::new(&text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_relative_key_inside_the_root_is_under_it() {
        assert!(is_under_root("src/lib.rs"));
        assert!(!is_under_root("../other/lib.rs"));
        assert!(!is_under_root("/abs/lib.rs"));
        assert!(!is_under_root("src/../../lib.rs"));
    }
}

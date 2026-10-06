//! The LSP's one reader of a `.spec` buffer (ADR 0023): a [`Document`]
//! owns an open buffer, its version and its [`LineIndex`], the only
//! conversion between byte offsets and UTF-16 positions.

mod line_index;

pub use line_index::LineIndex;

use std::sync::Arc;
use tower_lsp::lsp_types::Range;

/// An open document: the editor's text of one `.spec` file, its version
/// and its line index. Every question the LSP asks about a buffer's text
/// is asked here: where a position is (`index`). The graph is never
/// consulted for structure: it lags the buffer while the user types
/// (ADR 0023).
#[derive(Debug, Clone)]
pub struct Document {
    uri: String,
    text: String,
    /// The editor's version from didOpen/didChange, stamped onto published
    /// diagnostics so clients drop stale deliveries.
    version: Option<i32>,
    index: Arc<LineIndex>,
}

impl Document {
    pub fn new(uri: String, text: String) -> Document {
        let index = Arc::new(LineIndex::new(&text));
        Document {
            uri,
            text,
            version: None,
            index,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn version(&self) -> Option<i32> {
        self.version
    }

    pub fn set_version(&mut self, version: i32) {
        self.version = Some(version);
    }

    /// Where each line starts: positions in this document, both ways.
    pub fn index(&self) -> &Arc<LineIndex> {
        &self.index
    }

    /// Apply one content change: `range` replaced by `text`, its ends
    /// clamped to the document; the whole text when `range` is `None`.
    pub fn apply_change(&mut self, range: Option<Range>, text: &str) {
        match range {
            Some(range) => {
                let start = self.index.offset_clamped(range.start);
                let end = self.index.offset_clamped(range.end);
                let (start, end) = (start.min(end), start.max(end));
                self.text.replace_range(start..end, text);
            }
            None => self.text = text.to_string(),
        }
        self.index = Arc::new(LineIndex::new(&self.text));
    }
}

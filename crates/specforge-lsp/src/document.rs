/// A text buffer for an open document. Incremental parse trees are owned by
/// the shared [`specforge_watch::IncrementalPipeline`]; the buffer only holds
/// editor text and position math.
pub struct DocumentBuffer {
    uri: String,
    content: String,
}

impl DocumentBuffer {
    pub fn new(uri: String, content: String) -> Self {
        Self { uri, content }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// Apply an incremental text edit specified by (start_line, start_col) to
    /// (end_line, end_col) with replacement text. Lines and columns are 0-based.
    pub fn apply_change(
        &mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        new_text: &str,
    ) {
        let start_offset = self.line_col_to_offset(start_line, start_col);
        let end_offset = self.line_col_to_offset(end_line, end_col);

        self.content
            .replace_range(start_offset..end_offset, new_text);
    }

    fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let mut offset = 0;
        for (i, l) in self.content.split('\n').enumerate() {
            if i == line {
                return offset + utf16_col_to_byte_offset(l, col);
            }
            offset += l.len() + 1; // +1 for '\n'
        }
        self.content.len()
    }
}

/// Convert a UTF-16 code-unit column (the LSP `character` field) to a byte
/// offset within `line`. Characters are consumed while they end at or before
/// `col`; a column landing inside a surrogate pair or past the end of the
/// line clamps to the nearest char boundary / end of line.
pub fn utf16_col_to_byte_offset(line: &str, col: usize) -> usize {
    let mut units = 0usize;
    let mut byte_offset = 0usize;
    for ch in line.chars() {
        let len = ch.len_utf16();
        if units + len > col {
            break;
        }
        units += len;
        byte_offset += ch.len_utf8();
    }
    byte_offset
}

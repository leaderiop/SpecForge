/// A text buffer for an open document. Incremental parse trees are owned by
/// the shared [`specforge_watch::IncrementalPipeline`]; the buffer only holds
/// editor text and position math.
pub struct DocumentBuffer {
    uri: String,
    content: String,
    /// Editor document version from didOpen/didChange, stamped onto
    /// published diagnostics so clients can drop stale deliveries (C4-05).
    version: Option<i32>,
}

impl DocumentBuffer {
    pub fn new(uri: String, content: String) -> Self {
        Self {
            uri,
            content,
            version: None,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_version(&mut self, version: i32) {
        self.version = Some(version);
    }

    pub fn version(&self) -> Option<i32> {
        self.version
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
/// Convert a byte column within `line` to UTF-16 code units (the LSP
/// `character` field). tree-sitter emits byte columns; LSP positions are
/// UTF-16 — passing bytes through unconverted shifts every range on lines
/// containing non-ASCII characters (C3-09). Bytes past the line end clamp
/// to the line's full UTF-16 length.
pub fn byte_col_to_utf16_col(line: &str, byte_col: usize) -> usize {
    let mut units = 0usize;
    let mut bytes = 0usize;
    for ch in line.chars() {
        if bytes >= byte_col {
            break;
        }
        bytes += ch.len_utf8();
        units += ch.len_utf16();
    }
    units
}

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

#[cfg(test)]
mod utf16_tests {
    use super::*;

    #[test]
    fn ascii_lines_are_identity() {
        let line = "behavior b1 {";
        assert_eq!(byte_col_to_utf16_col(line, 5), 5);
        assert_eq!(utf16_col_to_byte_offset(line, 5), 5);
    }

    #[test]
    fn multibyte_characters_shift_utf16_columns() {
        // "héllo wörld" — é/ö are 2 bytes but 1 UTF-16 unit each
        let line = "h\u{e9}llo w\u{f6}rld";
        // Position 8 = after "w" (7 bytes) = 7 UTF-16 units (accents are
        // 2 bytes but 1 UTF-16 unit each). Position 10 = after "ö" = 8 units.
        assert_eq!(byte_col_to_utf16_col(line, 8), 7);
        assert_eq!(byte_col_to_utf16_col(line, 10), 8);
    }

    #[test]
    fn surrogate_pairs_count_as_two_units() {
        // emoji: 4 bytes, 2 UTF-16 units
        let line = "\u{1F600} behavior";
        let byte_col = 4; // past the emoji
        assert_eq!(byte_col_to_utf16_col(line, byte_col), 2);
        assert_eq!(utf16_col_to_byte_offset(line, 2), 4);
    }

    #[test]
    fn columns_past_line_end_clamp() {
        let line = "abc";
        assert_eq!(byte_col_to_utf16_col(line, 100), 3);
        assert_eq!(utf16_col_to_byte_offset(line, 100), 3);
    }
}

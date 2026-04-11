/// A text buffer for an open document that supports incremental edits.
pub struct DocumentBuffer {
    uri: String,
    content: String,
    /// Cached tree-sitter tree for incremental reparsing.
    previous_tree: Option<tree_sitter::Tree>,
}

impl DocumentBuffer {
    pub fn new(uri: String, content: String) -> Self {
        Self {
            uri,
            content,
            previous_tree: None,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn previous_tree(&self) -> Option<&tree_sitter::Tree> {
        self.previous_tree.as_ref()
    }

    pub fn set_previous_tree(&mut self, tree: Option<tree_sitter::Tree>) {
        self.previous_tree = tree;
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

        // Notify the cached tree about this edit so it can be used for incremental parsing
        if let Some(tree) = &mut self.previous_tree {
            let start_byte = start_offset;
            let old_end_byte = end_offset;
            let new_end_byte = start_offset + new_text.len();
            let start_position = tree_sitter::Point {
                row: start_line,
                column: start_col,
            };
            let old_end_position = tree_sitter::Point {
                row: end_line,
                column: end_col,
            };
            // Compute new end position
            let new_end_position = if new_text.contains('\n') {
                let lines: Vec<&str> = new_text.split('\n').collect();
                tree_sitter::Point {
                    row: start_line + lines.len() - 1,
                    column: if lines.len() > 1 {
                        lines.last().map_or(0, |l| l.len())
                    } else {
                        start_col + new_text.len()
                    },
                }
            } else {
                tree_sitter::Point {
                    row: start_line,
                    column: start_col + new_text.len(),
                }
            };
            tree.edit(&tree_sitter::InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            });
        }

        self.content.replace_range(start_offset..end_offset, new_text);
    }

    fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let mut offset = 0;
        for (i, l) in self.content.split('\n').enumerate() {
            if i == line {
                return offset + col;
            }
            offset += l.len() + 1; // +1 for '\n'
        }
        self.content.len()
    }
}

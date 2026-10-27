use super::*;
use crate::lex::{LiteralShape, is_number_word, literal_shape};

/// What a block's members read to; `obligations` and `obligation_spans` are pushed together, at one site.
#[derive(Default)]
pub(super) struct Body {
    pub(super) fields: FieldMap,
    pub(super) obligations: Vec<VerifyStatement>,
    pub(super) obligation_spans: Vec<SourceSpan>,
    pub(super) methods: Vec<MethodDecl>,
    /// The own-line comments of the body, nested block fields' included, in source order.
    pub(super) comments: Vec<SourceSpan>,
    /// The comments ending the row of the item before them.
    pub(super) trailing: Vec<SourceSpan>,
    /// The comments inside a member's value.
    pub(super) inside: Vec<SourceSpan>,
}

impl<'a> ParseContext<'a> {
    pub(super) fn parse_block_body(&mut self, node: Node) -> Body {
        let mut body = Body::default();
        let mut cursor = node.walk();
        // The row where the previous item ends: a comment on it trails that item.
        let mut trailed: Option<usize> = None;
        for child in node.children(&mut cursor) {
            match child.kind() {
                _ if child.is_error() => self.push_error_node(child),
                kind::COMMENT if own_line(child, trailed) => {
                    self.body_comments.push(self.span(child));
                }
                kind::COMMENT => self.trailing_comments.push(self.span(child)),
                kind::FIELD => {
                    if let Some(entry) = self.parse_field(child) {
                        body.fields.push_entry(entry);
                    }
                }
                kind::VERIFY_STATEMENT => {
                    if let Some(stmt) = self.parse_verify_statement(child) {
                        body.obligations.push(stmt);
                        body.obligation_spans.push(self.span(child));
                    }
                }
                kind::METHOD_STATEMENT => body.methods.push(self.parse_method_statement(child)),
                _ => {}
            }
            trailed = Some(child.end_position().row);
        }

        body.comments = std::mem::take(&mut self.body_comments);
        body.trailing = std::mem::take(&mut self.trailing_comments);
        body.inside = std::mem::take(&mut self.value_comments);
        body
    }

    fn parse_field(&mut self, node: Node) -> Option<FieldEntry> {
        let key = node.child_by_field_name(field::KEY)?;
        let value = node.child_by_field_name(field::VALUE)?;
        // Tree-sitter may recover from a syntax error deep inside a value
        // (e.g. a dangling operator in an expression group); surface it.
        if let Some(broken) = find_error_descendant(value) {
            let span = self.span(broken);
            self.errors.push(ParseError {
                message: "syntax error in field value".to_string(),
                span,
            });
        }
        let key_sym = Sym::new(self.text(key));
        let field_value = self.parse_value(value);
        self.comments_inside(value);

        // Extract annotations (children with kind "annotation")
        let mut annotations = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == kind::ANNOTATION {
                annotations.push(self.parse_annotation(child));
            }
        }

        Some(FieldEntry {
            key: key_sym,
            value: field_value,
            annotations,
            value_span: Some(self.span(value)),
        })
    }

    /// Parse an annotation node. The grammar defines annotation as:
    ///   `TOKEN("@" + identifier_pattern) string?`
    ///
    /// The `@name` portion is an opaque TOKEN — tree-sitter does not create a
    /// child node for it. Instead the `@name` text is the beginning of the
    /// annotation node's own source text. The only possible named child is
    /// an optional `string` node carrying the annotation value.
    fn parse_annotation(&self, node: Node) -> Annotation {
        // The annotation node's full text starts with "@name" followed by
        // optional whitespace and a quoted string value.
        // Extract the name by taking the first whitespace-delimited token
        // and stripping the leading '@'.
        let full_text = self.text(node).trim();
        let name = full_text
            .split_whitespace()
            .next()
            .unwrap_or(full_text)
            .strip_prefix('@')
            .unwrap_or(full_text)
            .to_string();

        // Look for an optional string child (the annotation value).
        // The grammar does not use field names on annotation children,
        // so we search by node kind.
        let value = {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|c| c.kind() == kind::STRING)
                .map(|n| self.unquote(n))
        };

        Annotation { name, value }
    }

    fn parse_value(&mut self, node: Node) -> FieldValue {
        match node.kind() {
            kind::STRING => FieldValue::String(string_text(self.text(node))),
            kind::TRIPLE_QUOTED_STRING => FieldValue::String(triple_text(self.text(node))),
            kind::IDENTIFIER => word_value(self.text(node)),
            kind::ARRAY_TYPE | kind::REFERENCE_PATH => {
                FieldValue::Identifier(self.text(node).to_string())
            }
            kind::EXPR_GROUP => FieldValue::Expression(self.parse_expr_group(node)),
            kind::TYPE_UNION => FieldValue::TypeUnion(self.parse_type_union(node)),
            kind::LIST => self.parse_list(node),
            kind::BLOCK_LIST => self.parse_block_list(node),
            kind::NESTED_BLOCK => self.parse_nested_block(node),
            _ => FieldValue::String(self.text(node).to_string()),
        }
    }

    /// A list as written: every item with its text, token span and how it
    /// was written. Never judged here: what a list references, and whether
    /// its items suit the field, is the field's declared type's reading.
    fn parse_list(&mut self, node: Node) -> FieldValue {
        let mut items = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let Some((written, text)) = value_token(child, self.source) else {
                continue;
            };
            items.push(ListItem {
                text,
                span: self.span(child),
                written,
            });
        }
        FieldValue::List(items)
    }

    /// Each comment inside `node`, a field's value, recorded as a value comment: a list's at any depth, an expression
    /// group's. A nested block's are its members' (`parse_nested_block` reads them).
    fn comments_inside(&mut self, node: Node) {
        if matches!(node.kind(), kind::NESTED_BLOCK | kind::BLOCK_LIST) {
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == kind::COMMENT {
                let span = self.span(child);
                self.value_comments.push(span);
            } else {
                self.comments_inside(child);
            }
        }
    }

    /// A list of blocks: each `{ … }` child read as a nested block, with its span. Its comments are its members'.
    fn parse_block_list(&mut self, node: Node) -> FieldValue {
        let mut items = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == kind::COMMENT {
                let span = self.span(child);
                self.value_comments.push(span);
            } else if child.kind() == kind::NESTED_BLOCK
                && let FieldValue::Block(fields) = self.parse_nested_block(child)
            {
                items.push(BlockItem {
                    fields,
                    span: self.span(child),
                });
            }
        }
        FieldValue::Blocks(items)
    }

    fn parse_nested_block(&mut self, node: Node) -> FieldValue {
        let mut fields = FieldMap::new();
        let mut cursor = node.walk();
        let mut trailed: Option<usize> = None;
        for child in node.children(&mut cursor) {
            if child.kind() == kind::COMMENT {
                let span = self.span(child);
                if own_line(child, trailed) {
                    self.body_comments.push(span);
                } else {
                    self.trailing_comments.push(span);
                }
            } else if child.kind() == kind::FIELD
                && let Some(entry) = self.parse_field(child)
            {
                fields.push_entry(entry);
            }
            trailed = Some(child.end_position().row);
        }
        FieldValue::Block(fields)
    }

    fn parse_method_statement(&self, node: Node) -> MethodDecl {
        let name_node = node.child_by_field_name(field::NAME);
        let name = name_node
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        let returns_node = node.child_by_field_name(field::RETURNS);
        let returns = returns_node.map(|n| self.text(n).trim().to_string());
        let mut params = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != kind::PARAMETER {
                continue;
            }
            let pname = child
                .child_by_field_name(field::NAME)
                .map(|n| self.text(n).to_string())
                .unwrap_or_default();
            let ty_node = child.child_by_field_name(field::TYPE);
            let pty = ty_node
                .map(|n| self.text(n).trim().to_string())
                .unwrap_or_default();
            let mut annotations = Vec::new();
            let mut param_cursor = child.walk();
            for part in child.children(&mut param_cursor) {
                if part.kind() == kind::ANNOTATION {
                    annotations.push(self.parse_annotation(part));
                }
            }
            params.push(Parameter {
                name: pname,
                ty: pty,
                ty_span: ty_node.map(|n| self.span(n)),
                optional: child.child_by_field_name(field::OPTIONAL).is_some(),
                annotations,
            });
        }
        MethodDecl {
            name,
            name_span: name_node.map(|n| self.span(n)),
            params,
            returns,
            returns_span: returns_node.map(|n| self.span(n)),
            span: self.span(node),
        }
    }

    fn parse_verify_statement(&self, node: Node) -> Option<VerifyStatement> {
        let desc = node.child_by_field_name(field::DESCRIPTION)?;
        let kind = node
            .child_by_field_name(field::KIND)
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        Some(VerifyStatement {
            kind,
            description: self.unquote(desc),
        })
    }

    /// Collect the declared types of a union-typed field value.
    fn parse_type_union(&self, node: Node<'a>) -> Vec<TypeMember> {
        let mut types = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                kind::IDENTIFIER
                | kind::ARRAY_TYPE
                | kind::TYPE_GENERIC
                | kind::UNIT_TYPE
                | kind::FUNCTION_TYPE
                | kind::STRING => {
                    types.push(TypeMember {
                        text: self.text(child).trim().to_string(),
                        span: self.span(child),
                        written: if child.kind() == kind::STRING {
                            Written::Quoted
                        } else {
                            Written::Bare
                        },
                    });
                }
                _ => {}
            }
        }
        types
    }
}

fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    let min_indent = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let mut result: Vec<&str> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                result.push(trimmed);
            }
        } else if line.trim().is_empty() {
            result.push("");
        } else {
            // `get`: another line's indentation may be a multi-byte space, and `min_indent` counts bytes
            result.push(line.get(min_indent..).unwrap_or(line));
        }
    }

    while result.last() == Some(&"") {
        result.pop();
    }

    result.join("\n")
}

/// A bare word as the parse stores it: tagged by its shape, losing nothing (ADR 0283). The field's declared type,
/// which the parser never reads, decides what the word means.
fn word_value(word: &str) -> FieldValue {
    match literal_shape(word) {
        LiteralShape::Integer(n) => FieldValue::Integer(n),
        LiteralShape::Boolean(b) => FieldValue::Boolean(b),
        LiteralShape::Date => FieldValue::Date(word.to_string()),
        LiteralShape::Other => FieldValue::Identifier(word.to_string()),
    }
}

/// The text a `"""…"""` token reads to: its inside dedented (the first line trimmed, the common indentation of the
/// others dropped, trailing blank lines dropped).
pub(crate) fn triple_text(raw: &str) -> String {
    let inner = raw
        .strip_prefix("\"\"\"")
        .and_then(|r| r.strip_suffix("\"\"\""))
        .unwrap_or(raw);
    dedent(inner)
}

/// A value token as the parse reads it (ADR 0247): how it is written and the text the parse stores for it. A `"…"`
/// is [`string_text`], a `"""…"""` is [`triple_text`], a bare word, scheme ref id or literal is its text; nothing is
/// otherwise trimmed. `true` / `false` arrive as identifiers inside a list and are literals. `None` for every other
/// node: a list, a nested block, an expression group, a type union, an array type, a bracket, a comma, a comment.
/// The parse's field values and list items are built from it; the formatter's rewrite reads it. `source` is the
/// text the node's tree was parsed from.
pub fn value_token(node: Node<'_>, source: &str) -> Option<(Written, String)> {
    let text = source.get(node.byte_range())?;
    Some(match node.kind() {
        kind::STRING => (Written::Quoted, string_text(text)),
        kind::TRIPLE_QUOTED_STRING => (Written::Quoted, triple_text(text)),
        kind::IDENTIFIER if text == "true" || text == "false" || is_number_word(text) => {
            (Written::Literal, text.to_string())
        }
        kind::IDENTIFIER => (Written::Bare, text.to_string()),
        kind::REFERENCE_PATH => (Written::Bare, text.to_string()),
        kind::SCHEME_REF_ID => (Written::SchemeRef, text.to_string()),
        kind::INTEGER | kind::NEGATIVE_INTEGER => (Written::Literal, text.to_string()),
        _ => return None,
    })
}

/// Depth-first search for an ERROR or MISSING node produced by the
/// parser's recovery inside a value subtree.
fn find_error_descendant<'a>(node: Node<'a>) -> Option<Node<'a>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_error_descendant(child) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod dedent_tests {
    #[test]
    fn a_multi_byte_space_in_the_indentation_does_not_split_a_character() {
        assert_eq!(super::dedent("a\n b\n\u{a0}c"), "a\nb\n\u{a0}c");
    }
}

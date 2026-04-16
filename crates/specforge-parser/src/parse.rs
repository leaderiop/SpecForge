use crate::ast::*;
use specforge_common::{SourceSpan, Sym};
use tree_sitter::{Node, Parser};

/// Process escape sequences in a string literal (after quote stripping).
fn unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn parse(source: &str, file_path: &str) -> SpecFile {
    parse_incremental(source, file_path, None).0
}

/// Parse with an optional old tree for incremental reparsing.
/// Returns the parsed `SpecFile` and the new `tree_sitter::Tree` for reuse.
pub fn parse_incremental(
    source: &str,
    file_path: &str,
    old_tree: Option<&tree_sitter::Tree>,
) -> (SpecFile, Option<tree_sitter::Tree>) {
    let file_sym = Sym::new(file_path);
    let mut parser = Parser::new();
    if let Err(e) = parser.set_language(&tree_sitter_specforge::LANGUAGE.into()) {
        return (SpecFile {
            path: file_sym,
            imports: Vec::new(),
            entities: Vec::new(),
            errors: vec![ParseError {
                message: format!("failed to load specforge grammar: {e}"),
                span: SourceSpan { file: file_sym, start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
                expected: None,
                found: None,
            }],
        }, None);
    }

    let Some(tree) = parser.parse(source, old_tree) else {
        return (SpecFile {
            path: file_sym,
            imports: Vec::new(),
            entities: Vec::new(),
            errors: vec![ParseError {
                message: "tree-sitter parse failed".to_string(),
                span: SourceSpan { file: file_sym, start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
                expected: None,
                found: None,
            }],
        }, None);
    };
    let mut ctx = ParseContext {
        source,
        file_sym,
        imports: Vec::new(),
        entities: Vec::new(),
        errors: Vec::new(),
    };

    // Scope the tree borrow (root + cursor) so `tree` can be moved into the return
    {
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "entity_block" => ctx.parse_entity_block(child),
                "spec_block" => ctx.parse_spec_block(child),
                "ref_block" => ctx.parse_ref_block(child),
                "define_block" => ctx.parse_define_block(child),
                "union_block" => ctx.parse_union_block(child),
                "use_import" => ctx.parse_use_import(child, false),
                "pub_use_import" => ctx.parse_use_import(child, true),
                "comment" => {}
                "ERROR" => ctx.push_error_node(child),
                _ => {}
            }
        }
    }

    (SpecFile {
        path: file_sym,
        imports: ctx.imports,
        entities: ctx.entities,
        errors: ctx.errors,
    }, Some(tree))
}

struct ParseContext<'a> {
    source: &'a str,
    file_sym: Sym,
    imports: Vec<ImportDeclaration>,
    entities: Vec<Entity>,
    errors: Vec<ParseError>,
}

impl<'a> ParseContext<'a> {
    fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    fn span(&self, node: Node) -> SourceSpan {
        let start = node.start_position();
        let end = node.end_position();
        SourceSpan {
            file: self.file_sym,
            start_line: start.row + 1,
            start_col: start.column + 1,
            end_line: end.row + 1,
            end_col: end.column + 1,
        }
    }

    fn unquote(&self, node: Node) -> String {
        let text = self.text(node);
        if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
            let inner = &text[1..text.len() - 1];
            unescape(inner)
        } else {
            text.to_string()
        }
    }

    fn extract_brace_body(&self, node: Node) -> Option<String> {
        let mut open = None;
        let mut close = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "{" {
                open = Some(child.end_byte());
            } else if child.kind() == "}" {
                close = Some(child.start_byte());
            }
        }
        match (open, close) {
            (Some(start), Some(end)) if end > start => {
                Some(self.source[start..end].to_string())
            }
            (Some(start), Some(end)) if start == end => Some(String::new()),
            _ => None,
        }
    }

    fn push_error_node(&mut self, node: Node) {
        let text = self.text(node);
        let trimmed = text.trim();
        let first_line = trimmed.lines().next().unwrap_or(trimmed);
        // Truncate very long error text for readability
        let display = if first_line.len() > 60 {
            format!("{}...", &first_line[..57])
        } else {
            first_line.to_string()
        };

        // Provide contextual suggestions for common mistakes
        let (message, expected) = if trimmed.contains('{') && !trimmed.contains('}') {
            (
                format!("syntax error near '{}': unclosed block — missing closing '}}'", display),
                Some("a closing '}'".to_string()),
            )
        } else if trimmed.starts_with('}') {
            (
                "syntax error: unexpected '}' — possible extra closing brace".to_string(),
                Some("a valid entity block (e.g., behavior name \"Title\" {{ ... }})".to_string()),
            )
        } else {
            (
                format!("syntax error: unexpected '{}'", display),
                Some("a valid block: keyword name \"Title\" {{ fields... }}".to_string()),
            )
        };

        self.errors.push(ParseError {
            message,
            span: self.span(node),
            expected,
            found: Some(display),
        });
    }

    fn parse_entity_block(&mut self, node: Node) {
        let kind = node
            .child_by_field_name("kind")
            .map(|n| Sym::new(self.text(n)))
            .unwrap_or_else(|| Sym::new(""));
        let name = node
            .child_by_field_name("name")
            .map(|n| Sym::new(self.text(n)))
            .unwrap_or_else(|| Sym::new(""));
        let title = node.child_by_field_name("title").map(|n| self.unquote(n));

        let raw_body = self.extract_brace_body(node);
        let (fields, verify) = self.parse_block_body(node);
        let mut field_map = fields;
        if !verify.is_empty() {
            field_map.push(Sym::new("verify"), FieldValue::VerifyList(verify));
        }

        self.entities.push(Entity {
            kind: EntityKind { raw: kind },
            id: EntityId { raw: name },
            title,
            fields: field_map,
            raw_body,
            span: self.span(node),
        });
    }

    fn parse_spec_block(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.unquote(n))
            .unwrap_or_default();

        let raw_body = self.extract_brace_body(node);
        let (fields, verify) = self.parse_block_body(node);
        let mut field_map = fields;
        if !verify.is_empty() {
            field_map.push(Sym::new("verify"), FieldValue::VerifyList(verify));
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: Sym::new("spec"),
            },
            id: EntityId { raw: Sym::new(&name) },
            title: Some(name),
            fields: field_map,
            raw_body,
            span: self.span(node),
        });
    }

    fn parse_ref_block(&mut self, node: Node) {
        let Some(inner) = node.child(0) else {
            self.push_error_node(node);
            return;
        };
        let id_text = inner
            .child_by_field_name("id")
            .map(|n| self.text(n))
            .unwrap_or_default();
        let title = inner.child_by_field_name("title").map(|n| self.unquote(n));

        let mut fields = FieldMap::new();
        if let Some((scheme, kind, identifier)) = parse_ref_id(id_text) {
            fields.push(Sym::new("scheme"), FieldValue::String(scheme));
            fields.push(Sym::new("ref_kind"), FieldValue::String(kind));
            fields.push(Sym::new("identifier"), FieldValue::String(identifier));
        }

        if inner.kind() == "ref_full" {
            let (body_fields, _) = self.parse_block_body(inner);
            for entry in body_fields.entries() {
                fields.push_annotated(entry.key, entry.value.clone(), entry.annotations.clone());
            }
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: Sym::new("ref"),
            },
            id: EntityId { raw: Sym::new(id_text) },
            title,
            fields,
            raw_body: None,
            span: self.span(node),
        });
    }

    fn parse_union_block(&mut self, node: Node) {
        let kind = node
            .child_by_field_name("kind")
            .map(|n| Sym::new(self.text(n)))
            .unwrap_or_else(|| Sym::new(""));
        let name = node
            .child_by_field_name("name")
            .map(|n| Sym::new(self.text(n)))
            .unwrap_or_else(|| Sym::new(""));

        let mut variants = Vec::new();
        if let Some(variants_node) = node.child_by_field_name("variants") {
            let mut cursor = variants_node.walk();
            for child in variants_node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => variants.push(self.text(child).to_string()),
                    "string" => variants.push(self.unquote(child)),
                    "integer" | "negative_integer" => {
                        variants.push(self.text(child).to_string())
                    }
                    _ => {}
                }
            }
        }

        let mut fields = FieldMap::new();
        fields.push(Sym::new("variants"), FieldValue::VariantList(variants));

        self.entities.push(Entity {
            kind: EntityKind { raw: kind },
            id: EntityId { raw: name },
            title: None,
            fields,
            raw_body: None,
            span: self.span(node),
        });
    }

    fn parse_define_block(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| Sym::new(self.text(n)))
            .unwrap_or_else(|| Sym::new(""));

        let raw_body = self.extract_brace_body(node);
        let (fields, verify) = self.parse_block_body(node);
        let mut field_map = fields;
        if !verify.is_empty() {
            field_map.push(Sym::new("verify"), FieldValue::VerifyList(verify));
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: Sym::new("define"),
            },
            id: EntityId { raw: name },
            title: None,
            fields: field_map,
            raw_body,
            span: self.span(node),
        });
    }

    fn parse_use_import(&mut self, node: Node, is_pub: bool) {
        let path = node
            .child_by_field_name("path")
            .map(|n| Sym::new(&self.unquote(n)))
            .unwrap_or_else(|| Sym::new(""));

        let (kind, bindings, namespace) =
            if let Some(bindings_node) = node.child_by_field_name("bindings") {
                let mut bs = Vec::new();
                let mut cursor = bindings_node.walk();
                for child in bindings_node.children(&mut cursor) {
                    if child.kind() == "import_binding" {
                        let name = child
                            .child(0)
                            .map(|n| self.text(n).to_string())
                            .unwrap_or_default();
                        let alias = child
                            .child_by_field_name("alias")
                            .map(|n| self.text(n).to_string());
                        bs.push(ImportBinding { name, alias });
                    }
                }
                (ImportKind::Selective, Some(bs), None)
            } else if let Some(ns_node) = node.child_by_field_name("namespace") {
                let alias = ns_node
                    .child_by_field_name("alias")
                    .map(|n| self.text(n).to_string())
                    .unwrap_or_default();
                (ImportKind::Namespace, None, Some(alias))
            } else {
                (ImportKind::Full, None, None)
            };

        self.imports.push(ImportDeclaration {
            path,
            kind,
            bindings,
            namespace,
            is_pub,
            span: self.span(node),
        });
    }

    fn parse_block_body(&mut self, node: Node) -> (FieldMap, Vec<VerifyStatement>) {
        let mut fields = FieldMap::new();
        let mut verify = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "field" => {
                    if let Some((key, value, annotations)) = self.parse_field(child) {
                        fields.push_annotated(key, value, annotations);
                    }
                }
                "verify_statement" => {
                    if let Some(stmt) = self.parse_verify_statement(child) {
                        verify.push(stmt);
                    }
                }
                "ERROR" => self.push_error_node(child),
                _ => {}
            }
        }

        (fields, verify)
    }

    fn parse_field(&mut self, node: Node) -> Option<(Sym, FieldValue, Vec<Annotation>)> {
        let key = node.child_by_field_name("key")?;
        let value = node.child_by_field_name("value")?;
        let key_sym = Sym::new(self.text(key));
        let mut field_value = self.parse_value(value);
        // A field named "values" contains enum tags, not entity references
        if key_sym == "values"
            && let FieldValue::ReferenceList(items) = &field_value
        {
            field_value = FieldValue::VariantList(items.clone());
        }

        // Extract annotations (children with kind "annotation")
        let mut annotations = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "annotation" {
                annotations.push(self.parse_annotation(child));
            }
        }

        Some((key_sym, field_value, annotations))
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
                .find(|c| c.kind() == "string")
                .map(|n| self.unquote(n))
        };

        Annotation { name, value }
    }

    fn parse_value(&mut self, node: Node) -> FieldValue {
        match node.kind() {
            "string" => FieldValue::String(self.unquote(node)),
            "triple_quoted_string" => FieldValue::String(self.parse_triple_quoted(node)),
            "integer" | "negative_integer" => {
                let text = self.text(node);
                match text.parse::<i64>() {
                    Ok(val) => FieldValue::Integer(val),
                    Err(_) => {
                        self.errors.push(ParseError {
                            message: format!("integer value '{}' is too large (overflows i64)", text),
                            span: self.span(node),
                            expected: Some("integer within i64 range".to_string()),
                            found: Some(text.to_string()),
                        });
                        FieldValue::Integer(0)
                    }
                }
            }
            "boolean" => FieldValue::Boolean(self.text(node) == "true"),
            "date_literal" => FieldValue::Date(self.text(node).to_string()),
            "identifier" => FieldValue::Identifier(self.text(node).to_string()),
            "array_type" => FieldValue::Identifier(self.text(node).to_string()),
            "list" => self.parse_list(node),
            "nested_block" => self.parse_nested_block(node),
            _ => FieldValue::String(self.text(node).to_string()),
        }
    }

    fn parse_triple_quoted(&self, node: Node) -> String {
        let text = self.text(node);
        let inner = &text[3..text.len() - 3];
        dedent(inner)
    }

    fn parse_list(&mut self, node: Node) -> FieldValue {
        let mut has_string = false;
        let mut has_identifier = false;
        let mut has_integer = false;
        let mut has_boolean = false;

        // Collect typed items for mixed-type detection
        let mut typed_items: Vec<FieldValue> = Vec::new();
        // Parallel flat string items for homogeneous lists
        let mut flat_items: Vec<String> = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let text = self.text(child).to_string();
                    // Detect boolean literals that tree-sitter parses
                    // as identifiers inside list context.
                    if text == "true" || text == "false" {
                        has_boolean = true;
                        typed_items.push(FieldValue::Boolean(text == "true"));
                    } else {
                        has_identifier = true;
                        typed_items.push(FieldValue::Identifier(text.clone()));
                    }
                    flat_items.push(text);
                }
                "string" => {
                    let text = self.unquote(child);
                    has_string = true;
                    typed_items.push(FieldValue::String(text.clone()));
                    flat_items.push(text);
                }
                "scheme_ref_id" => {
                    let text = self.text(child).to_string();
                    has_identifier = true;
                    typed_items.push(FieldValue::Identifier(text.clone()));
                    flat_items.push(text);
                }
                "integer" => {
                    let text = self.text(child);
                    has_integer = true;
                    let val = text.parse::<i64>().unwrap_or(0);
                    typed_items.push(FieldValue::Integer(val));
                    flat_items.push(text.to_string());
                }
                "boolean" => {
                    let text = self.text(child);
                    has_boolean = true;
                    typed_items.push(FieldValue::Boolean(text == "true"));
                    flat_items.push(text.to_string());
                }
                _ => {}
            }
        }

        // Determine how many distinct type categories are present
        let type_count = [has_string, has_identifier, has_integer, has_boolean]
            .iter()
            .filter(|&&b| b)
            .count();

        if type_count > 1 {
            // Mixed types detected — emit warning and preserve per-item types
            if has_string && has_identifier {
                self.errors.push(ParseError {
                    message: "mixed list contains both quoted strings and bare identifiers".to_string(),
                    span: self.span(node),
                    expected: Some("either all quoted strings or all bare identifiers".to_string()),
                    found: None,
                });
            }
            return FieldValue::MixedList(typed_items);
        }

        // Homogeneous list — use flat string representation
        if has_string {
            FieldValue::StringList(flat_items)
        } else {
            FieldValue::ReferenceList(flat_items)
        }
    }

    fn parse_nested_block(&mut self, node: Node) -> FieldValue {
        let mut fields = FieldMap::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "field"
                && let Some((key, value, annotations)) = self.parse_field(child) {
                    fields.push_annotated(key, value, annotations);
                }
        }
        FieldValue::Block(fields)
    }

    fn parse_verify_statement(&self, node: Node) -> Option<VerifyStatement> {
        let desc = node.child_by_field_name("description")?;
        let kind = node
            .child_by_field_name("kind")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        Some(VerifyStatement {
            kind,
            description: self.unquote(desc),
        })
    }
}

fn parse_ref_id(id: &str) -> Option<(String, String, String)> {
    let dot_pos = id.find('.')?;
    let colon_pos = id.find(':')?;
    if colon_pos <= dot_pos {
        return None;
    }
    let scheme = id[..dot_pos].to_string();
    let kind = id[dot_pos + 1..colon_pos].to_string();
    let identifier = id[colon_pos + 1..].to_string();
    Some((scheme, kind, identifier))
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
        } else if line.len() >= min_indent {
            result.push(&line[min_indent..]);
        } else {
            result.push(line);
        }
    }

    while result.last() == Some(&"") {
        result.pop();
    }

    result.join("\n")
}

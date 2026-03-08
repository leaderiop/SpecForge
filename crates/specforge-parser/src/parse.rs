use crate::ast::*;
use specforge_common::SourceSpan;
use tree_sitter::{Node, Parser};

pub fn parse(source: &str, file_path: &str) -> SpecFile {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_specforge::LANGUAGE.into())
        .expect("failed to load specforge grammar");

    let tree = parser.parse(source, None).expect("tree-sitter parse failed");
    let root = tree.root_node();

    let mut ctx = ParseContext {
        source,
        file_path,
        imports: Vec::new(),
        entities: Vec::new(),
        errors: Vec::new(),
    };

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "entity_block" => ctx.parse_entity_block(child),
            "spec_block" => ctx.parse_spec_block(child),
            "ref_block" => ctx.parse_ref_block(child),
            "define_block" => ctx.parse_define_block(child),
            "union_block" => ctx.parse_union_block(child),
            "use_import" => ctx.parse_use_import(child),
            "comment" => {}
            "ERROR" => ctx.push_error_node(child),
            _ => {}
        }
    }

    SpecFile {
        path: file_path.to_string(),
        imports: ctx.imports,
        entities: ctx.entities,
        errors: ctx.errors,
    }
}

struct ParseContext<'a> {
    source: &'a str,
    file_path: &'a str,
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
            file: self.file_path.to_string(),
            start_line: start.row + 1,
            start_col: start.column + 1,
            end_line: end.row + 1,
            end_col: end.column + 1,
        }
    }

    fn unquote(&self, node: Node) -> String {
        let text = self.text(node);
        if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
            text[1..text.len() - 1].to_string()
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
        self.errors.push(ParseError {
            message: format!("syntax error: unexpected '{}'", self.text(node)),
            span: self.span(node),
            expected: None,
            found: Some(self.text(node).to_string()),
        });
    }

    fn parse_entity_block(&mut self, node: Node) {
        let kind = node
            .child_by_field_name("kind")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        let name = node
            .child_by_field_name("name")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        let title = node.child_by_field_name("title").map(|n| self.unquote(n));

        let raw_body = self.extract_brace_body(node);
        let (fields, verify) = self.parse_block_body(node);
        let mut field_map = fields;
        if !verify.is_empty() {
            field_map.push("verify".to_string(), FieldValue::VerifyList(verify));
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
            field_map.push("verify".to_string(), FieldValue::VerifyList(verify));
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: "spec".to_string(),
            },
            id: EntityId { raw: name.clone() },
            title: Some(name),
            fields: field_map,
            raw_body,
            span: self.span(node),
        });
    }

    fn parse_ref_block(&mut self, node: Node) {
        let inner = node.child(0).unwrap();
        let id_text = inner
            .child_by_field_name("id")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        let title = inner.child_by_field_name("title").map(|n| self.unquote(n));

        let mut fields = FieldMap::new();
        if let Some((scheme, kind, identifier)) = parse_ref_id(&id_text) {
            fields.push("scheme".to_string(), FieldValue::String(scheme));
            fields.push("ref_kind".to_string(), FieldValue::String(kind));
            fields.push("identifier".to_string(), FieldValue::String(identifier));
        }

        if inner.kind() == "ref_full" {
            let (body_fields, _) = self.parse_block_body(inner);
            for entry in body_fields.entries() {
                fields.push(entry.key.clone(), entry.value.clone());
            }
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: "ref".to_string(),
            },
            id: EntityId { raw: id_text },
            title,
            fields,
            raw_body: None,
            span: self.span(node),
        });
    }

    fn parse_union_block(&mut self, node: Node) {
        let kind = node
            .child_by_field_name("kind")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();
        let name = node
            .child_by_field_name("name")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();

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
        fields.push("variants".to_string(), FieldValue::ReferenceList(variants));

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
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();

        let raw_body = self.extract_brace_body(node);
        let (fields, verify) = self.parse_block_body(node);
        let mut field_map = fields;
        if !verify.is_empty() {
            field_map.push("verify".to_string(), FieldValue::VerifyList(verify));
        }

        self.entities.push(Entity {
            kind: EntityKind {
                raw: "define".to_string(),
            },
            id: EntityId { raw: name },
            title: None,
            fields: field_map,
            raw_body,
            span: self.span(node),
        });
    }

    fn parse_use_import(&mut self, node: Node) {
        let path = node
            .child_by_field_name("path")
            .map(|n| self.text(n).to_string())
            .unwrap_or_default();

        let selected_ids = node.child_by_field_name("selective").map(|sel| {
            let mut ids = Vec::new();
            let mut cursor = sel.walk();
            for child in sel.children(&mut cursor) {
                if child.kind() == "identifier" {
                    ids.push(self.text(child).to_string());
                }
            }
            ids
        });

        self.imports.push(ImportDeclaration {
            path,
            selected_ids,
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
                    if let Some((key, value)) = self.parse_field(child) {
                        fields.push(key, value);
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

    fn parse_field(&self, node: Node) -> Option<(String, FieldValue)> {
        let key = node.child_by_field_name("key")?;
        let value = node.child_by_field_name("value")?;
        Some((self.text(key).to_string(), self.parse_value(value)))
    }

    fn parse_value(&self, node: Node) -> FieldValue {
        match node.kind() {
            "string" => FieldValue::String(self.unquote(node)),
            "triple_quoted_string" => FieldValue::String(self.parse_triple_quoted(node)),
            "integer" => {
                let text = self.text(node);
                FieldValue::Integer(text.parse().unwrap_or(0))
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

    fn parse_list(&self, node: Node) -> FieldValue {
        let mut has_string = false;
        let mut items = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => items.push(self.text(child).to_string()),
                "string" => {
                    has_string = true;
                    items.push(self.unquote(child));
                }
                "scheme_ref_id" => items.push(self.text(child).to_string()),
                "integer" => items.push(self.text(child).to_string()),
                _ => {}
            }
        }

        if has_string {
            FieldValue::StringList(items)
        } else {
            FieldValue::ReferenceList(items)
        }
    }

    fn parse_nested_block(&self, node: Node) -> FieldValue {
        let mut fields = FieldMap::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "field" {
                if let Some((key, value)) = self.parse_field(child) {
                    fields.push(key, value);
                }
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

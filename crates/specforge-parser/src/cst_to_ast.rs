use crate::ast::{AstEntity, ParseError, SpecFile, UseImport};
use crate::dedent::dedent_triple_quoted;
use specforge_common::{
    CustomEntityDef, CustomFieldDef, CustomFieldType,
    EntityId, EntityKind, FieldMap, FieldValue, Scenario, ScenarioStep, ScenarioStepKind,
    SourceSpan, VerifyKind, VerifyStatement,
};
use tree_sitter::{Node, Parser};

/// Parse a `.spec` file source string into a `SpecFile` AST.
///
/// Uses tree-sitter for CST parsing, then walks the CST to build the AST.
/// Collects all parse errors (INV-SF-2: multi-error collection).
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
        custom_defs: Vec::new(),
        errors: Vec::new(),
    };

    walk_root(&mut ctx, root);

    SpecFile {
        path: file_path.to_string(),
        imports: ctx.imports,
        entities: ctx.entities,
        custom_defs: ctx.custom_defs,
        errors: ctx.errors,
    }
}

struct ParseContext<'a> {
    source: &'a str,
    file_path: &'a str,
    imports: Vec<UseImport>,
    entities: Vec<AstEntity>,
    custom_defs: Vec<CustomEntityDef>,
    errors: Vec<ParseError>,
}

impl<'a> ParseContext<'a> {
    fn node_text(&self, node: Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    fn span(&self, node: Node) -> SourceSpan {
        let start = node.start_position();
        let end = node.end_position();
        SourceSpan::new(
            self.file_path,
            start.row as u32 + 1,
            start.column as u32 + 1,
            end.row as u32 + 1,
            end.column as u32 + 1,
        )
    }

    fn error(&mut self, node: Node, message: impl Into<String>) {
        self.errors.push(ParseError {
            message: message.into(),
            span: self.span(node),
        });
    }
}

fn walk_root(ctx: &mut ParseContext, root: Node) {
    // Pass 1: collect define blocks to know which keywords are custom entities
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "define_block" {
            parse_define_block(ctx, child);
        }
    }

    // Build the set of custom entity names for pass 2
    let custom_names: std::collections::HashSet<String> = ctx
        .custom_defs
        .iter()
        .map(|d| d.name.clone())
        .collect();

    // Pass 2: parse all blocks (define_block is skipped since already processed)
    let mut cursor2 = root.walk();
    for child in root.children(&mut cursor2) {
        match child.kind() {
            "use_import" => parse_use_import(ctx, child),
            "comment" => {} // skip
            "ERROR" => {
                // Try to parse as a custom entity instance first
                if !try_parse_error_as_custom_entity(ctx, child, &custom_names) {
                    ctx.error(child, format!("syntax error: {}", ctx.node_text(child)));
                }
            }
            "define_block" => {} // already processed in pass 1
            kind if kind.ends_with("_block") => parse_block(ctx, child),
            _ => {} // skip other node types
        }
    }
}

fn parse_use_import(ctx: &mut ParseContext, node: Node) {
    let path_node = node.child_by_field_name("path");
    let path = path_node
        .map(|n| ctx.node_text(n).to_string())
        .unwrap_or_default();

    let selective = node.child_by_field_name("selective").map(|sel_node| {
        let mut ids = Vec::new();
        let mut cursor = sel_node.walk();
        for child in sel_node.children(&mut cursor) {
            if child.kind() == "identifier" {
                ids.push(ctx.node_text(child).to_string());
            }
        }
        ids
    });

    ctx.imports.push(UseImport {
        path,
        selective,
        span: ctx.span(node),
    });
}

fn parse_block(ctx: &mut ParseContext, node: Node) {
    let kind_str = node.kind();

    // Map CST block type to EntityKind
    let kind = match kind_str {
        "spec_block" => EntityKind::Spec,
        "invariant_block" => EntityKind::Invariant,
        "behavior_block" => EntityKind::Behavior,
        "feature_block" => EntityKind::Feature,
        "event_block" => EntityKind::Event,
        "type_block" => return parse_type_block(ctx, node),
        "port_block" => EntityKind::Port,
        "ref_block" => return parse_ref_block(ctx, node),
        "capability_block" => EntityKind::Capability,
        "deliverable_block" => EntityKind::Deliverable,
        "roadmap_block" => EntityKind::Roadmap,
        "library_block" => EntityKind::Library,
        "glossary_block" => EntityKind::Glossary,
        "decision_block" => EntityKind::Decision,
        "constraint_block" => EntityKind::Constraint,
        "failure_mode_block" => EntityKind::FailureMode,
        "define_block" => return parse_define_block(ctx, node),
        "qualified_entity_block" => return parse_qualified_entity_block(ctx, node),
        _ => {
            ctx.error(node, format!("unknown block type: {kind_str}"));
            return;
        }
    };

    // Extract ID and title based on entity kind
    let (id, title) = match kind {
        EntityKind::Spec => {
            let name = node
                .child_by_field_name("name")
                .map(|n| unquote(ctx.node_text(n)));
            (
                EntityId::Named { name: "spec".to_string() },
                name,
            )
        }
        EntityKind::Glossary => (EntityId::Named { name: "glossary".to_string() }, None),
        EntityKind::Port => {
            let name = node
                .child_by_field_name("name")
                .map(|n| ctx.node_text(n).to_string());
            (
                EntityId::Named { name: name.clone().unwrap_or_default() },
                name,
            )
        }
        _ => {
            let id_node = node.child_by_field_name("id");
            let id = id_node
                .map(|n| EntityId::parse(ctx.node_text(n)))
                .unwrap_or_else(|| EntityId::Named { name: "unknown".to_string() });
            let title = node
                .child_by_field_name("title")
                .map(|n| unquote(ctx.node_text(n)));
            (id, title)
        }
    };

    // Parse fields
    let fields = parse_fields(ctx, node);

    ctx.entities.push(AstEntity {
        kind,
        id,
        title,
        fields,
        span: ctx.span(node),
    });
}

/// Try to parse an ERROR node as one or more custom entity instances.
///
/// When tree-sitter encounters `keyword id [title] { fields }` where `keyword`
/// is not a built-in entity type, it produces an ERROR node with flat children.
/// Multiple consecutive custom entity blocks may be grouped into a single ERROR.
/// This function splits them by detecting entity boundaries (keyword identifiers
/// at column 0) and reconstructs each entity from the flat children.
///
/// Returns `true` if at least one entity was parsed, `false` to fall through.
fn try_parse_error_as_custom_entity(
    ctx: &mut ParseContext,
    error_node: Node,
    custom_names: &std::collections::HashSet<String>,
) -> bool {
    let mut cursor = error_node.walk();
    let named_children: Vec<Node> = error_node.named_children(&mut cursor).collect();

    // Need at least keyword + id
    if named_children.len() < 2 {
        return false;
    }

    // First named child must be an identifier matching a custom entity name
    if named_children[0].kind() != "identifier" {
        return false;
    }
    if !custom_names.contains(ctx.node_text(named_children[0])) {
        return false;
    }

    // Find entity boundaries: each starts with a custom keyword at column 0
    // followed by an identifier (the entity id).
    let mut entity_starts: Vec<usize> = Vec::new();
    for (i, child) in named_children.iter().enumerate() {
        if child.kind() == "identifier"
            && child.start_position().column == 0
            && custom_names.contains(ctx.node_text(*child))
            && i + 1 < named_children.len()
            && named_children[i + 1].kind() == "identifier"
        {
            entity_starts.push(i);
        }
    }

    if entity_starts.is_empty() {
        return false;
    }

    // Parse each entity from its slice of named children
    for (idx, &start) in entity_starts.iter().enumerate() {
        let end = if idx + 1 < entity_starts.len() {
            entity_starts[idx + 1]
        } else {
            named_children.len()
        };
        parse_custom_entity_from_children(ctx, &named_children[start..end], error_node);
    }

    true
}

/// Parse a single custom entity instance from a slice of flat named children.
///
/// Expected layout: `[keyword, id, optional_title, key1, val1, key2, val2, ...]`
fn parse_custom_entity_from_children(
    ctx: &mut ParseContext,
    children: &[Node],
    span_node: Node,
) {
    if children.len() < 2 {
        return;
    }

    let keyword = ctx.node_text(children[0]).to_string();
    let id = EntityId::parse(ctx.node_text(children[1]));

    // Optional title (third child is a string)
    let (title, field_start) = if children.len() > 2 && children[2].kind() == "string" {
        (Some(unquote(ctx.node_text(children[2]))), 3)
    } else {
        (None, 2)
    };

    // Parse remaining children as flat key-value pairs
    let mut fields = FieldMap::new();
    let mut i = field_start;
    while i + 1 < children.len() {
        let key_node = children[i];
        let val_node = children[i + 1];
        if key_node.kind() == "identifier" {
            let key = ctx.node_text(key_node).to_string();
            let value = parse_value(ctx, val_node);
            fields.insert(key, value);
            i += 2;
        } else {
            i += 1;
        }
    }

    ctx.entities.push(AstEntity {
        kind: EntityKind::Custom(keyword),
        id,
        title,
        fields,
        span: ctx.span(span_node),
    });
}

fn parse_type_block(ctx: &mut ParseContext, node: Node) {
    // type_block wraps either type_struct or type_union
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_struct" | "type_union" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| ctx.node_text(n).to_string())
                    .unwrap_or_default();

                let mut fields = FieldMap::new();

                if child.kind() == "type_union" {
                    // Collect union variants
                    let mut variants_cursor = child.walk();
                    for vc in child.children(&mut variants_cursor) {
                        if vc.kind() == "union_variants" {
                            let mut var_cursor = vc.walk();
                            let variant_names: Vec<String> = vc
                                .children(&mut var_cursor)
                                .filter(|c| c.kind() == "identifier")
                                .map(|c| ctx.node_text(c).to_string())
                                .collect();
                            fields.insert(
                                "variants",
                                FieldValue::StringList(variant_names),
                            );
                        }
                    }
                } else {
                    // Collect struct fields as nested entries
                    let mut field_entries = FieldMap::new();
                    let mut struct_cursor = child.walk();
                    for field_node in child.children(&mut struct_cursor) {
                        if field_node.kind() == "type_field" {
                            let field_name = field_node
                                .child_by_field_name("name")
                                .map(|n| ctx.node_text(n).to_string())
                                .unwrap_or_default();
                            let field_type = field_node
                                .child_by_field_name("type")
                                .map(|n| extract_type_expr(ctx, n))
                                .unwrap_or_default();

                            // Collect annotations
                            let mut annotations = Vec::new();
                            let mut ann_cursor = field_node.walk();
                            for ann in field_node.children(&mut ann_cursor) {
                                if ann.kind() == "annotation" {
                                    annotations.push(ctx.node_text(ann).to_string());
                                }
                            }

                            let mut entry = FieldMap::new();
                            entry.insert("type", FieldValue::String(field_type));
                            if !annotations.is_empty() {
                                entry.insert(
                                    "annotations",
                                    FieldValue::StringList(annotations),
                                );
                            }
                            field_entries.insert(field_name, FieldValue::Block(entry));
                        }
                    }
                    fields.insert("fields", FieldValue::Block(field_entries));
                }

                ctx.entities.push(AstEntity {
                    kind: EntityKind::TypeDef,
                    id: EntityId::Named { name: name.clone() },
                    title: Some(name),
                    fields,
                    span: ctx.span(child),
                });
            }
            _ => {}
        }
    }
}

fn parse_ref_block(ctx: &mut ParseContext, node: Node) {
    // ref_block wraps either ref_inline or ref_full
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "ref_inline" | "ref_full" => {
                let id = child
                    .child_by_field_name("id")
                    .map(|n| EntityId::parse(ctx.node_text(n)))
                    .unwrap_or_else(|| EntityId::Named { name: "unknown".to_string() });
                let title = child
                    .child_by_field_name("title")
                    .map(|n| unquote(ctx.node_text(n)));

                let fields = if child.kind() == "ref_full" {
                    parse_fields(ctx, child)
                } else {
                    FieldMap::new()
                };

                ctx.entities.push(AstEntity {
                    kind: EntityKind::Ref,
                    id,
                    title,
                    fields,
                    span: ctx.span(child),
                });
            }
            _ => {}
        }
    }
}

fn parse_fields(ctx: &mut ParseContext, node: Node) -> FieldMap {
    let mut fields = FieldMap::new();
    let mut verify_list: Vec<VerifyStatement> = Vec::new();
    let mut scenario_list: Vec<Scenario> = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "key_value" => {
                let key = child
                    .child_by_field_name("key")
                    .map(|n| ctx.node_text(n).to_string())
                    .unwrap_or_default();
                let value = child
                    .child_by_field_name("value")
                    .map(|n| parse_value(ctx, n))
                    .unwrap_or(FieldValue::String(String::new()));
                fields.insert(key, value);
            }
            "verify_statement" => {
                let kind_str = child
                    .child_by_field_name("kind")
                    .map(|n| ctx.node_text(n))
                    .unwrap_or("unit");
                let kind = kind_str.parse().unwrap_or(VerifyKind::Unit);
                let desc = child
                    .child_by_field_name("description")
                    .map(|n| unquote(ctx.node_text(n)))
                    .unwrap_or_default();
                verify_list.push(VerifyStatement {
                    kind,
                    description: desc,
                });
            }
            "scenario_block" => {
                scenario_list.push(parse_scenario(ctx, child));
            }
            "persona_def" | "surface_def" | "providers_block" | "coverage_block"
            | "gen_block" => {
                parse_spec_sub_block(ctx, child, &mut fields);
            }
            "term_def" => {
                parse_term_def(ctx, child, &mut fields);
            }
            "method_def" => {
                parse_method_def(ctx, child, &mut fields);
            }
            "ERROR" => {
                ctx.error(child, format!("syntax error: {}", ctx.node_text(child)));
            }
            _ => {}
        }
    }

    if !verify_list.is_empty() {
        fields.insert("verify", FieldValue::VerifyList(verify_list));
    }
    if !scenario_list.is_empty() {
        fields.insert("scenario", FieldValue::ScenarioList(scenario_list));
    }

    fields
}

fn parse_scenario(ctx: &mut ParseContext, node: Node) -> Scenario {
    let title = node
        .child_by_field_name("title")
        .map(|n| unquote(ctx.node_text(n)))
        .unwrap_or_default();

    let mut steps = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = match child.kind() {
            "given_step" => ScenarioStepKind::Given,
            "when_step" => ScenarioStepKind::When,
            "then_step" => ScenarioStepKind::Then,
            _ => continue,
        };
        let description = child
            .child_by_field_name("description")
            .map(|n| unquote(ctx.node_text(n)))
            .unwrap_or_default();
        steps.push(ScenarioStep {
            kind,
            description,
            span: ctx.span(child),
        });
    }

    Scenario {
        title,
        steps,
        span: ctx.span(node),
    }
}

fn parse_value(ctx: &mut ParseContext, node: Node) -> FieldValue {
    match node.kind() {
        "triple_quoted_string" => {
            FieldValue::String(dedent_triple_quoted(ctx.node_text(node)))
        }
        "string" => FieldValue::String(unquote(ctx.node_text(node))),
        "integer" => {
            let text = ctx.node_text(node);
            FieldValue::Integer(text.parse().unwrap_or(0))
        }
        "boolean" => FieldValue::Bool(ctx.node_text(node) == "true"),
        "date_literal" => FieldValue::String(ctx.node_text(node).to_string()),
        "scheme_ref_id" => FieldValue::Reference(EntityId::parse(ctx.node_text(node))),
        "identifier" => FieldValue::Enum(ctx.node_text(node).to_string()),
        "array_identifier" => {
            // e.g., `string[]` — parse as enum with array suffix
            FieldValue::Enum(ctx.node_text(node).to_string())
        }
        "list" => parse_list(ctx, node),
        "nested_block" => {
            let inner = parse_nested_block_fields(ctx, node);
            FieldValue::Block(inner)
        }
        _ => {
            ctx.error(node, format!("unexpected value type: {}", node.kind()));
            FieldValue::String(ctx.node_text(node).to_string())
        }
    }
}

fn parse_list(ctx: &mut ParseContext, node: Node) -> FieldValue {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();

    // Collect list items by type
    let mut refs = Vec::new();
    let mut strings = Vec::new();
    let mut integers = Vec::new();

    for child in &children {
        match child.kind() {
            // With named identifiers, identifiers in lists are entity references
            "identifier" => refs.push(EntityId::parse(ctx.node_text(*child))),
            "scheme_ref_id" => refs.push(EntityId::parse(ctx.node_text(*child))),
            "string" => strings.push(unquote(ctx.node_text(*child))),
            "integer" => integers.push(ctx.node_text(*child).parse::<i64>().unwrap_or(0)),
            "ERROR" => {
                ctx.error(
                    *child,
                    format!("syntax error in list: `{}`", ctx.node_text(*child)),
                );
            }
            _ => {} // brackets, commas, whitespace
        }
    }

    // Determine list type by what we found
    if !refs.is_empty() {
        FieldValue::ReferenceList(refs)
    } else if !strings.is_empty() {
        FieldValue::StringList(strings)
    } else if !integers.is_empty() {
        FieldValue::StringList(integers.iter().map(|i| i.to_string()).collect())
    } else {
        FieldValue::ReferenceList(Vec::new())
    }
}

fn parse_nested_block_fields(ctx: &mut ParseContext, node: Node) -> FieldMap {
    let mut fields = FieldMap::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "key_value" {
            let key = child
                .child_by_field_name("key")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let value = child
                .child_by_field_name("value")
                .map(|n| parse_value(ctx, n))
                .unwrap_or(FieldValue::String(String::new()));
            fields.insert(key, value);
        }
    }
    fields
}

fn parse_spec_sub_block(ctx: &mut ParseContext, node: Node, fields: &mut FieldMap) {
    match node.kind() {
        "persona_def" => {
            let id = node
                .child_by_field_name("id")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let display = node
                .child_by_field_name("display_name")
                .map(|n| unquote(ctx.node_text(n)))
                .unwrap_or_default();
            let mut persona_fields = FieldMap::new();
            persona_fields.insert("display_name", FieldValue::String(display));
            // Parse inner key-values
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "key_value" {
                    let key = child
                        .child_by_field_name("key")
                        .map(|n| ctx.node_text(n).to_string())
                        .unwrap_or_default();
                    let value = child
                        .child_by_field_name("value")
                        .map(|n| parse_value(ctx, n))
                        .unwrap_or(FieldValue::String(String::new()));
                    persona_fields.insert(key, value);
                }
            }
            fields.insert(format!("persona:{id}"), FieldValue::Block(persona_fields));
        }
        "surface_def" => {
            let id = node
                .child_by_field_name("id")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let display = node
                .child_by_field_name("display_name")
                .map(|n| unquote(ctx.node_text(n)))
                .unwrap_or_default();
            let mut surface_fields = FieldMap::new();
            surface_fields.insert("display_name", FieldValue::String(display));
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "key_value" {
                    let key = child
                        .child_by_field_name("key")
                        .map(|n| ctx.node_text(n).to_string())
                        .unwrap_or_default();
                    let value = child
                        .child_by_field_name("value")
                        .map(|n| parse_value(ctx, n))
                        .unwrap_or(FieldValue::String(String::new()));
                    surface_fields.insert(key, value);
                }
            }
            fields.insert(format!("surface:{id}"), FieldValue::Block(surface_fields));
        }
        "providers_block" => {
            let mut providers = FieldMap::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "provider_instance" {
                    let scheme = child
                        .child_by_field_name("scheme")
                        .map(|n| ctx.node_text(n).to_string())
                        .unwrap_or_default();
                    let alias = child
                        .child_by_field_name("alias")
                        .map(|n| unquote(ctx.node_text(n)))
                        .unwrap_or_default();
                    let mut inner = FieldMap::new();
                    inner.insert("alias", FieldValue::String(alias));
                    let mut inner_cursor = child.walk();
                    for kv in child.children(&mut inner_cursor) {
                        if kv.kind() == "key_value" {
                            let key = kv
                                .child_by_field_name("key")
                                .map(|n| ctx.node_text(n).to_string())
                                .unwrap_or_default();
                            let value = kv
                                .child_by_field_name("value")
                                .map(|n| parse_value(ctx, n))
                                .unwrap_or(FieldValue::String(String::new()));
                            inner.insert(key, value);
                        }
                    }
                    providers.insert(scheme, FieldValue::Block(inner));
                }
            }
            fields.insert("providers", FieldValue::Block(providers));
        }
        "coverage_block" => {
            let inner = parse_nested_block_fields(ctx, node);
            fields.insert("coverage", FieldValue::Block(inner));
        }
        "gen_block" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let inner = parse_nested_block_fields(ctx, node);
            fields.insert(format!("gen:{name}"), FieldValue::Block(inner));
        }
        _ => {}
    }
}

fn parse_term_def(ctx: &mut ParseContext, node: Node, fields: &mut FieldMap) {
    let name = node
        .child_by_field_name("name")
        .map(|n| unquote(ctx.node_text(n)))
        .unwrap_or_default();
    let mut term_fields = FieldMap::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "key_value" {
            let key = child
                .child_by_field_name("key")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let value = child
                .child_by_field_name("value")
                .map(|n| parse_value(ctx, n))
                .unwrap_or(FieldValue::String(String::new()));
            term_fields.insert(key, value);
        }
    }
    fields.insert(format!("term:{name}"), FieldValue::Block(term_fields));
}

fn parse_method_def(ctx: &mut ParseContext, node: Node, fields: &mut FieldMap) {
    let name = node
        .child_by_field_name("name")
        .map(|n| ctx.node_text(n).to_string())
        .unwrap_or_default();

    // Extract method signature as opaque string (Phase 1: no deep parsing)
    let signature = ctx.node_text(node).to_string();
    fields.insert(format!("method:{name}"), FieldValue::String(signature));
}

fn extract_type_expr(ctx: &ParseContext, node: Node) -> String {
    // For type expressions, return the raw text (Phase 1 simplicity)
    ctx.node_text(node).to_string()
}

/// Parse a `define` block into a `CustomEntityDef`.
///
/// Syntax:
/// ```text
/// define my_entity {
///   testable true
///   description string
///   severity string
/// }
/// ```
fn parse_define_block(ctx: &mut ParseContext, node: Node) {
    let name = node
        .child_by_field_name("name")
        .map(|n| ctx.node_text(n).to_string())
        .unwrap_or_default();

    if name.is_empty() {
        ctx.error(node, "define block must have a name");
        return;
    }

    let mut testable = false;
    let mut fields = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "key_value" {
            let key = child
                .child_by_field_name("key")
                .map(|n| ctx.node_text(n).to_string())
                .unwrap_or_default();
            let value_node = child.child_by_field_name("value");

            match key.as_str() {
                "testable" => {
                    if let Some(vn) = value_node {
                        testable = ctx.node_text(vn) == "true";
                    }
                }
                _ => {
                    // Treat as a field definition: key is field name, value is field type
                    if let Some(vn) = value_node {
                        let type_str = ctx.node_text(vn);
                        let field_type =
                            CustomFieldType::from_str_opt(type_str).unwrap_or(CustomFieldType::String);
                        fields.push(CustomFieldDef {
                            name: key,
                            field_type,
                            required: false,
                        });
                    }
                }
            }
        }
    }

    ctx.custom_defs.push(CustomEntityDef {
        name,
        testable,
        fields,
    });
}

/// Parse a `@plugin/kind` qualified entity block.
///
/// These blocks use the syntax `@plugin/kind id "Title" { ... }` to explicitly
/// reference an entity kind from a specific plugin, resolving ambiguity when
/// multiple plugins register the same kind name.
fn parse_qualified_entity_block(ctx: &mut ParseContext, node: Node) {
    let qualified_node = node.child_by_field_name("qualified_keyword");
    let (plugin, kind_name) = if let Some(qn) = qualified_node {
        let plugin = qn
            .child_by_field_name("plugin")
            .map(|n| ctx.node_text(n).to_string())
            .unwrap_or_default();
        let kind = qn
            .child_by_field_name("kind")
            .map(|n| ctx.node_text(n).to_string())
            .unwrap_or_default();
        (plugin, kind)
    } else {
        ctx.error(node, "qualified entity block must have @plugin/kind keyword");
        return;
    };

    if plugin.is_empty() || kind_name.is_empty() {
        ctx.error(node, "qualified entity keyword must have both plugin and kind");
        return;
    }

    let qualified = format!("@{plugin}/{kind_name}");

    let id_node = node.child_by_field_name("id");
    let title_node = node.child_by_field_name("title");

    let id = id_node
        .map(|n| ctx.node_text(n).to_string())
        .unwrap_or_default();
    let title = title_node.map(|n| unquote(ctx.node_text(n)));

    if id.is_empty() {
        ctx.error(node, "qualified entity block must have an identifier");
        return;
    }

    let entity_id = EntityId::parse(&id);
    let fields = parse_fields(ctx, node);

    ctx.entities.push(AstEntity {
        kind: EntityKind::Custom(qualified),
        id: entity_id,
        title,
        fields,
        span: ctx.span(node),
    });
}

fn unquote(s: &str) -> String {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_file() {
        let result = parse("", "empty.spec");
        assert!(result.entities.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn parse_comment_only() {
        let result = parse("// just a comment\n", "comments.spec");
        assert!(result.entities.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn parse_use_imports() {
        let source = "use invariants/core\nuse types/errors\n";
        let result = parse(source, "test.spec");
        assert_eq!(result.imports.len(), 2);
        assert_eq!(result.imports[0].path, "invariants/core");
        assert_eq!(result.imports[1].path, "types/errors");
        assert!(result.imports[0].selective.is_none());
    }

    #[test]
    fn parse_selective_import() {
        let source = "use invariants/core { spec_root_singleton, multi_error_collection }\n";
        let result = parse(source, "test.spec");
        assert_eq!(result.imports.len(), 1);
        let sel = result.imports[0].selective.as_ref().unwrap();
        assert_eq!(sel, &["spec_root_singleton", "multi_error_collection"]);
    }

    #[test]
    fn parse_invariant() {
        let source = r#"invariant data_persistence "Test Invariant" {
  guarantee """
    Must be true.
  """
  enforced_by [Parser, Resolver]
  risk medium
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.errors.len(), 0);

        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Invariant);
        assert_eq!(entity.id, EntityId::parse("data_persistence"));
        assert_eq!(entity.title.as_deref(), Some("Test Invariant"));

        // Check fields
        assert!(entity.fields.get("guarantee").is_some());
        assert!(entity.fields.get("risk").is_some());

        if let Some(FieldValue::Enum(risk)) = entity.fields.get("risk") {
            assert_eq!(risk, "medium");
        } else {
            panic!("risk should be Enum");
        }
    }

    #[test]
    fn parse_behavior_with_verify() {
        let source = r#"behavior parse_spec "Parse Spec" {
  invariants [multi_error_collection]
  contract """
    Given a valid .spec file, produce an AST.
  """
  verify unit "parse valid file"
  verify integration "end to end"
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Behavior);

        if let Some(FieldValue::VerifyList(verifies)) = entity.fields.get("verify") {
            assert_eq!(verifies.len(), 2);
            assert_eq!(verifies[0].kind, VerifyKind::Unit);
            assert_eq!(verifies[0].description, "parse valid file");
            assert_eq!(verifies[1].kind, VerifyKind::Integration);
        } else {
            panic!("verify should be VerifyList");
        }
    }

    #[test]
    fn parse_type_struct() {
        let source = r#"type User {
  id string @readonly
  email string @unique
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::TypeDef);
        assert_eq!(entity.id, EntityId::Named { name: "User".to_string() });
    }

    #[test]
    fn parse_type_union() {
        let source = "type EntityKind = spec | invariant | behavior | feature\n";
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::TypeDef);

        if let Some(FieldValue::StringList(variants)) = entity.fields.get("variants") {
            assert_eq!(variants, &["spec", "invariant", "behavior", "feature"]);
        } else {
            panic!("variants should be StringList");
        }
    }

    #[test]
    fn parse_port_with_methods() {
        let source = r#"port FileSystem {
  direction outbound
  category "io/filesystem"
  method readFile(path: string) -> Result<string, Error>
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Port);
        assert_eq!(entity.id, EntityId::Named { name: "FileSystem".to_string() });
        assert!(entity.fields.get("method:readFile").is_some());
    }

    #[test]
    fn parse_event_with_payload() {
        let source = r#"event file_parsed "File Parsed" {
  trigger parse_spec
  channel "compiler.file_parsed"
  payload {
    filePath string
    entityCount integer
  }
  consumers [build_graph]
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Event);

        if let Some(FieldValue::Block(payload)) = entity.fields.get("payload") {
            assert!(payload.get("filePath").is_some());
        } else {
            panic!("payload should be Block");
        }
    }

    #[test]
    fn parse_spec_block() {
        let source = r#"spec "specforge" {
  version "1.0"
  plugins ["@specforge/product"]
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Spec);
    }

    #[test]
    fn parse_failure_mode_with_nested() {
        let source = r#"failure_mode FM-SF-001 "Stale Graph" {
  invariant INV-SF-7
  severity 8
  occurrence 3
  detection 4
  rpn 96
  post_mitigation {
    severity 8
    occurrence 1
    detection 2
    rpn 16
  }
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::FailureMode);

        if let Some(FieldValue::Block(pm)) = entity.fields.get("post_mitigation") {
            if let Some(FieldValue::Integer(sev)) = pm.get("severity") {
                assert_eq!(*sev, 8);
            } else {
                panic!("post_mitigation.severity should be Integer");
            }
        } else {
            panic!("post_mitigation should be Block");
        }
    }

    #[test]
    fn error_recovery() {
        let source = r#"invariant INV-SF-1 "Good" {
  guarantee """OK"""
}

thingamajig !!!

invariant INV-SF-2 "Also Good" {
  guarantee """Also OK"""
}
"#;
        let result = parse(source, "test.spec");
        // Should still parse the valid blocks despite the error in the middle
        assert!(result.entities.len() >= 1, "should parse at least one entity");
        assert!(!result.errors.is_empty(), "should have parse errors");
    }

    #[test]
    fn parse_feature() {
        let source = r#"feature spec_parsing "Parsing" {
  behaviors [parse_spec_files, walk_cst_to_ast]
  problem """
    Need to parse spec files.
  """
  solution """
    Use tree-sitter.
  """
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Feature);

        if let Some(FieldValue::ReferenceList(refs)) = entity.fields.get("behaviors") {
            assert_eq!(refs.len(), 2);
        } else {
            panic!("behaviors should be ReferenceList");
        }
    }

    #[test]
    fn parse_glossary() {
        let source = r#"glossary {
  term "entity" {
    definition """A named block."""
  }
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Glossary);
        assert!(entity.fields.get("term:entity").is_some());
    }

    #[test]
    fn parse_behavior_with_scenario() {
        let source = r#"behavior login_flow "Login Flow" {
  contract """The system authenticates the user."""

  scenario "successful login" {
    given "a registered user with valid credentials"
    when "the user submits login form"
    then "the system grants access"
    then "a session token is returned"
  }

  scenario "failed login" {
    given "a user with invalid credentials"
    when "the user submits login form"
    then "the system denies access"
  }
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.errors.len(), 0);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Behavior);

        if let Some(FieldValue::ScenarioList(scenarios)) = entity.fields.get("scenario") {
            assert_eq!(scenarios.len(), 2);
            assert_eq!(scenarios[0].title, "successful login");
            assert_eq!(scenarios[0].steps.len(), 4);
            assert_eq!(scenarios[0].steps[0].kind, ScenarioStepKind::Given);
            assert_eq!(scenarios[0].steps[1].kind, ScenarioStepKind::When);
            assert_eq!(scenarios[0].steps[2].kind, ScenarioStepKind::Then);
            assert_eq!(scenarios[0].steps[3].kind, ScenarioStepKind::Then);
            assert_eq!(scenarios[0].steps[0].description, "a registered user with valid credentials");

            assert_eq!(scenarios[1].title, "failed login");
            assert_eq!(scenarios[1].steps.len(), 3);
        } else {
            panic!("scenario should be ScenarioList");
        }
    }

    #[test]
    fn parse_capability_with_scenario() {
        let source = r#"capability validate_spec_files "Validate Spec Files" {
  persona developer
  surface [cli]
  features [spec_file_parsing]

  scenario "developer validates clean spec" {
    given "a project with valid spec files"
    when "developer runs specforge check"
    then "exit code is 0"
  }
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.errors.len(), 0);
        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Capability);

        if let Some(FieldValue::ScenarioList(scenarios)) = entity.fields.get("scenario") {
            assert_eq!(scenarios.len(), 1);
            assert_eq!(scenarios[0].title, "developer validates clean spec");
            assert_eq!(scenarios[0].steps.len(), 3);
        } else {
            panic!("scenario should be ScenarioList");
        }
    }

    #[test]
    fn parse_list_error_node_captured() {
        let source = r#"behavior validate_input "Validate Input" {
  contract """The system validates user input."""
  invariants [BEH-SF-010]
}"#;
        let result = parse(source, "test.spec");
        assert!(
            !result.errors.is_empty(),
            "Should have parse errors for invalid list syntax"
        );
        let error_messages: Vec<&str> = result.errors.iter().map(|e| e.message.as_str()).collect();
        assert!(
            error_messages.iter().any(|m| m.contains("syntax error")),
            "Should contain a syntax error message, got: {error_messages:?}"
        );
    }

    #[test]
    fn triple_quoted_dedent() {
        let source = r#"invariant INV-SF-1 "Test" {
  guarantee """
    Line one.
    Line two.
  """
}"#;
        let result = parse(source, "test.spec");
        if let Some(FieldValue::String(s)) = result.entities[0].fields.get("guarantee") {
            assert_eq!(s, "Line one.\nLine two.");
        } else {
            panic!("guarantee should be dedented String");
        }
    }

    #[test]
    fn parse_define_block() {
        let source = r#"define risk_register {
  testable true
  severity integer
  description string
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 0, "define blocks are not entities");
        assert_eq!(result.custom_defs.len(), 1);
        assert_eq!(result.custom_defs[0].name, "risk_register");
        assert!(result.custom_defs[0].testable);
        assert_eq!(result.custom_defs[0].fields.len(), 2);
        assert_eq!(result.custom_defs[0].fields[0].name, "severity");
        assert_eq!(result.custom_defs[0].fields[0].field_type, CustomFieldType::Integer);
        assert_eq!(result.custom_defs[0].fields[1].name, "description");
        assert_eq!(result.custom_defs[0].fields[1].field_type, CustomFieldType::String);
    }

    #[test]
    fn parse_custom_entity_instance() {
        let source = r#"define risk_register {
  severity integer
  description string
}

risk_register auth_risk "Auth Risk" {
  severity 8
  description "Authentication bypass"
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.errors.len(), 0, "no parse errors: {:?}", result.errors);
        assert_eq!(result.custom_defs.len(), 1);
        assert_eq!(result.entities.len(), 1);

        let entity = &result.entities[0];
        assert_eq!(entity.kind, EntityKind::Custom("risk_register".to_string()));
        assert_eq!(entity.id, EntityId::parse("auth_risk"));
        assert_eq!(entity.title.as_deref(), Some("Auth Risk"));

        if let Some(FieldValue::Integer(sev)) = entity.fields.get("severity") {
            assert_eq!(*sev, 8);
        } else {
            panic!("severity should be Integer, got: {:?}", entity.fields.get("severity"));
        }

        if let Some(FieldValue::String(desc)) = entity.fields.get("description") {
            assert_eq!(desc, "Authentication bypass");
        } else {
            panic!("description should be String");
        }
    }

    #[test]
    fn parse_multiple_custom_entities() {
        let source = r#"define risk_register {
  severity integer
  description string
}

risk_register auth_risk "Auth Risk" {
  severity 8
  description "high risk"
}

risk_register data_risk "Data Risk" {
  severity 5
  description "medium risk"
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.errors.len(), 0, "no parse errors: {:?}", result.errors);
        assert_eq!(result.entities.len(), 2);

        assert_eq!(result.entities[0].id, EntityId::parse("auth_risk"));
        assert_eq!(result.entities[1].id, EntityId::parse("data_risk"));
    }

    #[test]
    fn parse_unknown_entity_type_is_error() {
        let source = r#"unknown_entity foo "Foo" {
  bar "baz"
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.entities.len(), 0);
        assert!(!result.errors.is_empty(), "should have a syntax error");
    }

    #[test]
    fn custom_entity_coexists_with_builtins() {
        let source = r#"define risk_register {
  severity integer
}

invariant data_integrity "Data Integrity" {
  guarantee """ok"""
}

risk_register auth_risk "Auth Risk" {
  severity 8
}

behavior validate_input "Validate Input" {
  contract """ok"""
}"#;
        let result = parse(source, "test.spec");
        assert_eq!(result.errors.len(), 0, "no parse errors: {:?}", result.errors);
        assert_eq!(result.entities.len(), 3);
        assert_eq!(result.entities[0].kind, EntityKind::Invariant);
        assert_eq!(result.entities[1].kind, EntityKind::Custom("risk_register".to_string()));
        assert_eq!(result.entities[2].kind, EntityKind::Behavior);
    }
}

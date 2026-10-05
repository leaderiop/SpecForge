//! One-shot generator (plan 03 T3): turns the builtins' hand-written
//! describe JSON into SDK builder calls, one `src/declaration.rs` per
//! extension. It reads the pinned wire answers
//! (`crates/specforge-component/tests/declarations/<dir>/`, byte-identical
//! to the JSON the extensions served) into the typed declaration, so every
//! descriptor field is destructured exhaustively: a field the generator
//! does not emit is a compile error here, and a value the builders cannot
//! spell (a non-canonical vocabulary name) is refused.
//!
//!   cargo run -p xtask --bin declaration-to-rust

use specforge_protocol_types::{
    AnalyzerDescriptor, CheckKind, CompilerPassDescriptor, ConstraintKind, DescribeResponse,
    EdgeTypeDescriptor, EntityEnhancementDescriptor, EntityKindDescriptor, ExtensionDeclaration,
    FeatureFlagDescriptor, FieldConstraintDescriptor, FieldDescriptor, FieldType,
    HandshakeResponse, ValidationRuleDescriptor, ValidationSeverity,
};
use std::fmt::Write as _;
use std::path::Path;

/// The builtins that served raw JSON, and the categories they served so.
const RAW: &[(&str, &str, &[&str])] = &[
    ("product", "@specforge/product", &CATEGORIES),
    ("software", "@specforge/software", &CATEGORIES),
    ("governance", "@specforge/governance", &CATEGORIES),
    // formal declares its passes with `c.pass` already.
    (
        "formal",
        "@specforge/formal",
        &[
            "entities",
            "edges",
            "shared_fields",
            "enhancements",
            "validation_rules",
            "feature_flags",
        ],
    ),
    ("rust", "@specforge/rust", &["analyzers"]),
    ("typescript", "@specforge/typescript", &["analyzers"]),
];

const CATEGORIES: [&str; 7] = [
    "entities",
    "edges",
    "shared_fields",
    "enhancements",
    "validation_rules",
    "passes",
    "feature_flags",
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for (dir, name, categories) in RAW {
        let pinned = root
            .join("crates/specforge-component/tests/declarations")
            .join(dir);
        let read = |file: &str| {
            std::fs::read_to_string(pinned.join(file))
                .unwrap_or_else(|e| panic!("{dir}/{file}: {e}"))
        };
        let handshake: HandshakeResponse = serde_json::from_str(&read("handshake.json")).unwrap();
        let declaration = ExtensionDeclaration::from_wire(
            handshake,
            |category| {
                Ok(serde_json::from_str::<DescribeResponse>(&read(&format!(
                    "describe_{category}.json"
                )))
                .unwrap())
            },
            |key| panic!("{dir}: unknown key {key:?}"),
        )
        .unwrap();
        let code = generate(name, &declaration, categories);
        let out = root.join("extensions").join(dir).join("src/declaration.rs");
        std::fs::write(&out, code).unwrap();
        println!("[ok]     {}", out.display());
    }
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn list(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|s| lit(s)).collect();
    format!("&[{}]", items.join(", "))
}

fn field_type(name: &str) -> String {
    let parsed = FieldType::parse(name).unwrap_or_else(|| panic!("unknown field type {name}"));
    assert_eq!(parsed.as_str(), name, "non-canonical field type {name}");
    format!("FieldType::{parsed:?}")
}

fn check_kind(name: &str) -> String {
    let parsed = CheckKind::parse(name).unwrap_or_else(|| panic!("unknown check {name}"));
    assert_eq!(parsed.as_str(), name, "non-canonical check {name}");
    format!("CheckKind::{parsed:?}")
}

fn constraint_kind(name: &str) -> String {
    let parsed =
        ConstraintKind::parse(name).unwrap_or_else(|| panic!("unknown constraint kind {name}"));
    assert_eq!(
        parsed.as_str(),
        name,
        "non-canonical constraint kind {name}"
    );
    format!("ConstraintKind::{parsed:?}")
}

/// `receiver.setter(..).setter(..);`, or nothing without setters.
fn chain(out: &mut String, indent: &str, receiver: &str, setters: &[String]) {
    if setters.is_empty() {
        return;
    }
    let joined = setters.join(&format!("\n{indent}    "));
    writeln!(out, "{indent}{receiver}{joined};").unwrap();
}

fn opt(setters: &mut Vec<String>, name: &str, value: &Option<String>) {
    if let Some(v) = value {
        setters.push(format!(".{name}({})", lit(v)));
    }
}

fn field(out: &mut String, indent: &str, receiver: &str, f: &FieldDescriptor) {
    let FieldDescriptor {
        name,
        field_type: ty,
        required,
        description,
        edge,
        target_kind,
        file_reference,
        default_value,
        enum_values,
        inverse_of,
        normative,
        exempts_obligations,
        headline,
        derived_from,
        proof_role,
    } = f;
    let mut s = vec![format!(".field_type({})", field_type(ty))];
    if *required {
        s.push(".required()".to_string());
    }
    opt(&mut s, "description", description);
    opt(&mut s, "edge", edge);
    opt(&mut s, "target_kind", target_kind);
    if *file_reference {
        s.push(".file_reference()".to_string());
    }
    opt(&mut s, "default_value", default_value);
    if !enum_values.is_empty() {
        s.push(format!(".enum_values({})", list(enum_values)));
    }
    opt(&mut s, "inverse_of", inverse_of);
    if *normative {
        s.push(".normative()".to_string());
    }
    if *exempts_obligations {
        s.push(".exempts_obligations()".to_string());
    }
    if *headline {
        s.push(".headline()".to_string());
    }
    opt(&mut s, "derived_from", derived_from);
    opt(&mut s, "proof_role", proof_role);
    writeln!(out, "{indent}{receiver}({}, |f| {{", lit(name)).unwrap();
    chain(out, &format!("{indent}    "), "f", &s);
    writeln!(out, "{indent}}});").unwrap();
}

fn kind(out: &mut String, k: &EntityKindDescriptor) {
    let EntityKindDescriptor {
        name,
        keyword,
        description,
        fields,
        testable,
        singleton,
        supports_verify,
        incremental,
        has_body_parser,
        open_fields,
        semantic_token,
        lsp_icon,
        dot_shape,
        dot_color,
        dot_fillcolor,
        verify_kinds,
        inference_guide,
        contract_target,
        declares_types,
        lifecycle_field,
    } = k;
    let mut s = Vec::new();
    opt(&mut s, "keyword", keyword);
    opt(&mut s, "description", description);
    if *testable {
        s.push(".testable(true)".to_string());
    }
    if *singleton {
        s.push(".singleton(true)".to_string());
    }
    if *supports_verify {
        s.push(".supports_verify(true)".to_string());
    }
    if let Some(i) = incremental {
        s.push(format!(".incremental({i})"));
    }
    if *has_body_parser {
        s.push(".has_body_parser()".to_string());
    }
    if *open_fields {
        s.push(".open_fields(true)".to_string());
    }
    opt(&mut s, "semantic_token", semantic_token);
    opt(&mut s, "lsp_icon", lsp_icon);
    opt(&mut s, "dot_shape", dot_shape);
    opt(&mut s, "dot_color", dot_color);
    opt(&mut s, "dot_fillcolor", dot_fillcolor);
    if !verify_kinds.is_empty() {
        s.push(format!(".verify_kinds({})", list(verify_kinds)));
    }
    opt(&mut s, "inference_guide", inference_guide);
    if *contract_target {
        s.push(".contract_target()".to_string());
    }
    if *declares_types {
        s.push(".declares_types()".to_string());
    }
    opt(&mut s, "lifecycle_field", lifecycle_field);
    writeln!(out, "    c.kind({}, |k| {{", lit(name)).unwrap();
    chain(out, "        ", "k", &s);
    for f in fields {
        field(out, "        ", "k.field", f);
    }
    writeln!(out, "    }});").unwrap();
}

fn edge_setters(e: &EdgeTypeDescriptor) -> Vec<String> {
    let EdgeTypeDescriptor {
        label: _,
        description,
        source_kind,
        target_kind,
        edge_style,
        edge_color,
        edge_arrowhead,
    } = e;
    let mut s = Vec::new();
    opt(&mut s, "description", description);
    opt(&mut s, "source_kind", source_kind);
    opt(&mut s, "target_kind", target_kind);
    opt(&mut s, "edge_style", edge_style);
    opt(&mut s, "edge_color", edge_color);
    opt(&mut s, "edge_arrowhead", edge_arrowhead);
    s
}

fn edge(out: &mut String, indent: &str, receiver: &str, e: &EdgeTypeDescriptor) {
    let s = edge_setters(e);
    if s.is_empty() {
        writeln!(out, "{indent}{receiver}({}, |_| {{}});", lit(&e.label)).unwrap();
        return;
    }
    writeln!(out, "{indent}{receiver}({}, |e| {{", lit(&e.label)).unwrap();
    chain(out, &format!("{indent}    "), "e", &s);
    writeln!(out, "{indent}}});").unwrap();
}

fn enhancement(out: &mut String, e: &EntityEnhancementDescriptor) {
    let EntityEnhancementDescriptor {
        target_kind,
        source_extension,
        fields,
        edge_types,
        verify_kinds,
    } = e;
    writeln!(
        out,
        "    c.enhance({}, {}, |e| {{",
        lit(target_kind),
        lit(source_extension)
    )
    .unwrap();
    if let Some(kinds) = verify_kinds {
        writeln!(out, "        e.verify_kinds({});", list(kinds)).unwrap();
    }
    for f in fields {
        field(out, "        ", "e.field", f);
    }
    for t in edge_types {
        edge(out, "        ", "e.edge_type", t);
    }
    writeln!(out, "    }});").unwrap();
}

fn rule(out: &mut String, r: &ValidationRuleDescriptor) {
    let ValidationRuleDescriptor {
        code,
        severity,
        message_template,
        check,
        target_kind,
        edge_type,
        field,
        constraint,
        wasm_function,
    } = r;
    let severity = match severity {
        ValidationSeverity::Error => "Error",
        ValidationSeverity::Warning => "Warning",
        ValidationSeverity::Info => "Info",
    };
    let mut s = vec![
        format!(".check({})", check_kind(check)),
        format!(".severity(ValidationSeverity::{severity})"),
        format!(".message_template({})", lit(message_template)),
    ];
    opt(&mut s, "target_kind", target_kind);
    opt(&mut s, "edge_type", edge_type);
    opt(&mut s, "field", field);
    opt(&mut s, "wasm_function", wasm_function);
    writeln!(out, "    c.rule({}, |r| {{", lit(code)).unwrap();
    chain(out, "        ", "r", &s);
    if let Some(FieldConstraintDescriptor {
        kind,
        pattern,
        values,
    }) = constraint
    {
        let mut c = vec![format!(".kind({})", constraint_kind(kind))];
        opt(&mut c, "pattern", pattern);
        if !values.is_empty() {
            c.push(format!(".values({})", list(values)));
        }
        writeln!(out, "        r.constraint(|fc| {{").unwrap();
        chain(out, "            ", "fc", &c);
        writeln!(out, "        }});").unwrap();
    }
    writeln!(out, "    }});").unwrap();
}

fn pass(out: &mut String, p: &CompilerPassDescriptor) {
    let CompilerPassDescriptor {
        name,
        after,
        before,
        phase,
    } = p;
    let mut s = Vec::new();
    opt(&mut s, "after", after);
    opt(&mut s, "before", before);
    opt(&mut s, "phase", phase);
    if s.is_empty() {
        writeln!(out, "    c.pass({}, |_| {{}});", lit(name)).unwrap();
        return;
    }
    writeln!(out, "    c.pass({}, |p| {{", lit(name)).unwrap();
    chain(out, "        ", "p", &s);
    writeln!(out, "    }});").unwrap();
}

fn feature_flag(out: &mut String, f: &FeatureFlagDescriptor) {
    let FeatureFlagDescriptor {
        name,
        description,
        default_enabled,
    } = f;
    assert_ne!(
        description.as_deref(),
        Some(""),
        "{name}: empty description"
    );
    writeln!(
        out,
        "    c.feature_flag({}, {default_enabled}, {});",
        lit(name),
        lit(description.as_deref().unwrap_or(""))
    )
    .unwrap();
}

fn analyzer(out: &mut String, a: &AnalyzerDescriptor) {
    let AnalyzerDescriptor {
        language,
        file_extensions,
        excluded_dirs,
        scan_export,
        classify_export,
        map_export,
        description,
    } = a;
    let mut s = Vec::new();
    if !file_extensions.is_empty() {
        s.push(format!(".file_extensions({})", list(file_extensions)));
    }
    if !excluded_dirs.is_empty() {
        s.push(format!(".excluded_dirs({})", list(excluded_dirs)));
    }
    for (setter, value, default) in [
        ("scan_export", scan_export, format!("scan__{language}")),
        (
            "classify_export",
            classify_export,
            format!("classify__{language}"),
        ),
        ("map_export", map_export, format!("map__{language}")),
    ] {
        if *value != default {
            s.push(format!(".{setter}({})", lit(value)));
        }
    }
    opt(&mut s, "description", description);
    writeln!(out, "    c.analyzer({}, |a| {{", lit(language)).unwrap();
    chain(out, "        ", "a", &s);
    writeln!(out, "    }});").unwrap();
}

fn generate(name: &str, d: &ExtensionDeclaration, categories: &[&str]) -> String {
    let mut out = String::new();
    let what: Vec<&str> = categories
        .iter()
        .map(|c| match *c {
            "entities" => "kinds",
            "edges" => "edges",
            "shared_fields" => "shared fields",
            "enhancements" => "enhancements",
            "validation_rules" => "validation rules",
            "passes" => "passes",
            "feature_flags" => "feature flags",
            "analyzers" => "analyzers",
            other => other,
        })
        .collect();
    writeln!(
        out,
        "//! What {name} declares: its {}, with the SDK builders.\n\
         //! The host loads exactly this (`ContributionsBuilder::declaration`);\n\
         //! `crates/specforge-component/tests/declarations/` pins its wire form.\n",
        what.join(", ")
    )
    .unwrap();
    writeln!(out, "use specforge_extension_sdk::prelude::*;\n").unwrap();
    let mut sections: Vec<(&str, String)> = Vec::new();
    for category in categories {
        let mut body = String::new();
        let fun = match *category {
            "entities" => {
                d.entities.iter().for_each(|k| kind(&mut body, k));
                "kinds"
            }
            "edges" => {
                d.edges
                    .iter()
                    .for_each(|e| edge(&mut body, "    ", "c.edge", e));
                "edges"
            }
            "shared_fields" => {
                d.shared_fields
                    .iter()
                    .for_each(|f| field(&mut body, "    ", "c.shared_field", f));
                "shared_fields"
            }
            "enhancements" => {
                d.enhancements
                    .iter()
                    .for_each(|e| enhancement(&mut body, e));
                "enhancements"
            }
            "validation_rules" => {
                d.validation_rules.iter().for_each(|r| rule(&mut body, r));
                "rules"
            }
            "passes" => {
                d.passes.iter().for_each(|p| pass(&mut body, p));
                "passes"
            }
            "feature_flags" => {
                d.feature_flags
                    .iter()
                    .for_each(|f| feature_flag(&mut body, f));
                "feature_flags"
            }
            "analyzers" => {
                d.analyzers.iter().for_each(|a| analyzer(&mut body, a));
                "analyzers"
            }
            other => panic!("{other}"),
        };
        if !body.is_empty() {
            sections.push((fun, body));
        }
    }
    writeln!(
        out,
        "/// Declare everything this module holds on `c`.\npub(crate) fn declare(c: &mut ContributionsBuilder) {{"
    )
    .unwrap();
    for (fun, _) in &sections {
        writeln!(out, "    {fun}(c);").unwrap();
    }
    writeln!(out, "}}").unwrap();
    for (fun, body) in &sections {
        writeln!(out, "\nfn {fun}(c: &mut ContributionsBuilder) {{\n{body}}}").unwrap();
    }
    out
}

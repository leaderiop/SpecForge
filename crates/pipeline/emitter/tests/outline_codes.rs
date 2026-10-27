//! The outline prints a rule's code as a diagnostic does (ADR 0286).

use serde_json::Value;
use specforge_emitter::outline::{
    DependencyDepth, OutlineDetail, OutlineFormat, OutlineOptions, export,
};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_wasm::protocol::load_declaration;

fn rules(declaration: ExtensionDeclaration) -> Value {
    let outline = export(
        &[declaration],
        &OutlineOptions {
            format: OutlineFormat::Json,
            detail: OutlineDetail::All,
            deps: DependencyDepth::Full,
        },
    );
    let outline: Value = serde_json::from_str(&outline).unwrap();
    outline["extensions"][0]["validation_rules"].clone()
}

#[test]
fn a_third_party_rule_code_is_qualified_and_a_first_party_one_is_bare() {
    let runtime = specforge_component::ComponentRuntime::new();
    let names = vec!["@specforge/product".to_string()];
    specforge_component::builtins::load_builtins_for(&runtime, &names).unwrap();
    let product = load_declaration(&runtime, "@specforge/product")
        .unwrap()
        .declaration;
    let local = product.validation_rules[0].code.clone();
    let mut third_party = product.clone();
    third_party.handshake.name = "@acme/lint".into();

    assert_eq!(rules(third_party)[0]["code"], format!("acme.lint/{local}"));
    assert_eq!(rules(product)[0]["code"], local);
}

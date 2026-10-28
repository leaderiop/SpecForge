//! Judgements are on the wire (ADR 0290, protocol 1.9.0): a pass's input carries each entity's.

use serde_json::Value;
use specforge_protocol_types::PassInput;

#[specforge_test_macros::test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a pass input carries each entity's judgements as they were written"
)]
fn a_pass_input_reads_back_with_its_judgements() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wire/pass.judged.input.json");
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let input: PassInput = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(input.entities[0].judgements[0].rule, "ext/W901");
    assert_eq!(serde_json::to_value(&input).unwrap(), golden);
}

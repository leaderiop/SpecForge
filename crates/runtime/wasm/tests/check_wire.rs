//! Value checks are on the wire (ADR 0336): what the host sends `__check_values` in each phase and what it answers.

use serde_json::Value;
use specforge_protocol_types::{
    CHECK_VALUES_EXPORT, CheckPhase, CheckPlan, CheckValuesInput, Operation, OutcomeClass, Probe,
};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{CallFailure, ExtensionCalls};

const EXT: &str = "@calls/values";

fn wire(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn extension() -> specforge_extension_sdk::ContributionsBuilder {
    specforge_extension_sdk::ContributionsBuilder::new(specforge_extension_sdk::ExtensionMeta::new(
        EXT, "1.0.0",
    ))
}

fn answering() -> InProcessRuntime {
    InProcessRuntime::new().with_handler(extension, |export, input| {
        (export == CHECK_VALUES_EXPORT).then(|| {
            let input: CheckValuesInput = serde_json::from_slice(input).unwrap();
            let name = match input.phase {
                CheckPhase::Plan => "check_values.plan.output.json",
                CheckPhase::Judge => "check_values.judge.output.json",
            };
            Ok(wire(name).to_string().into_bytes())
        })
    })
}

fn called(phase: &'static str) -> (CheckPlan, CheckValuesInput) {
    let runtime = answering();
    let golden = wire(&format!("check_values.{phase}.input.json"));
    let input: CheckValuesInput = serde_json::from_value(golden.clone()).unwrap();
    let plan = ExtensionCalls::new(&runtime)
        .check_values(EXT, &input)
        .unwrap();
    assert_eq!(runtime.calls().last().unwrap().input, golden);
    (plan, input)
}

#[test]
fn a_check_values_call_is_typed_both_ways() {
    let (plan, _) = called("plan");
    assert_eq!(plan.items[0].findings[0].at, Some([0, 5]));
    assert_eq!(plan.items[0].probes.len(), 11);
    assert!(matches!(plan.items[0].probes[3], Probe::DnsResolve { .. }));
    assert_eq!(
        serde_json::to_value(&plan).unwrap(),
        wire("check_values.plan.output.json")
    );

    let (judged, input) = called("judge");
    assert_eq!(judged.items[0].findings[0].code, "E093");
    assert_eq!(input.items[0].outcomes[0].class, OutcomeClass::Status(404));
    assert!(input.items[0].outcomes[0].from_cache);
    assert_eq!(
        input.items[0].carry,
        Some(serde_json::json!({ "round": 2 }))
    );
    // A judge's probes ask for another round, carrying what it hands back.
    assert_eq!(judged.items[1].probes.len(), 1);
    assert_eq!(
        judged.items[1].carry,
        Some(serde_json::json!({ "round": 3 }))
    );
    assert_eq!(
        serde_json::to_value(&judged).unwrap(),
        wire("check_values.judge.output.json")
    );
}

#[test]
fn a_malformed_checker_answer_is_a_call_error_naming_the_type() {
    let runtime = InProcessRuntime::new().with_handler(extension, |export, _| {
        (export == CHECK_VALUES_EXPORT).then(|| Ok(br#"{"items":[{"id":"x"}]}"#.to_vec()))
    });
    let error = ExtensionCalls::new(&runtime)
        .check_values(EXT, &CheckValuesInput::default())
        .unwrap_err();
    assert_eq!(error.operation, Operation::CheckValues);
    assert!(
        matches!(error.failure, CallFailure::Malformed { .. }),
        "{error}"
    );
}

#[test]
fn a_probe_root_that_is_no_name_fails_the_call() {
    let plan_under = |root: &'static str| {
        serde_json::json!({ "items": [{ "id": 0, "probes": [
            { "probe": "fs.read", "root": root, "path": "a.md", "range": { "kind": "prefix", "len": 8 } }
        ] }] })
    };
    let asks = serde_json::from_value::<CheckPlan>;
    assert!(asks(plan_under("project")).is_ok());
    for hostile in ["", "..", "../etc", "Project", "a/b"] {
        assert!(asks(plan_under(hostile)).is_err(), "{hostile}");
    }

    // On the host that is a malformed answer, the extension's E028.
    let runtime = InProcessRuntime::new().with_handler(extension, |export, _| {
        (export == CHECK_VALUES_EXPORT).then(|| {
            Ok(br#"{"items":[{"id":0,"probes":[
                {"probe":"fs.read","root":"../etc","path":"passwd","range":{"kind":"prefix","len":8}}
            ]}]}"#
                .to_vec())
        })
    });
    let error = ExtensionCalls::new(&runtime)
        .check_values(EXT, &CheckValuesInput::default())
        .unwrap_err();
    assert!(
        matches!(error.failure, CallFailure::Malformed { .. }),
        "{error}"
    );
}

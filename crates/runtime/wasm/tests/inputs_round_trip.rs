//! Pins that an input, read into its protocol type and written back, is the same bytes, and that an input of 1.10
//! (without `files` or `now`, the fields 1.11.0 adds) is still read and written as it was: a field the protocol adds is
//! absent from the wire when it is empty, so a 1.10 guest reads what it read.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use specforge_protocol_types::{CollectInput, CommandInput, HookInput, PassInput};

fn golden(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// `name` read as a `T` and written back is `name`, but for the member `without` (a command's graph is a raw JSON
/// value the host splices in as rendered, so its golden is not its canonical form).
fn round_trips<T: DeserializeOwned + Serialize>(name: &str, without: Option<&str>) {
    let mut golden = golden(name);
    let typed: T = serde_json::from_value(golden.clone()).unwrap();
    let mut written = serde_json::to_value(&typed).unwrap();
    for value in [&mut golden, &mut written] {
        if let Some(member) = without {
            value.as_object_mut().unwrap().remove(member);
        }
    }
    assert_eq!(written, golden, "{name}");
}

#[test]
fn every_golden_encodes_byte_equal() {
    round_trips::<CommandInput>("command.input.json", Some("graph"));
    round_trips::<CommandInput>("command.writes.input.json", Some("graph"));
    round_trips::<CollectInput>("collect.input.json", None);
    round_trips::<HookInput>("hook.input.json", None);
    for pass in [
        "pass.input.json",
        "pass.check.input.json",
        "pass.guard.input.json",
        "pass.judged.input.json",
    ] {
        round_trips::<PassInput>(pass, None);
    }
}

/// `name` without its `now`, as a 1.10 host sent it.
fn without_now(name: &str) -> Value {
    let mut value = golden(name);
    assert!(
        value.as_object_mut().unwrap().remove("now").is_some(),
        "{name} carries now"
    );
    value
}

#[test]
fn a_1_10_input_without_now_is_read_and_written_without_it() {
    for (name, graph) in [
        ("command.input.json", true),
        ("command.writes.input.json", true),
        ("hook.input.json", false),
    ] {
        let mut old = without_now(name);
        let typed: Value = if name.starts_with("hook") {
            let input: HookInput = serde_json::from_value(old.clone()).unwrap();
            assert_eq!(input.now, "");
            serde_json::to_value(input).unwrap()
        } else {
            let input: CommandInput = serde_json::from_value(old.clone()).unwrap();
            assert_eq!(input.now, "");
            serde_json::to_value(input).unwrap()
        };
        let mut written = typed;
        if graph {
            old.as_object_mut().unwrap().remove("graph");
            written.as_object_mut().unwrap().remove("graph");
        }
        assert_eq!(written, old, "{name}");
    }
}

#[test]
fn a_collect_input_carries_now_only_when_it_has_one() {
    let mut input: CollectInput = serde_json::from_value(golden("collect.input.json")).unwrap();
    assert!(serde_json::to_value(&input).unwrap().get("now").is_none());
    input.now = Some("2026-10-28T09:30:00Z".into());
    assert_eq!(
        serde_json::to_value(&input).unwrap()["now"],
        "2026-10-28T09:30:00Z"
    );
}

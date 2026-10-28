//! Pins that a 1.10 input, read into its protocol type and written back, is the same bytes: every field the
//! protocol adds later (`files`, `now`) is absent from the wire when it is empty, so a 1.10 guest reads what it read.
//! Flips with nothing: these goldens stay as they are; the command, collect and hook inputs the host builds carry
//! `now` from 1.11.0 on (`tests/calls.rs` and the ops tests pin that), the goldens here are what a guest may be sent
//! by an older host.

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
fn every_1_10_golden_encodes_byte_equal_without_new_fields() {
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

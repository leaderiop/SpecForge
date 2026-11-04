//! A judge may ask for more probes after reading the first answers (ADR 0336 D4): at most [`MAX_PROBE_ROUNDS`] rounds per
//! value, each through the same gate, budget and cache, a value handing what it carries to its next round.

use std::sync::Arc;

use serde_json::json;
use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_project::values::CheckStatus;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{EXT, Scene, coded, described};

fn read(path: &str, range: ByteRange) -> Probe {
    Probe::FsRead {
        root: Root::Project,
        path: path.to_string(),
        range,
    }
}

/// The bytes the last answer read.
fn last_read(item: &CheckItem) -> String {
    match item.outcomes.last().map(|outcome| &outcome.data) {
        Some(OutcomeData::Bytes { bytes, .. }) => String::from_utf8_lossy(bytes).into_owned(),
        other => format!("{other:?}"),
    }
}

/// A file's last four bytes name where two bytes are to be read: a second round reads them.
fn pointed(item: &CheckItem) -> Judged {
    match &item.carry {
        None => match last_read(item).parse::<u64>() {
            Ok(offset) => Judged::more(
                vec![read(&item.text, ByteRange::Span { offset, len: 2 })],
                json!({ "from": offset }),
            ),
            Err(_) => vec![CheckFinding::new("E001", "no offset at the end")].into(),
        },
        Some(carry) => {
            let told = format!(
                "{} from {carry} after {}",
                last_read(item),
                item.outcomes.len()
            );
            vec![CheckFinding::new("E001", told)].into()
        }
    }
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Read])
            .why("to read"),
    );
    c.checker("pointed", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![read(&item.text, ByteRange::Suffix { len: 4 })])
            })
            .judge(pointed);
    });
    c.checker("forever", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![read(&item.text, ByteRange::Prefix { len: 1 })])
            })
            .judge(|item: &CheckItem| {
                let next = item.outcomes.len() as u64;
                Judged::more(
                    vec![read(
                        &item.text,
                        ByteRange::Span {
                            offset: next,
                            len: 1,
                        },
                    )],
                    json!(next),
                )
            });
    });
    // Ten reads, then ten more: twenty over every round, past the per-value cap of sixteen.
    let ten = |item: &CheckItem, from: u64| -> Vec<Probe> {
        (from..from + 10)
            .map(|offset| read(&item.text, ByteRange::Span { offset, len: 1 }))
            .collect()
    };
    c.checker("wide", move |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(move |item: &CheckItem| PlannedItem::probes(ten(item, 0)))
            .judge(move |item: &CheckItem| match item.carry {
                None => Judged::more(ten(item, 10), json!(2)),
                Some(_) => vec![CheckFinding::new("E001", "judged")].into(),
            });
    });
    c.kind("service", |k| {
        k.keyword("service");
        for field in ["pointed", "forever", "wide"] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(field);
            });
        }
    });
    c
}

fn scene(spec: &str) -> (Scene, Arc<InProcessRuntime>) {
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let scene = Scene::over(FakeHttp::new(), spec, &[EXT], Arc::clone(&runtime) as _);
    (scene, runtime)
}

fn check_calls(runtime: &InProcessRuntime) -> usize {
    runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count()
}

const POINTED: &str = "service a \"A\" {\n  pointed \"a.bin\"\n}\n";

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a second round reads the span the first named"
)]
fn a_second_round_reads_the_span_the_first_named() {
    let (scene, runtime) = scene(POINTED);
    scene.dir.write("a.bin", "abcdef0002");
    let found = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let told: Vec<&str> = coded(&found, "pin.probes/E001")
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    // The second round's judge is sent only that round's answer, beside what the first carried.
    assert_eq!(told, ["cd from {\"from\":2} after 1"], "{found:?}");
    // A plan, then a judgement per round.
    assert_eq!(check_calls(&runtime), 3);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a fourth round of probes is E028 naming the checker, and its value is not judged"
)]
fn a_fourth_round_is_e028() {
    let (scene, _) = scene("service a \"A\" {\n  forever \"a.bin\"\n}\n");
    scene.dir.write("a.bin", "abcdef");
    let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    let found = project.diagnostics();
    let failed = coded(&found, "E028");
    assert_eq!(failed.len(), 1, "{found:?}");
    assert!(
        failed[0].message.contains("fourth round"),
        "{}",
        failed[0].message
    );
    assert!(
        failed[0].message.contains("forever"),
        "{}",
        failed[0].message
    );
    assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");
    let rows = project.value_checks().of("a");
    assert_eq!(rows[0].status, CheckStatus::NotRun, "{rows:?}");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a cached check replays every round without calling the guest"
)]
fn a_cached_check_replays_every_round_without_the_guest() {
    let (scene, runtime) = scene(POINTED);
    scene.dir.write("a.bin", "abcdef0002");
    let first = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let calls = check_calls(&runtime);
    let again = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    assert_eq!(
        check_calls(&runtime),
        calls,
        "every round came from the cache"
    );
    assert_eq!(
        coded(&again, "pin.probes/E001"),
        coded(&first, "pin.probes/E001")
    );
    // What the second round read changed: its judgement is asked again.
    scene.dir.write("a.bin", "abXYef0002");
    let changed = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    assert_eq!(check_calls(&runtime), calls + 1);
    let told = coded(&changed, "pin.probes/E001");
    assert!(told[0].message.starts_with("XY"), "{told:?}");
}

#[test]
fn the_per_value_probe_cap_holds_over_every_round() {
    let (scene, _) = scene("service a \"A\" {\n  wide \"a.bin\"\n}\n");
    scene.dir.write("a.bin", "0123456789abcdefghijk");
    let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    let found = project.diagnostics();
    assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");
    let statuses: Vec<_> = project
        .value_checks()
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.status))
        .collect();
    assert_eq!(
        statuses,
        [specforge_project::values::CheckStatus::NotRun],
        "the second round's ten are past the cap with the first's"
    );
}

//! A spec root outside the project is an outside root (ADR 0336 D6): the specs compile, but a probe, a guest's lent `fs`
//! and a reader with no approval do not reach through it.

use std::sync::Arc;

use specforge_effects::testing::{FakeHttp, MemoryConsent, ports};
use specforge_effects::{
    Approval, Consent, EffectsMode, FsOp, FsRequest, OutcomeClass, RootApproval,
};
use specforge_extension_sdk::prelude::*;
use specforge_project::effects;
use specforge_project::values::CheckStatus;
use specforge_protocol_types::{Operation, Root};
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{EXT, Scene, coded, described};

/// A checker that reads a file under the spec root and tells what it read.
fn reader() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat, FsAccess::Read])
            .why("to read"),
    );
    c.checker("read", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsRead {
                    root: Root::Spec,
                    path: item.text.clone(),
                    bytes: 64,
                }])
            })
            .judge(|item: &CheckItem| {
                item.outcomes
                    .iter()
                    .map(|outcome| CheckFinding::new("E001", format!("{:?}", outcome.data)))
                    .collect()
            });
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("doc", |f| {
            f.field_type(FieldType::String).check("read");
        });
    });
    c
}

/// A project whose spec root is `outside`, holding the only source and a secret.
fn scene(outside: &std::path::Path) -> Scene {
    std::fs::write(outside.join("secret.md"), "the secret").unwrap();
    std::fs::write(
        outside.join("a.spec"),
        "service a \"A\" {\n  doc \"secret.md\"\n}\n",
    )
    .unwrap();
    let scene = Scene::with(
        FakeHttp::new(),
        "",
        &[EXT],
        InProcessRuntime::new().with(reader),
    );
    let mut config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scene.dir.root().join("specforge.json")).unwrap())
            .unwrap();
    config["spec_root"] = outside.display().to_string().into();
    scene.dir.write("specforge.json", config.to_string());
    scene
}

fn approval(scene: &Scene) -> Approval {
    let env = scene.env();
    let spec = env
        .effects_allowed()
        .roots
        .iter()
        .find(|root| root.name == "spec")
        .expect("the spec root is an outside root");
    assert!(!spec.inside);
    Approval::Root(RootApproval {
        project: specforge_effects::grant::project_key(scene.dir.root()),
        root: "spec".to_string(),
        target: spec.target.as_ref().unwrap().display().to_string(),
        approved_at: Default::default(),
    })
}

fn told_the_secret(found: &[specforge_common::Diagnostic]) -> bool {
    found.iter().any(|d| d.message.contains("bytes: ["))
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a spec root outside the project is an outside root a probe needs approval to read"
)]
fn a_probe_reads_an_outside_spec_root_only_once_approved() {
    let outside = tempfile::tempdir().unwrap();
    let scene = scene(outside.path());

    // Compiled as ever, and not read: nothing goes red for it.
    for mode in [EffectsMode::Cached, EffectsMode::Auto] {
        let project = scene.checked(mode, &MemoryConsent::default());
        let rows = project.value_checks().of("a");
        assert_eq!(rows[0].status, CheckStatus::NotRun, "{mode:?}");
        let found = project.diagnostics();
        assert!(!told_the_secret(&found), "{mode:?}: {found:?}");
        assert_eq!(coded(&found, "I206").len(), 1, "{mode:?}: {found:?}");
    }
    // `auto` asks for the root by name, with the command that approves it.
    let found = scene
        .checked(EffectsMode::Auto, &MemoryConsent::default())
        .diagnostics();
    let asked = coded(&found, "W170");
    assert_eq!(asked.len(), 1, "{found:?}");
    assert!(
        asked[0].message.contains("root 'spec'"),
        "{}",
        asked[0].message
    );

    let consent = MemoryConsent::default();
    consent.approve(approval(&scene)).unwrap();
    let approved = scene.checked(EffectsMode::Auto, &consent);
    let rows = approved.value_checks().of("a");
    assert_eq!(rows[0].outcome_class, Some(OutcomeClass::Ok));
    assert!(told_the_secret(&approved.diagnostics()));
}

#[test]
fn a_guests_lent_fs_is_denied_an_outside_spec_root_until_approved() {
    let outside = tempfile::tempdir().unwrap();
    let scene = scene(outside.path());
    let env = scene.env();
    let roots = env
        .effects_allowed()
        .with_named(specforge_effects::Roots::new(env.root(), env.spec_root()));
    let request = FsRequest {
        root: "spec".to_string(),
        path: "secret.md".to_string(),
        op: FsOp::Read { max_bytes: 64 },
    };
    let lent = |consent: &MemoryConsent| {
        let grant = effects::grants(&env, scene.dir.root(), consent)
            .into_iter()
            .find(|grant| grant.extension() == EXT)
            .expect("it declares a capability");
        Arc::new(grant)
            .fs(Operation::CheckValues, ports().fs)
            .run(&roots, &request)
            .class
    };
    assert_eq!(lent(&MemoryConsent::default()), OutcomeClass::Denied);
    let consent = MemoryConsent::default();
    consent.approve(approval(&scene)).unwrap();
    assert_ne!(lent(&consent), OutcomeClass::Denied);
}

#[test]
fn a_reader_with_no_approval_has_no_outside_spec_root() {
    let outside = tempfile::tempdir().unwrap();
    let scene = scene(outside.path());
    assert!(scene.env().read_roots().get(&Root::Spec).is_none());
    let inside = crate::value_probes::Scene::new(FakeHttp::new(), "");
    assert!(inside.env().read_roots().get(&Root::Spec).is_some());
    assert!(inside.env().effects_allowed().roots.is_empty());
}

/// A checker that lists under its value, and one that stats it.
fn lister() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat, FsAccess::Walk])
            .why("to read"),
    );
    let told = |item: &CheckItem| {
        item.outcomes
            .iter()
            .map(|outcome| CheckFinding::new("E001", format!("{:?}", outcome.data)))
            .collect()
    };
    c.checker("list", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsWalk {
                    root: Root::Spec,
                    path: item.text.clone(),
                    glob: "**/*.md".to_string(),
                }])
            })
            .judge(told);
    });
    c.checker("stat", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsStat {
                    root: Root::Spec,
                    path: item.text.clone(),
                }])
            })
            .judge(told);
    });
    c.kind("service", |k| {
        k.keyword("service");
        for field in ["list", "stat"] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(field);
            });
        }
    });
    c
}

#[test]
fn a_root_alone_is_the_root_to_a_probe() {
    let scene = Scene::with(
        FakeHttp::new(),
        "service one \"A\" {\n  list \"<project>\"\n  stat \"<project>\"\n}\n",
        &[EXT],
        InProcessRuntime::new().with(lister),
    );
    std::fs::create_dir_all(scene.dir.root().join("docs")).unwrap();
    std::fs::write(scene.dir.root().join("docs/a.md"), "a").unwrap();
    let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
    let found = project.diagnostics();
    let told = coded(&found, "pin.probes/E001");
    let saying = |text: &str| told.iter().any(|d| d.message.contains(text));
    assert!(saying("docs/a.md"), "the walk lists under it: {told:?}");
    assert!(saying("Dir"), "the stat finds a directory: {told:?}");
}

#[test]
fn a_value_the_host_warns_of_is_not_judged_and_counts_toward_i206() {
    let run = |value: &str, code: &str| {
        let scene = Scene::with(
            FakeHttp::new(),
            &format!("service one \"A\" {{\n  stat \"{value}\"\n}}\n"),
            &[EXT],
            InProcessRuntime::new().with(lister),
        );
        std::fs::write(scene.dir.root().join("README.md"), "a").unwrap();
        let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
        let found = project.diagnostics();
        assert_eq!(coded(&found, code).len(), 1, "{found:?}");
        assert_eq!(coded(&found, "I206").len(), 1, "{found:?}");
        assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");
        let row = &project.value_checks().of("one")[0];
        assert_eq!(row.status, CheckStatus::NotRun);
        assert_eq!(row.outcome_class, None, "a judged verdict over it is I207");
    };
    run("<project>/readme.md", "W174");
    run("<project>/docs\\\\a.md", "W175");
}

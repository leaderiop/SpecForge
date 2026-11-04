//! A checker's probes (ADR 0336): the host gates each on the checker's scopes, contains each path, performs through the
//! effect plane, and judges only what it performed. Every request is judged by the asking extension's own grant.

use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_effects::testing::{FakeGit, FakeHttp, MemoryConsent, ports};
use specforge_effects::{
    Approval, Asker, Consent, EffectRequest, EffectsMode, Http, HttpRequest, OutcomeClass, Ports,
    Served, UnixMillis, mode::Why,
};
use specforge_extension_sdk::prelude::*;
use specforge_project::effects;
use specforge_project::testing::TestProject;
use specforge_project::values::CheckStatus;
use specforge_project::{CompiledProject, Environment, SharedRuntime};
use specforge_protocol_types::Operation;
use specforge_registry::PlainDisk;
use specforge_wasm::testing::InProcessRuntime;

pub(crate) const EXT: &str = "@pin/probes";
pub(crate) const OTHER: &str = "@pin/other";
pub(crate) const A_URL: &str = "https://a.example/page";

/// Judged: anything but `ok` is an error naming the class.
pub(crate) fn judged_by_class(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .filter(|outcome| outcome.class != OutcomeClass::Ok)
        .map(|outcome| CheckFinding::new("E001", format!("{}: {}", item.text, outcome.class)))
        .collect()
}

pub(crate) fn head(item: &CheckItem) -> PlannedItem {
    PlannedItem::probes(vec![Probe::HttpHead {
        url: item.text.clone(),
        redirects: 5,
    }])
}

fn stat(item: &CheckItem) -> PlannedItem {
    PlannedItem::probes(vec![Probe::FsStat {
        root: "project".to_string(),
        path: item.text.clone(),
    }])
}

fn missing_with_near(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .filter(|outcome| outcome.class == OutcomeClass::Missing)
        .map(|outcome| {
            let finding = CheckFinding::new("E001", format!("{} is missing", item.text));
            match &outcome.data {
                OutcomeData::Stat {
                    near: Some(near), ..
                } => finding.with_help(format!("did you mean '{near}'?")),
                _ => finding,
            }
        })
        .collect()
}

/// A finding that tells what the probe read.
fn told(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .map(|outcome| CheckFinding::new("E001", format!("{:?}", outcome.data)))
        .collect()
}

pub(crate) fn described(c: &mut ContributionsBuilder) {
    c.code("E001", |d| {
        d.title("A bad value").explanation("A bad value.");
    });
}

/// `service`, whose fields name the checkers below.
fn extension_named(name: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.capability(
        Capability::http("only-a")
            .hosts(&["a.example"])
            .why("to ask a"),
    );
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
            .why("to read"),
    );
    c.capability(Capability::git("history").why("to look"));
    c.checker("link", |k| {
        k.applies_to(&["string", "string_list"])
            .scope("links")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.checker("near_a", |k| {
        k.applies_to(&["string"])
            .scope("only-a")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.checker("name", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::DnsResolve {
                    host: item.text.clone(),
                }])
            })
            .judge(judged_by_class);
    });
    c.checker("file", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(stat)
            .judge(missing_with_near);
    });
    c.checker("no_files", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(stat)
            .judge(missing_with_near);
    });
    c.checker("tracked", |k| {
        k.applies_to(&["string"])
            .scope("history")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::GitObject {
                    rev: item.text.clone(),
                }])
            })
            .judge(judged_by_class);
    });
    c.checker("head_of", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsReadPrefix {
                    root: "project".to_string(),
                    path: item.text.clone(),
                    bytes: 3,
                }])
            })
            .judge(told);
    });
    c.checker("under", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsWalk {
                    root: "project".to_string(),
                    path: item.text.clone(),
                    glob: "**/*.md".to_string(),
                }])
            })
            .judge(told);
    });
    c.checker("https_only", |k| {
        k.applies_to(&["string"])
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                let findings = (!item.text.starts_with("https:"))
                    .then(|| CheckFinding::new("E001", "not https"));
                PlannedItem::findings(findings.into_iter().collect())
            });
    });
    c.kind("service", |k| {
        k.keyword("service");
        for (field, checker) in [
            ("link", "link"),
            ("near_a", "near_a"),
            ("name", "name"),
            ("file", "file"),
            ("no_files", "no_files"),
            ("tracked", "tracked"),
            ("head_of", "head_of"),
            ("under", "under"),
        ] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(checker);
            });
        }
        k.field("links", |f| {
            f.field_type(FieldType::StringList).check("link");
        });
        k.field("strict_link", |f| {
            f.field_type(FieldType::String)
                .check("https_only")
                .check("link");
        });
    });
    c
}

fn extension() -> ContributionsBuilder {
    extension_named(EXT)
}

/// A second extension: a capability of its own, and a field of its own.
fn other() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OTHER, "1.0.0"));
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(head)
            .judge(judged_by_class);
    });
    c.kind("widget", |k| {
        k.keyword("widget");
        k.field("link", |f| {
            f.field_type(FieldType::String).check("link");
        });
    });
    c
}

/// One project, its extensions in-process, over a fake network.
pub(crate) struct Scene {
    pub(crate) dir: TestProject,
    pub(crate) runtime: SharedRuntime,
    pub(crate) http: Arc<FakeHttp>,
}

impl Scene {
    pub(crate) fn new(http: FakeHttp, spec: &str) -> Self {
        Self::with(http, spec, &[EXT], InProcessRuntime::new().with(extension))
    }

    pub(crate) fn with(
        http: FakeHttp,
        spec: &str,
        enabled: &[&str],
        runtime: InProcessRuntime,
    ) -> Self {
        let dir = TestProject::new()
            .config(|c| c["name"] = "p".into())
            .enabling(enabled)
            .installing_enabled()
            .file("a.spec", spec);
        Scene {
            dir,
            runtime: Arc::new(runtime),
            http: Arc::new(http),
        }
    }

    fn ports(&self) -> Ports {
        Ports {
            http: Arc::clone(&self.http) as Arc<dyn Http>,
            ..ports()
        }
    }

    fn env(&self) -> Environment {
        self.dir.environment_in(Arc::clone(&self.runtime))
    }

    /// The project checked in `mode`, approvals read from `consent`.
    pub(crate) fn checked(&self, mode: EffectsMode, consent: &dyn Consent) -> CompiledProject {
        let env = self.env();
        let root = self.dir.project_root();
        let effects = effects::of(mode, &self.ports(), &env, &root, consent);
        CompiledProject::compile_through(env.with_effects(effects), &PlainDisk)
    }

    pub(crate) fn online(&self) -> CompiledProject {
        self.checked(EffectsMode::Online, &MemoryConsent::default())
    }

    /// The approval of `extension`'s capability `scope`, as this project keys it.
    pub(crate) fn approval(&self, extension: &str, scope: &str) -> Approval {
        let env = self.env();
        let grants = effects::grants(&env, self.dir.root(), &MemoryConsent::default());
        let grant = grants
            .iter()
            .find(|grant| grant.extension() == extension)
            .expect("it declares capabilities");
        let granted = grant
            .capabilities()
            .iter()
            .find(|granted| granted.declared.name == scope)
            .expect("it declares the scope");
        Approval::Capability(grant.key(granted).expect("it grants something"))
    }
}

pub(crate) fn coded<'a>(project: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    project.iter().filter(|d| d.code == code).collect()
}

fn get(url: &str) -> EffectRequest {
    EffectRequest::Http(HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        ..HttpRequest::default()
    })
}

fn served(
    effects: &specforge_effects::Effects,
    extension: &str,
    op: Operation,
    request: &EffectRequest,
) -> Served {
    let scopes = ["links".to_string()];
    let asker = Asker {
        extension,
        op,
        scopes: &scopes,
    };
    effects.serve_for(&asker, "links", request, UnixMillis(1), |_| {
        specforge_effects::Answer {
            outcome: specforge_effects::Outcome {
                class: OutcomeClass::Ok,
                digest: String::new(),
            },
            payload: None,
        }
    })
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a probe outside its checker's scopes is E090 and not performed"
)]
fn a_probe_outside_the_checkers_scopes_is_e090_and_not_performed() {
    let spec = "service a \"A\" {\n  near_a \"https://b.example/\"\n  no_files \"a.spec\"\n}\n";
    let scene = Scene::new(FakeHttp::new().route("https://b.example/", 200, ""), spec);
    let project = scene.online();
    let found = project.diagnostics();
    let refused = coded(&found, "E090");
    assert_eq!(refused.len(), 2, "{found:?}");
    assert!(refused.iter().all(|d| d.span.is_some()), "on the value");
    assert!(scene.http.requests().is_empty(), "nothing was asked");
    let rows = project.value_checks().of("a");
    assert!(rows.iter().all(|row| row.status == CheckStatus::Failed));
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "two values naming one URL make one request"
)]
fn two_values_naming_one_url_make_one_request() {
    let spec = format!("service a \"A\" {{\n  links [\"{A_URL}\", \"{A_URL}\"]\n}}\n");
    let scene = Scene::new(FakeHttp::new().route(A_URL, 200, ""), &spec);
    let project = scene.online();
    assert_eq!(scene.http.requests().len(), 1);
    let rows = project.value_checks().of("a");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.status == CheckStatus::Passed));
    assert!(
        rows.iter()
            .all(|row| row.outcome_class == Some(OutcomeClass::Ok))
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a later check's probes are dropped after an earlier error"
)]
fn a_later_checks_probes_are_dropped_after_an_earlier_error() {
    let spec = "service a \"A\" {\n  strict_link \"http://a.example/\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    let project = scene.online();
    assert!(scene.http.requests().is_empty(), "never fetched");
    let found = project.diagnostics();
    assert_eq!(coded(&found, "pin.probes/E001").len(), 1, "{found:?}");
    let rows = project.value_checks().of("a");
    assert_eq!(rows[0].status, CheckStatus::Failed);
    assert_eq!(rows[1].status, CheckStatus::NotRun);
    assert!(
        coded(&found, "I206").is_empty(),
        "a dropped check is not one this machine failed to run"
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a value whose probe was not performed is not judged and counts toward I206"
)]
fn in_cached_mode_an_unrecorded_url_is_not_judged_and_counts_toward_i206() {
    let spec = format!("service a \"A\" {{\n  links [\"{A_URL}\", \"{A_URL}\"]\n}}\n");
    let scene = Scene::new(FakeHttp::new().route(A_URL, 404, ""), &spec);
    let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    let found = project.diagnostics();
    assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");
    let summary = coded(&found, "I206");
    assert_eq!(summary.len(), 1, "{found:?}");
    assert!(
        summary[0].message.contains("link 2"),
        "{}",
        summary[0].message
    );
    let rows = project.value_checks().of("a");
    assert!(rows.iter().all(|row| row.status == CheckStatus::NotRun));
    assert!(rows.iter().all(|row| row.outcome_class.is_none()));
    assert!(scene.http.requests().is_empty());
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "an answer read from the journal is the answer a live request gave"
)]
fn a_cached_run_reads_what_the_online_run_observed() {
    let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
    let scene = Scene::new(FakeHttp::new().route(A_URL, 404, ""), &spec);
    let live = scene.online();
    let cached = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    let finding = |project: &CompiledProject| {
        let found = project.diagnostics();
        coded(&found, "pin.probes/E001")
            .first()
            .map(|d| d.message.clone())
    };
    assert_eq!(finding(&live), finding(&cached));
    assert_eq!(finding(&live), Some(format!("{A_URL}: status:404")));
    let rows = cached.value_checks().of("a");
    assert!(rows[0].from_cache);
    assert_eq!(
        rows[0].observed_at,
        live.value_checks().of("a")[0].observed_at
    );
    assert_eq!(scene.http.requests().len(), 1);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "only the scopes these values use are asked for with W170"
)]
fn without_approval_only_the_used_scopes_are_asked_w170() {
    let spec = format!("service a \"A\" {{\n  link \"{A_URL}\"\n}}\n");
    let scene = Scene::new(FakeHttp::new().route(A_URL, 200, ""), &spec);
    let project = scene.checked(EffectsMode::Auto, &MemoryConsent::default());
    let found = project.diagnostics();
    let asked = coded(&found, "W170");
    assert_eq!(asked.len(), 1, "{found:?}");
    assert!(asked[0].message.contains("'links'"), "{}", asked[0].message);
    assert!(!asked[0].message.contains("only-a"), "{}", asked[0].message);
    assert_eq!(coded(&found, "I206").len(), 1, "{found:?}");
    assert!(scene.http.requests().is_empty());
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a dns probe runs through the Http port under its checker's scope"
)]
fn a_dns_probe_runs_through_the_http_port_under_its_checkers_scope() {
    let spec = "service a \"A\" {\n  name \"a.example\"\n}\n";
    let scene = Scene::new(FakeHttp::new().name("a.example", &["192.0.2.1"]), spec);
    let project = scene.online();
    assert_eq!(scene.http.asked().len(), 1);
    assert_eq!(
        project.value_checks().of("a")[0].status,
        CheckStatus::Passed
    );
    let absent = Scene::new(
        FakeHttp::new(),
        "service a \"A\" {\n  name \"gone.example\"\n}\n",
    );
    let found = absent.online().diagnostics();
    let reported = coded(&found, "pin.probes/E001");
    assert!(reported[0].message.contains("unreachable:dns"), "{found:?}");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a file value that leaves its root is E094 on the value"
)]
fn a_file_value_that_leaves_its_root_is_e094_on_the_value() {
    for written in ["../../etc/passwd", "/etc/passwd"] {
        let spec = format!("service a \"A\" {{\n  file \"{written}\"\n}}\n");
        let scene = Scene::new(FakeHttp::new(), &spec);
        let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
        let found = project.diagnostics();
        let refused = coded(&found, "E094");
        assert_eq!(refused.len(), 1, "{written}: {found:?}");
        assert_eq!(refused[0].span.as_ref().unwrap().start_line, 2);
        assert_eq!(refused[0].subject(), Some("a"));
        assert!(coded(&found, "pin.probes/E001").is_empty(), "never judged");
        assert_eq!(
            project.value_checks().of("a")[0].status,
            CheckStatus::Failed
        );
    }
    let spec = "service a \"A\" {\n  file \"<vault>/a.md\"\n}\n";
    let found = Scene::new(FakeHttp::new(), spec)
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let unknown = coded(&found, "E095");
    let span = unknown[0].span.as_ref().expect("on the value");
    let line = spec.lines().nth(1).unwrap();
    assert_eq!(&line[span.start_col - 1..span.end_col - 1], "vault");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a missing file suggests no name from outside its root"
)]
fn a_missing_file_suggests_no_name_from_outside_its_root() {
    let spec = "service a \"A\" {\n  file \"docs/guidee.md\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    scene.dir.write("docs/guide.md", "x");
    let found = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let missing = coded(&found, "pin.probes/E001");
    assert_eq!(
        missing[0].suggestion.as_deref(),
        Some("did you mean 'docs/guide.md'?"),
        "{found:?}"
    );
    // Through a link out, the host lists nothing: only what the value wrote can be named.
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("mission.md"), "x").unwrap();
        let spec = "service a \"A\" {\n  file \"out/misson.md\"\n}\n";
        let scene = Scene::new(FakeHttp::new(), spec);
        std::os::unix::fs::symlink(outside.path(), scene.dir.root().join("out")).unwrap();
        let found = scene
            .checked(EffectsMode::Cached, &MemoryConsent::default())
            .diagnostics();
        assert_eq!(coded(&found, "E094").len(), 1, "{found:?}");
        assert!(
            found
                .iter()
                .all(|d| !format!("{d:?}").contains("mission.md")),
            "{found:?}"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "one extension's approval approves no other extension's request"
)]
fn one_extensions_approval_approves_no_other_extensions_request() {
    let spec = format!(
        "service a \"A\" {{\n  link \"{A_URL}\"\n}}\nwidget b \"B\" {{\n  link \"{A_URL}\"\n}}\n"
    );
    let runtime = InProcessRuntime::new().with(extension).with(other);
    let scene = Scene::with(
        FakeHttp::new().route(A_URL, 200, ""),
        &spec,
        &[EXT, OTHER],
        runtime,
    );
    let consent = MemoryConsent::default();
    consent.approve(scene.approval(EXT, "links")).unwrap();

    let env = scene.env();
    let root = scene.dir.project_root();
    let auto = effects::of(EffectsMode::Auto, &scene.ports(), &env, &root, &consent);
    let request = get(A_URL);
    assert!(matches!(
        served(&auto, EXT, Operation::CheckValues, &request),
        Served::Outcome { .. }
    ));
    let fresh = effects::of(EffectsMode::Auto, &scene.ports(), &env, &root, &consent);
    assert_eq!(
        served(&fresh, OTHER, Operation::CheckValues, &request),
        Served::NotPerformed(Why::NotApproved),
        "and neither does the record the first one left"
    );

    // Whole: the approved extension's value is performed, the other's is not.
    let project = scene.checked(EffectsMode::Auto, &consent);
    assert_eq!(
        project.value_checks().of("a")[0].status,
        CheckStatus::Passed
    );
    assert_eq!(
        project.value_checks().of("b")[0].status,
        CheckStatus::NotRun
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a capability approved for another kind of call does not approve a check request"
)]
fn a_capability_approved_for_collect_does_not_approve_a_check_request() {
    fn kinds(name: &str, kinds: &[Operation]) -> ContributionsBuilder {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
        c.capability(
            Capability::http("links")
                .from_values()
                .call_kinds(kinds)
                .why("to ask"),
        );
        c
    }
    fn for_collect() -> ContributionsBuilder {
        kinds(EXT, &[Operation::Collect])
    }
    fn for_checks() -> ContributionsBuilder {
        kinds(EXT, &[Operation::CheckValues])
    }
    let request = get(A_URL);
    for (extension, op, performed) in [
        (
            for_collect as fn() -> ContributionsBuilder,
            Operation::CheckValues,
            false,
        ),
        (for_collect, Operation::Collect, true),
        (for_checks, Operation::CheckValues, true),
    ] {
        let runtime = InProcessRuntime::new().with(extension);
        let scene = Scene::with(FakeHttp::new(), "", &[EXT], runtime);
        let consent = MemoryConsent::default();
        consent.approve(scene.approval(EXT, "links")).unwrap();
        let env = scene.env();
        let root = scene.dir.project_root();
        let auto = effects::of(EffectsMode::Auto, &scene.ports(), &env, &root, &consent);
        let answer = served(&auto, EXT, op, &request);
        assert_eq!(
            matches!(answer, Served::Outcome { .. }),
            performed,
            "{op:?}: {answer:?}"
        );
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a redirect to a host outside the grant is refused on the checker's path"
)]
fn a_redirect_outside_the_grant_is_refused_on_the_checkers_path() {
    let there = "https://b.example/landed";
    let spec = format!("service a \"A\" {{\n  near_a \"{A_URL}\"\n}}\n");
    let http = FakeHttp::new().redirect(A_URL, there).route(there, 200, "");
    let scene = Scene::new(http, &spec);
    let project = scene.online();
    let found = project.diagnostics();
    assert_eq!(coded(&found, "E090").len(), 1, "{found:?}");
    assert!(coded(&found, "pin.probes/E001").is_empty(), "never judged");
    assert_eq!(
        project.value_checks().of("a")[0].status,
        CheckStatus::Failed
    );
}

#[specforge_test_macros::test(
    behavior = "classify_project_changes",
    verify = "a file a checked field names re-runs the checks"
)]
fn a_created_file_clears_its_missing_finding_on_the_next_update() {
    let spec = "service a \"A\" {\n  file \"docs/new.md\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    let mut session = scene.dir.session_in(Arc::clone(&scene.runtime));
    let found = session.project().diagnostics();
    assert_eq!(coded(&found, "pin.probes/E001").len(), 1, "{found:?}");
    scene.dir.write("docs/new.md", "x");
    specforge_project::testing::disk_changed(&mut session, &["docs/new.md"])
        .expect("the checks read that path");
    let found = session.project().diagnostics();
    assert!(coded(&found, "pin.probes/E001").is_empty(), "{found:?}");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a file probe reads a held prefix, and a walk stays under its root"
)]
fn a_file_probe_reads_a_held_prefix_and_a_walk_stays_under_its_root() {
    let spec = "service a \"A\" {\n  head_of \"docs/a.md\"\n  under \"docs\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    scene.dir.write("docs/a.md", "hello");
    scene.dir.write("docs/deep/b.md", "x");
    scene.dir.write("docs/c.txt", "x");
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("d.md"), "x").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), scene.dir.root().join("docs/out")).unwrap();
    let found = scene
        .checked(EffectsMode::Cached, &MemoryConsent::default())
        .diagnostics();
    let told: Vec<&str> = coded(&found, "pin.probes/E001")
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        told,
        [
            format!(
                "{:?}",
                OutcomeData::Bytes {
                    bytes: b"hel".to_vec(),
                    length: 5,
                    truncated: true,
                }
            ),
            format!(
                "{:?}",
                OutcomeData::Walk {
                    entries: vec!["a.md".to_string(), "deep/b.md".to_string()],
                    truncated: false,
                }
            ),
        ],
        "{found:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a git probe asks the Git port and needs no journal"
)]
fn a_git_probe_asks_the_git_port_in_cached_mode() {
    let spec = "service a \"A\" {\n  tracked \"nope\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    let env = scene.env().with_git(Arc::new(FakeGit::repo(
        scene.dir.root(),
        "abc123",
        Vec::new(),
    )));
    let root = scene.dir.project_root();
    let effects = effects::of(
        EffectsMode::Cached,
        &scene.ports(),
        &env,
        &root,
        &MemoryConsent::default(),
    );
    let project = CompiledProject::compile_through(env.with_effects(effects), &PlainDisk);
    let row = &project.value_checks().of("a")[0];
    assert_eq!(row.outcome_class, Some(OutcomeClass::Missing));
    assert_eq!(row.status, CheckStatus::Failed);
    let found = project.diagnostics();
    assert!(coded(&found, "I206").is_empty(), "{found:?}");
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a git probe this machine cannot answer is not judged"
)]
fn a_git_probe_needs_a_git_scope_and_a_repository() {
    let spec = "service a \"A\" {\n  tracked \"HEAD\"\n}\n";
    let scene = Scene::new(FakeHttp::new(), spec);
    let project = scene.checked(EffectsMode::Cached, &MemoryConsent::default());
    // No git port was given: this machine cannot answer it, and nothing goes red.
    let found = project.diagnostics();
    assert_eq!(coded(&found, "I206").len(), 1, "{found:?}");
    assert_eq!(
        project.value_checks().of("a")[0].status,
        CheckStatus::NotRun
    );
}

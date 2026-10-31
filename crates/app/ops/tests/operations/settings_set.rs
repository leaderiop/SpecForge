//! `settings set` and `unset` (ADR 0296): a value its setting's form admits is written into `specforge.json` as text,
//! the rest of the file as the author left it; a secret, an unknown setting and a value the form refuses are refused
//! before any write; the write goes through the lock and the journal.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::codes;
use specforge_extension_sdk::prelude::*;
use specforge_ops::extension::{SetOutcome, SetRequest, SettingChange, set_setting};
use specforge_ops::view::ProjectView;
use specforge_ops::{OpError, OpErrorKind, Wrote};
use specforge_package::Sha256;
use specforge_project::SharedRuntime;
use specforge_project::testing::{SETTINGS_EXTENSION, compile_at, settings_extension};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;
use tempfile::TempDir;

/// `@t/s` (the shared settings extension) and `@t/l`, which adds a list of strings.
fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with(settings_extension).with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/l", "1.0.0"));
        c.setting("paths", |s| {
            s.string_list();
        });
        c
    })
}

const CONFIG: &str = "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [\"@t/s\", \"@t/l\"],\n  \"kept\":   [1,   2],\n  \"settings\": {\n    \"@t/s\": {\n      \"profile\": \"compat\"\n    }\n  }\n}\n";

struct Project {
    dir: TempDir,
    runtime: SharedRuntime,
}

fn project(config: &str) -> Project {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config).unwrap();
    let runtime: SharedRuntime = Arc::new(runtime());
    specforge_installed::testing::install(dir.path(), &*runtime, &[SETTINGS_EXTENSION, "@t/l"]);
    std::fs::write(dir.path().join("main.spec"), "behavior a \"A\" {\n}\n").unwrap();
    Project { dir, runtime }
}

impl Project {
    fn config(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("specforge.json")).unwrap()
    }

    /// Whether a write left its journal or took the write lock: a refusal before the write does neither.
    fn touched_the_write_machinery(&self) -> bool {
        ["write-journal.json", "write.lock"]
            .iter()
            .any(|f| self.dir.path().join(".specforge").join(f).exists())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.config()).unwrap()
    }

    fn run(
        &self,
        extension: &str,
        name: &str,
        change: SettingChange,
        dry_run: bool,
    ) -> Result<SetOutcome, OpError> {
        let compiled = compile_at(self.dir.path(), Some(self.runtime.clone()));
        set_setting(
            &ProjectView::of(&compiled),
            &SetRequest {
                extension,
                name,
                change,
                dry_run,
                caller: specforge_ops::testing::caller(),
            },
        )
        .map_err(|refusal| refusal.error)
    }

    fn set(&self, name: &str, words: &[&str]) -> Result<SetOutcome, OpError> {
        let words = words.iter().map(ToString::to_string).collect();
        self.run(SETTINGS_EXTENSION, name, SettingChange::Words(words), false)
    }
}

#[specforge_test(
    behavior = "load_extension_settings",
    verify = "settings set writes a value its setting's form admits and leaves the rest of specforge.json as it was"
)]
fn set_writes_an_admitted_value() {
    let p = project(CONFIG);

    let done = p.set("level", &["2"]).unwrap();

    assert!(done.document.changed);
    assert_eq!(done.document.value, Some(json!(2)));
    assert_eq!(done.document.previous, None);
    assert!(matches!(done.wrote, Wrote::Ran(_)));
    let written = p.json();
    assert_eq!(written["settings"]["@t/s"]["level"], 2);
    assert_eq!(written["settings"]["@t/s"]["profile"], "compat");
    // The author's own spacing, which a re-render would normalize, is still there.
    assert!(
        p.config().contains("\"kept\":   [1,   2],"),
        "{}",
        p.config()
    );
    assert!(p.config().contains("\"name\": \"p\""));
}

#[specforge_test(
    behavior = "load_extension_settings",
    verify = "settings set refuses a secret, an unknown setting and a value the form refuses before it writes"
)]
fn set_refuses_before_writing() {
    let p = project(CONFIG);
    let refused = |error: OpError, code| {
        assert!(error.is(code), "{} {}", error.code, error.message);
        assert_eq!(p.config(), CONFIG, "nothing was written");
    };

    // An unknown setting, a value out of range, text for an integer, a word that is not an enum value.
    refused(p.set("nope", &["1"]).err().unwrap(), codes::E086);
    refused(p.set("level", &["9"]).err().unwrap(), codes::E086);
    refused(p.set("level", &["two"]).err().unwrap(), codes::E086);
    refused(p.set("mode", &["c"]).err().unwrap(), codes::E086);
    // A secret, declared or merely named like one: E087, and the value is not echoed.
    let secret = p.set("api_key", &["hunter2-do-not-echo"]).err().unwrap();
    assert!(!secret.message.contains("hunter2"), "{}", secret.message);
    refused(secret, codes::E087);
    refused(p.set("github_token", &["x"]).err().unwrap(), codes::E087);
    // A block is given as JSON, and `null` is an unset.
    assert_eq!(
        p.set("sections", &["x"]).err().unwrap().kind,
        OpErrorKind::InvalidInput
    );
    let null = p.run(
        SETTINGS_EXTENSION,
        "level",
        SettingChange::Json(Value::Null),
        false,
    );
    assert_eq!(null.err().unwrap().kind, OpErrorKind::InvalidInput);
    assert_eq!(p.config(), CONFIG);
    assert!(
        !p.touched_the_write_machinery(),
        "a refusal takes no lock and leaves no journal"
    );
}

#[test]
fn set_an_integer_that_is_text_is_e086() {
    let p = project(CONFIG);
    let error = p
        .run(
            SETTINGS_EXTENSION,
            "level",
            SettingChange::Json(json!("2")),
            false,
        )
        .err()
        .unwrap();
    assert!(error.is(codes::E086), "{}", error.message);
    assert_eq!(p.config(), CONFIG);
}

#[test]
fn set_a_list_takes_every_remaining_word() {
    let p = project(CONFIG);

    p.run(
        "@t/l",
        "paths",
        SettingChange::Words(vec!["a".into(), "b c".into()]),
        false,
    )
    .unwrap();

    assert_eq!(p.json()["settings"]["@t/l"]["paths"], json!(["a", "b c"]));
}

#[test]
fn set_a_block_needs_json() {
    let p = project(CONFIG);

    p.run(
        SETTINGS_EXTENSION,
        "sections",
        SettingChange::Json(json!([{"heading": "Goals"}])),
        false,
    )
    .unwrap();

    assert_eq!(
        p.json()["settings"]["@t/s"]["sections"],
        json!([{"heading": "Goals"}])
    );
    let bad = p.run(
        SETTINGS_EXTENSION,
        "sections",
        SettingChange::Json(json!([{"title": "Goals"}])),
        false,
    );
    assert!(bad.err().unwrap().is(codes::E086));
}

#[test]
fn set_a_family_key() {
    let p = project(CONFIG);

    p.set("models.planning", &["opus"]).unwrap();

    assert_eq!(p.json()["settings"]["@t/s"]["models.planning"], "opus");
    assert!(
        p.set("models.other", &["opus"])
            .err()
            .unwrap()
            .is(codes::E086)
    );
}

#[test]
fn set_into_a_missing_settings_object_creates_it() {
    let p = project(
        "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [\"@t/s\", \"@t/l\"]\n}\n",
    );

    p.set("level", &["3"]).unwrap();

    assert_eq!(p.json()["settings"]["@t/s"]["level"], 3);
    assert_eq!(p.json()["name"], "p");
}

#[test]
fn the_edit_keeps_the_files_layout() {
    // Tabs and a one-line array stay; only the new member is added.
    let config = "{\n\t\"name\": \"p\",\n\t\"version\": \"0.1.0\",\n\t\"extensions\": [\"@t/s\", \"@t/l\"],\n\t\"settings\": { \"@t/s\": { \"profile\": \"compat\" } }\n}\n";
    let p = project(config);

    p.set("level", &["2"]).unwrap();

    let after = p.config();
    assert!(
        after.contains("\t\"extensions\": [\"@t/s\", \"@t/l\"],"),
        "{after}"
    );
    assert!(after.starts_with("{\n\t\"name\": \"p\","), "{after}");
    assert_eq!(p.json()["settings"]["@t/s"]["level"], 2);
}

#[specforge_test(
    behavior = "load_extension_settings",
    verify = "settings unset removes the member and an interrupted set is rolled back"
)]
fn unset_restores_the_default() {
    let p = project(CONFIG);
    p.set("level", &["2"]).unwrap();

    let done = p
        .run(SETTINGS_EXTENSION, "level", SettingChange::Unset, false)
        .unwrap();

    assert!(done.document.changed);
    assert_eq!(done.document.value, None);
    assert_eq!(done.document.previous, Some(json!(2)));
    assert_eq!(p.config(), CONFIG, "back to the file as it was");
}

#[test]
fn unsetting_an_unwritten_setting_changes_nothing() {
    let p = project(CONFIG);

    let done = p
        .run(SETTINGS_EXTENSION, "level", SettingChange::Unset, false)
        .unwrap();

    assert!(!done.document.changed);
    assert_eq!(p.config(), CONFIG);
}

#[test]
fn an_extension_not_enabled_is_refused() {
    let p = project(CONFIG);

    let error = p
        .run(
            "@x/y",
            "anything",
            SettingChange::Words(vec!["1".into()]),
            false,
        )
        .err()
        .unwrap();

    assert_eq!(error.kind, OpErrorKind::ExtensionNotFound);
    assert!(error.message.contains("@x/y"), "{}", error.message);
    assert_eq!(p.config(), CONFIG);
}

#[test]
fn dry_run_writes_nothing() {
    let p = project(CONFIG);

    let done = p
        .run(
            SETTINGS_EXTENSION,
            "level",
            SettingChange::Words(vec!["2".into()]),
            true,
        )
        .unwrap();

    assert!(done.document.changed && done.document.dry_run);
    assert_eq!(done.document.value, Some(json!(2)));
    assert!(matches!(done.wrote, Wrote::Previewed));
    assert_eq!(p.config(), CONFIG);
}

#[test]
fn an_interrupted_set_is_rolled_back_by_the_journal() {
    let p = project(CONFIG);
    // A set was killed after it wrote `specforge.json`: the journal holds the file as it was.
    let half = CONFIG.replace("\"compat\"", "\"strict\"");
    std::fs::write(p.dir.path().join("specforge.json"), &half).unwrap();
    std::fs::create_dir_all(p.dir.path().join(".specforge")).unwrap();
    std::fs::write(
        p.dir.path().join(".specforge/write-journal.json"),
        json!({"format": 1, "doing": "cannot write specforge.json", "files": [
            {"path": "specforge.json", "before": CONFIG,
             "after": format!("sha256:{}", Sha256::of(half.as_bytes()))},
        ]})
        .to_string(),
    )
    .unwrap();

    p.set("level", &["2"]).unwrap();

    let written = p.json();
    assert_eq!(written["settings"]["@t/s"]["profile"], "compat", "put back");
    assert_eq!(written["settings"]["@t/s"]["level"], 2);
    assert!(!p.dir.path().join(".specforge/write-journal.json").exists());
}

const STALE: &str = "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [\"@t/s\", \"@t/l\"],\n  \"settings\": {\n    \"@t/s\": {\n      \"profile\": \"compat\",\n      \"level_typo\": 3,\n      \"api_key\": \"hunter2-committed\"\n    }\n  }\n}\n";

#[test]
fn unset_removes_a_stale_key_the_load_reports() {
    let p = project(STALE);

    let done = p
        .run(
            SETTINGS_EXTENSION,
            "level_typo",
            SettingChange::Unset,
            false,
        )
        .unwrap();

    assert!(done.document.changed);
    assert_eq!(done.document.previous, Some(json!(3)));
    assert_eq!(p.config(), STALE.replace("      \"level_typo\": 3,\n", ""));
    // A name neither declared nor written is still refused.
    let none = p
        .run(
            SETTINGS_EXTENSION,
            "never_written",
            SettingChange::Unset,
            false,
        )
        .err()
        .unwrap();
    assert!(none.is(codes::E086), "{}", none.message);
}

#[test]
fn unset_removes_a_committed_secret_and_never_repeats_it() {
    let p = project(STALE);

    let done = p
        .run(SETTINGS_EXTENSION, "api_key", SettingChange::Unset, false)
        .unwrap();

    assert!(done.document.changed);
    assert_eq!(done.document.previous, None, "the value is not echoed");
    let shown = serde_json::to_string(&done.document).unwrap();
    assert!(!shown.contains("hunter2"), "{shown}");
    assert!(!p.config().contains("hunter2"), "{}", p.config());
    assert_eq!(p.json()["settings"]["@t/s"]["profile"], "compat");
    // Setting a secret is still refused, and a secret-named key that is not written cannot be unset.
    assert!(p.set("api_key", &["x"]).err().unwrap().is(codes::E087));
    let unwritten = p
        .run(
            SETTINGS_EXTENSION,
            "github_token",
            SettingChange::Unset,
            false,
        )
        .err()
        .unwrap();
    assert!(unwritten.is(codes::E087), "{}", unwritten.message);
}

#[test]
fn the_edit_that_would_not_round_trip_writes_nothing() {
    // `@t/s` is a number: nothing can be spliced under it, and the load already says E086 for it.
    let config = CONFIG.replace("{\n      \"profile\": \"compat\"\n    }", "5");
    assert_ne!(config, CONFIG);
    let p = project(&config);

    let error = p.set("level", &["2"]).err().unwrap();

    assert_eq!(&*error.code, "config_edit_failed", "{}", error.message);
    assert_eq!(p.config(), config, "no half file");
    assert!(
        !p.dir.path().join(".specforge/write-journal.json").exists(),
        "no journal left"
    );
}

#[specforge_test(
    behavior = "load_extension_settings",
    verify = "a number setting takes a decimal within its bounds"
)]
fn settings_set_reads_a_decimal_word() {
    let dir = TempDir::new().unwrap();
    let config =
        "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [\"@t/n\"]\n}\n";
    std::fs::write(dir.path().join("specforge.json"), config).unwrap();
    let declare = || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/n", "1.0.0"));
        c.raw_category(
            specforge_extension_sdk::DeclaredCategory::Settings,
            json!([{"kind": "setting", "name": "x.ratio", "field_type": "number", "minimum": 0, "maximum": 100}]),
        );
        c
    };
    let runtime: SharedRuntime = Arc::new(InProcessRuntime::new().with(declare));
    specforge_installed::testing::install(dir.path(), &*runtime, &["@t/n"]);
    std::fs::write(dir.path().join("main.spec"), "behavior a \"A\" {\n}\n").unwrap();
    let compiled = compile_at(dir.path(), Some(runtime));
    set_setting(
        &ProjectView::of(&compiled),
        &SetRequest {
            extension: "@t/n",
            name: "x.ratio",
            change: SettingChange::Words(vec!["30.5".into()]),
            dry_run: false,
            caller: specforge_ops::testing::caller(),
        },
    )
    .map_err(|refusal| refusal.error)
    .unwrap();
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("specforge.json")).unwrap())
            .unwrap();
    assert_eq!(written["settings"]["@t/n"]["x.ratio"], json!(30.5));
}

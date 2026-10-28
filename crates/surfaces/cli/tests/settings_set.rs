//! K16 (ADR 0296): `specforge settings set` and `unset`. The command line over the real binary, which loads the
//! builtins and no extension that declares a setting: what it can show is the refusal before any write, and that the
//! listing form stays. The success path (the words, `--value-json`, `--dry-run`, the printed lines, `--format json`,
//! the exit codes) is pinned in process over an extension that declares settings in `src/settings_tests.rs`, and the
//! writes themselves in `specforge-ops`.
use crate::support::{Cli, TestProject};

const CONFIG: &str = "{\n  \"name\": \"p\",\n  \"version\": \"0.1.0\",\n  \"extensions\": [],\n  \"settings\": {}\n}\n";

#[test]
fn settings_set_refuses_an_extension_that_is_not_enabled_and_writes_nothing() {
    let project = TestProject::new().raw_config(CONFIG).file("a.spec", "");
    let cli = Cli::on(&project);

    let run = cli.run(&["settings", "set", "@x/y", "workflow.tdd_mode", "strict"]);

    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert!(run.stderr.contains("@x/y"), "{}", run.stderr);
    assert_eq!(
        project.read("specforge.json"),
        CONFIG,
        "the file is untouched"
    );
}

#[test]
fn specforge_settings_still_lists() {
    let project = TestProject::new().raw_config(CONFIG).file("a.spec", "");
    let cli = Cli::on(&project);

    let listing = cli.document(&["settings", "--all"]);
    assert_eq!(listing.code, 0, "{}", listing.stderr);
    assert_eq!(listing.json["count"], 0);
    let human = cli.run(&["settings"]);
    assert_eq!(human.code, 0);
    assert!(human.stdout.contains("No settings."), "{}", human.stdout);
}

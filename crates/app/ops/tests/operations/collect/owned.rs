//! The files a run writes at names it owns (ADR 0171): the report, the captured output and the config file. A link
//! committed at one of those names, pointing at another project file, is replaced: never written through.
#![cfg(unix)]

use super::{Fixture, Machine, Runner};
use specforge_ops::analysis::collect::{Mode, RunnerOutput};
use specforge_ops::asking::Asking;
use specforge_ops::testing::ScriptedRun;
use specforge_test_macros::test as specforge_test;

const CONFIG: &str = ".specforge/reports/nextest.nextest.toml";
const CAPTURE: &str = "out/nextest.json.stdout.txt";
const REPORT: &str = "out/nextest.json";
const MANIFEST: &str = "[package]\nname = \"toy\"\n";
const KEPT: &str = "kept\n";

/// Runner `nextest`, capturing its output, whose command reads its config file `nextest.toml`; a `Cargo.toml` and a
/// `Kept.txt` in the project, and a link at each name the run owns: the config at `Cargo.toml`, the capture at
/// `Kept.txt`, the report at `src/planted.json`, which is not there.
fn linked() -> Fixture {
    let fx = Fixture::new(vec![
        Runner::new("nextest")
            .run(&["cargo-nextest", "--tool-config-file", "specforge:{config}"])
            .configuring(
                "nextest.toml",
                "[profile.specforge.junit]\npath = \"{report}\"\n",
            )
            .capturing_stdout()
            .proving("a", "t"),
    ]);
    fx.write("Cargo.toml", MANIFEST);
    fx.write("Kept.txt", KEPT);
    std::fs::create_dir_all(fx.root().join(".specforge/reports")).unwrap();
    std::fs::create_dir_all(fx.root().join("out")).unwrap();
    std::fs::create_dir_all(fx.root().join("src")).unwrap();
    for (name, target) in [
        (CONFIG, "../../Cargo.toml"),
        (CAPTURE, "../Kept.txt"),
        (REPORT, "../src/planted.json"),
    ] {
        std::os::unix::fs::symlink(target, fx.root().join(name)).unwrap();
    }
    fx
}

fn is_link(fx: &Fixture, name: &str) -> bool {
    std::fs::symlink_metadata(fx.root().join(name))
        .unwrap()
        .file_type()
        .is_symlink()
}

#[specforge_test(
    behavior = "run_collector_command",
    verify = "a link at the report, capture or config name is replaced, never written through"
)]
fn a_link_at_a_name_the_run_owns_is_replaced_never_written_through() {
    let fx = linked();
    let machine = Machine::new()
        .approving_everything()
        .answering(ScriptedRun {
            files: vec![("".into(), "{}".to_string())],
            stdout: Some("t ... ok\n".to_string()),
            exit_code: Some(0),
            ..ScriptedRun::default()
        });

    fx.collect_with(
        Some("nextest"),
        Mode::Run(RunnerOutput::Discard),
        &specforge_ops::Containment::within(fx.root()),
        Asking::Nobody,
        &machine,
    )
    .unwrap();

    assert_eq!(fx.read("Cargo.toml"), MANIFEST);
    assert_eq!(fx.read("Kept.txt"), KEPT);
    assert!(!fx.exists("src/planted.json"));
    for name in [CONFIG, CAPTURE, REPORT] {
        assert!(!is_link(&fx, name), "{name}");
    }
    assert!(fx.read(CONFIG).starts_with("[profile.specforge.junit]"));
    assert_eq!(fx.read(CAPTURE), "t ... ok\n");
    assert_eq!(fx.read(REPORT), "{}");
}

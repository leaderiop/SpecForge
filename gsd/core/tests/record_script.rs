//! `gsd/parity/record.sh` records a golden only from a `gsd-tools` run that succeeded.

use std::path::Path;
use std::process::Command;

fn goldens(dir: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(dir)
        .expect("goldens")
        .map(|e| {
            let path = e.expect("entry").path();
            (
                path.file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read_to_string(&path).expect("read"),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_failing_gsd_tools_that_still_prints_json_records_nothing() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is not installed: record.sh cannot run, nothing to check");
        return;
    }
    let fake = tempfile::tempdir().expect("a temp dir");
    std::fs::write(fake.path().join("package.json"), r#"{"version":"1.18.0"}"#).expect("write");
    let bin = fake.path().join("gsd-core/bin");
    std::fs::create_dir_all(&bin).expect("dirs");
    std::fs::write(
        bin.join("gsd-tools.cjs"),
        "process.stdout.write('{\"bogus\": true}\\n'); process.exit(1);\n",
    )
    .expect("write");

    let parity = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity");
    let before = goldens(&parity.join("goldens/taskboard"));
    let run = Command::new("sh")
        .arg(parity.join("record.sh"))
        .env("GSD_CORE_DIR", fake.path())
        .output()
        .expect("run record.sh");
    assert!(
        !run.status.success(),
        "a failing gsd-tools fails the script"
    );
    assert_eq!(
        goldens(&parity.join("goldens/taskboard")),
        before,
        "no golden changed"
    );
}

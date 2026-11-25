//! `gsd/parity/record.sh`, `record-security.sh` and `record-ledger.sh` record a golden only from a `gsd-tools` run
//! that succeeded: each runs a fake `gsd-tools` that prints JSON and fails, and must fail there, writing nothing.
//! `record-reports.sh` needs Docker, so it never runs here: its isolation is read from its text.

use std::path::Path;
use std::process::Command;

use regex::Regex;

#[path = "common/node.rs"]
mod node;

/// Every file under `dir`, at any depth (the recorded tool reports sit in `reports/`), with its text, by path.
fn goldens(dir: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("goldens") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            found.extend(
                goldens(&path)
                    .into_iter()
                    .map(|(file, text)| (format!("{name}/{file}"), text)),
            );
        } else {
            found.push((name, std::fs::read_to_string(&path).expect("read")));
        }
    }
    found.sort();
    found
}

fn check(script: &str, goldens_dir: &str) {
    if !node::node_test_runs() {
        eprintln!(
            "node --test with the JUnit reporter does not run here: {script} cannot run, nothing to check"
        );
        return;
    }
    let fake = tempfile::tempdir().expect("a temp dir");
    std::fs::write(fake.path().join("package.json"), r#"{"version":"1.19.0"}"#).expect("write");
    let bin = fake.path().join("gsd-core/bin");
    std::fs::create_dir_all(&bin).expect("dirs");
    std::fs::write(
        bin.join("gsd-tools.cjs"),
        "require('fs').writeFileSync(__dirname + '/ran', ''); process.stdout.write('{\"bogus\": true}\\n'); process.exit(1);\n",
    )
    .expect("write");

    let parity = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity");
    let before = goldens(&parity.join(goldens_dir));
    let run = Command::new("bash")
        .arg(parity.join(script))
        .env("GSD_CORE_DIR", fake.path())
        .output()
        .expect("run the script");
    assert!(
        !run.status.success(),
        "a failing gsd-tools fails the script"
    );
    assert!(
        bin.join("ran").is_file(),
        "{script} failed before it ran gsd-tools: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        goldens(&parity.join(goldens_dir)),
        before,
        "no golden changed"
    );
}

#[test]
fn a_failing_gsd_tools_that_still_prints_json_records_nothing() {
    check("record.sh", "goldens/taskboard");
}

#[test]
fn record_security_records_nothing_from_a_failing_gsd_tools_either() {
    check("record-security.sh", "goldens/vault");
}

#[test]
fn record_ledger_records_nothing_from_a_failing_gsd_tools_either() {
    // The verbs run before the tests, so the script stops at the fake gsd-tools, before any cargo-nextest run.
    check("record-ledger.sh", "goldens/ledger");
}

/// `record-reports.sh`'s text with its comment lines dropped and each `\` continuation joined to its line.
fn record_reports() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/record-reports.sh");
    let text = std::fs::read_to_string(path).expect("read record-reports.sh");
    let code: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    code.join("\n").replace("\\\n", " ")
}

#[test]
fn record_reports_pins_every_image_by_digest() {
    let script = record_reports();
    let assigned = Regex::new(r"(?m)^([A-Z_]+_IMAGE)=(.*)$").expect("regex");
    let pinned = Regex::new(r"^[^@\s]+@sha256:[0-9a-f]{64}$").expect("regex");
    let images: Vec<&str> = assigned
        .captures_iter(&script)
        .map(|image| {
            let (name, value) = (image.get(1).expect("name"), image.get(2).expect("value"));
            assert!(
                pinned.is_match(value.as_str()),
                "{} is not pinned by digest: {}",
                name.as_str(),
                value.as_str()
            );
            name.as_str()
        })
        .collect();
    assert_eq!(images.len(), 3, "the node, rust and osv-scanner images");
    // Every container runs one of them: `container`'s first argument is the image `docker run` gets.
    let calls = Regex::new(r#"(?m)^\s*container "\$([A-Za-z_]+)""#).expect("regex");
    let used: Vec<&str> = calls
        .captures_iter(&script)
        .map(|call| call.get(1).expect("name").as_str())
        .collect();
    assert_eq!(
        used.len(),
        3,
        "the javascript, ledger and osv-scanner containers"
    );
    for image in used {
        assert!(
            images.contains(&image),
            "container runs ${image}, no pinned image"
        );
    }
}

#[test]
fn record_reports_runs_one_container_with_no_mount_privilege_or_host_network() {
    let script = record_reports();
    assert_eq!(script.matches("docker").count(), 1, "one docker call");
    let run = script
        .lines()
        .find(|line| line.contains("docker run"))
        .expect("a docker run");
    // Docker's options are the tokens before the image; what follows is the container's command.
    let tokens: Vec<&str> = run.split_whitespace().collect();
    let docker = tokens.iter().position(|t| *t == "docker").expect("docker");
    let image = tokens
        .iter()
        .position(|t| *t == "\"$image\"")
        .expect("docker run takes container's \"$image\"");
    let options = &tokens[docker + 2..image];
    for (at, option) in options.iter().enumerate() {
        let short = option.starts_with('-') && !option.starts_with("--");
        assert!(!(short && option.contains('v')), "a volume: {option}");
        for refused in [
            "--volume",
            "--mount",
            "--privileged",
            "--cap-add",
            "--volumes-from",
        ] {
            assert!(
                *option != refused && !option.starts_with(&format!("{refused}=")),
                "docker run takes {option}"
            );
        }
        let host_network = matches!(*option, "--network=host" | "--net=host")
            || (matches!(*option, "--network" | "--net") && options.get(at + 1) == Some(&"host"));
        assert!(!host_network, "docker run joins the host network");
    }
}

#[test]
fn record_reports_refuses_any_fixture_but_the_three_before_running_one() {
    let script = record_reports();
    assert!(script.contains("\n[ ${#fixtures[@]} -gt 0 ] || fixtures=(taskboard vault ledger)\n"));
    let allowlist = "for fixture in \"${fixtures[@]}\"; do
  case $fixture in
    taskboard | vault | ledger) ;;
    *) echo \"record-reports.sh: no fixture '$fixture' (taskboard, vault, ledger)\" >&2; exit 1 ;;
  esac
";
    let checked = script.find(allowlist).expect("the fixture allowlist");
    let ran = script
        .find("\n  \"$fixture\"\n")
        .expect("each fixture's run");
    assert!(
        checked < ran,
        "the allowlist is checked before any fixture runs"
    );
}

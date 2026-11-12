//! A phase spec's probe rows, `edge` and `spec_prohibition` (ADR 0333), and gsd/E523, over the discuss project.

use super::*;

/// A phase spec's edge and prohibition rows are written whole by one `specforge new` each (ADR 0333).
#[test]
fn an_edge_and_a_spec_prohibition_are_written_whole() {
    let p = project("discuss");
    let edge = p.run(&[
        "new",
        "edge",
        "EC1",
        "LR1 boundary",
        "--in",
        "spec/discuss.spec",
        "--spec",
        "v1.0/02/02-SPEC",
        "--requirement",
        "LR1",
        "--category",
        "boundary",
        "--status",
        "unresolved",
    ]);
    assert_eq!(edge.code, 0, "{}", edge.all());
    let prohibition = p.run(&[
        "new",
        "spec_prohibition",
        "SP1",
        "No plaintext",
        "--in",
        "spec/discuss.spec",
        "--spec",
        "v1.0/02/02-SPEC",
        "--requirement",
        "LR1",
        "--statement",
        "MUST NOT store tasks in plaintext",
        "--status",
        "dismissed",
        "--reason",
        "no secrets in a task",
    ]);
    assert_eq!(prohibition.code, 0, "{}", prohibition.all());
    let edges = edges(&p, "v1.0/02/02-SPEC/EC1");
    assert!(
        has_edge(
            &edges,
            "v1.0/02/02-SPEC/EC1",
            "v1.0/02/02-SPEC/LR1",
            "requirement"
        ),
        "{edges:?}"
    );
    checks_clean(&p);
}

/// A typed edit that leaves an edge resolved without its tier is refused whole: gsd/E523 is an error.
#[test]
fn an_edge_resolved_without_a_tier_is_e523() {
    let p = project("discuss");
    let edge = p.run(&[
        "new",
        "edge",
        "EC1",
        "LR1 boundary",
        "--in",
        "spec/discuss.spec",
        "--spec",
        "v1.0/02/02-SPEC",
        "--requirement",
        "LR1",
        "--category",
        "boundary",
        "--status",
        "resolved",
        "--resolution",
        "x",
    ]);
    assert_ne!(edge.code, 0, "{}", edge.all());
    assert!(
        edge.all().contains("E080") && edge.all().contains("gsd/E523"),
        "{}",
        edge.all()
    );
    assert!(
        edge.all().contains("resolved requires a verification tier"),
        "{}",
        edge.all()
    );
    assert_eq!(fixture("discuss.spec"), p.read("spec/discuss.spec"));
}

/// A resolved edge with its tier and resolution, and a dismissed prohibition with its reason, check clean.
#[test]
fn a_resolved_edge_and_a_dismissed_prohibition_are_not_e523() {
    let p = project("discuss");
    for args in [
        &[
            "new",
            "edge",
            "EC1",
            "LR1 boundary",
            "--in",
            "spec/discuss.spec",
            "--spec",
            "v1.0/02/02-SPEC",
            "--requirement",
            "LR1",
            "--category",
            "boundary",
            "--status",
            "resolved",
            "--verification",
            "explicit",
            "--resolution",
            "AC#2 pins the restart",
        ][..],
        &[
            "new",
            "spec_prohibition",
            "SP1",
            "No plaintext",
            "--in",
            "spec/discuss.spec",
            "--spec",
            "v1.0/02/02-SPEC",
            "--requirement",
            "LR1",
            "--statement",
            "MUST NOT store tasks in plaintext",
            "--status",
            "dismissed",
            "--reason",
            "no secrets in a task",
        ][..],
    ] {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    checks_clean(&p);
}

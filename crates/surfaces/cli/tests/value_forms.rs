//! A value is written in its field's one form (ADR 0147): `check` warns W159 and `format` rewrites it.

use crate::support::{Cli, TestProject};

fn project(file: &str, source: &str) -> TestProject {
    TestProject::new()
        .enabling(&["@specforge/software"])
        .config(|c| c["spec_root"] = "spec".into())
        .file(file, source)
}

const CART: &str = "behavior add_item \"Add\" {\n  category \"cart\"\n  contract \"x\"\n}\n";

#[specforge_test_macros::test(
    behavior = "check_field_value_types",
    verify = "a quoted word on a word field and a bare word on a text field are W159"
)]
fn check_reports_a_tag_written_as_text() {
    let dir = project("spec/cart.spec", CART);

    let diagnostics = Cli::on(&dir).document(&["check"]).json;

    let w159: Vec<_> = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "W159")
        .collect();
    assert_eq!(w159.len(), 1, "{diagnostics}");
    assert_eq!(w159[0]["span"]["start_line"], 2, "{diagnostics}");
    let message = w159[0]["message"].as_str().unwrap();
    assert!(
        message.contains("category") && message.contains("cart"),
        "{message}"
    );
}

#[specforge_test_macros::test(
    behavior = "format_spec_files",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn format_writes_a_tag_bare() {
    let dir = project("spec/cart.spec", CART);
    let cli = Cli::on(&dir);
    let before = cli.run(&["export", "--format", "json"]).exits(0).stdout;

    cli.run(&["format", "--check"]).exits(1);
    cli.run(&["format"]).exits(0);

    let written = std::fs::read_to_string(dir.root().join("spec/cart.spec")).unwrap();
    assert!(written.contains("\n  category cart\n"), "{written}");
    let after = cli.run(&["export", "--format", "json"]).exits(0).stdout;
    assert_eq!(before, after, "formatting the quotes keeps the graph");
}

#[specforge_test_macros::test(
    behavior = "format_spec_files",
    verify = "a value is written in its field's form, read from the project's declarations"
)]
fn format_quotes_text_written_bare() {
    let dir = project(
        "spec/events.spec",
        "event e1 \"E\" {\n  channel compilation\n}\n",
    );

    Cli::on(&dir).run(&["format"]).exits(0);

    let written = std::fs::read_to_string(dir.root().join("spec/events.spec")).unwrap();
    assert!(
        written.contains("\n  channel \"compilation\"\n"),
        "{written}"
    );
}

const DEFAULTS: &str = "defaults behavior {\n  category \"cart\"\n}\n\n\
                        behavior add_item \"Add\" {\n  contract \"x\"\n}\n\n\
                        behavior remove_item \"Remove\" {\n  contract \"y\"\n}\n";

#[specforge_test_macros::test(
    behavior = "apply_format_rules",
    verify = "a defaults block's values are written in the forms of the kind it names"
)]
fn format_writes_a_defaults_blocks_values_in_the_forms_of_the_kind_it_names() {
    let dir = project("spec/cart.spec", DEFAULTS);
    let cli = Cli::on(&dir);
    let before = cli.run(&["export", "--format", "json"]).exits(0).stdout;

    cli.run(&["format", "--check"]).exits(1);
    cli.run(&["format"]).exits(0);

    let written = std::fs::read_to_string(dir.root().join("spec/cart.spec")).unwrap();
    assert!(
        written.starts_with("defaults behavior {\n  category cart\n}\n"),
        "{written}"
    );
    let after = cli.run(&["export", "--format", "json"]).exits(0).stdout;
    assert_eq!(
        before, after,
        "formatting a default's quotes keeps the graph"
    );
}

#[specforge_test_macros::test(
    behavior = "check_field_value_types",
    verify = "a value a defaults block writes in the other form is W159 once, at the default"
)]
fn check_reports_a_defaults_value_in_the_other_form_once_at_the_default() {
    let dir = project("spec/cart.spec", DEFAULTS);
    let diagnostics = Cli::on(&dir).document(&["check"]).json;
    let w159: Vec<_> = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "W159")
        .collect();
    assert_eq!(w159.len(), 1, "{diagnostics}");
    assert_eq!(w159[0]["span"]["start_line"], 2, "{diagnostics}");
    let message = w159[0]["message"].as_str().unwrap();
    assert!(
        message.contains("of defaults behavior") && !message.contains("add_item"),
        "{message}"
    );
}

fn governed(file: &str, source: &str) -> TestProject {
    TestProject::new()
        .enabling(&["@specforge/governance"])
        .config(|c| c["spec_root"] = "spec".into())
        .file(file, source)
}

#[specforge_test_macros::test(
    behavior = "format_spec_files",
    verify = "check, format and set agree on a quoted number, a quoted date and a whole value on a list field"
)]
fn check_format_and_set_agree_on_a_quoted_number_a_quoted_date_and_a_whole_value_on_a_list() {
    let dir = governed(
        "spec/g.spec",
        "failure_mode fm1 \"FM1\" {\n  severity high\n  cause    \"c\"\n  effect   \"e\"\n  \
         rpn      \"100\"\n  refs     \"https://x\"\n}\n\n\
         decision d1 \"D1\" {\n  status   accepted\n  date     \"2026-03-01\"\n  \
         decision \"x\"\n  context  \"y\"\n}\n",
    );
    let cli = Cli::on(&dir);
    let w159 = cli
        .document(&["check"])
        .json
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "W159")
        .count();
    assert_eq!(w159, 3);
    let before = cli.run(&["export", "--format", "json"]).exits(0).stdout;

    cli.run(&["format", "--check"]).exits(1);
    cli.run(&["format"]).exits(0);

    let written = std::fs::read_to_string(dir.root().join("spec/g.spec")).unwrap();
    for line in [
        "\n  rpn      100\n",
        "\n  refs     [\"https://x\"]\n",
        "\n  date     2026-03-01\n",
    ] {
        assert!(written.contains(line), "{line:?} in {written}");
    }
    let after = cli.run(&["export", "--format", "json"]).exits(0).stdout;
    assert_eq!(before, after);
    for args in [
        &["set", "fm1", "rpn", "100", "--dry-run"][..],
        &["set", "fm1", "refs", "https://x", "--dry-run"][..],
        &["set", "d1", "date", "2026-03-01", "--dry-run"][..],
    ] {
        let run = cli.run(args).exits(0);
        assert!(
            run.stdout.contains("nothing to write"),
            "{args:?}: {}",
            run.stdout
        );
    }
}

#[specforge_test_macros::test(
    behavior = "check_field_value_types",
    verify = "a decision's, a milestone's and a release's date is one calendar day; any other value is E061"
)]
fn a_date_field_holds_one_calendar_day_written_bare() {
    let dir = TestProject::new()
        .enabling(&["@specforge/product", "@specforge/governance"])
        .config(|c| c["spec_root"] = "spec".into())
        .file(
            "spec/d.spec",
            "decision d1 \"D1\" {\n  status accepted\n  date 2026-03-01\n  decision \"x\"\n  context \"y\"\n}\n\n\
             decision d2 \"D2\" {\n  status accepted\n  date \"2026-03-02\"\n  decision \"x\"\n  context \"y\"\n}\n\n\
             decision d3 \"D3\" {\n  status accepted\n  date 2026-02-30\n  decision \"x\"\n  context \"y\"\n}\n\n\
             decision d4 \"D4\" {\n  status accepted\n  date \"March 2026\"\n  decision \"x\"\n  context \"y\"\n}\n\n\
             milestone m1 \"M1\" {\n  status planned\n  target_date \"2026-09-01\"\n  start_date 2026-08-01\n}\n\n\
             milestone m2 \"M2\" {\n  status planned\n  target_date \"Q3 2026\"\n}\n\n\
             release r1 \"R1\" {\n  status planned\n  release_date 2026-13-01\n}\n",
        );
    let cli = Cli::on(&dir);
    let diagnostics = cli.document(&["check"]).json;
    let codes = |code: &str| -> Vec<String> {
        diagnostics
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == code)
            .map(|d| d["message"].as_str().unwrap().to_string())
            .collect()
    };
    let e061 = codes("E061");
    assert_eq!(e061.len(), 4, "{e061:?}");
    for (needle, given) in [
        ("'d3'", "was given 2026-02-30"),
        ("'d4'", "was given \"March 2026\""),
        ("'m2'", "\"Q3 2026\""),
        ("'r1'", "2026-13-01"),
    ] {
        assert!(
            e061.iter().any(|m| m.contains(needle) && m.contains(given)),
            "{needle}: {e061:?}"
        );
    }
    for retired in ["I053", "I086", "I087"] {
        assert!(codes(retired).is_empty(), "{retired}");
    }
    let schema = cli.run(&["schema", "--kind", "decision"]).exits(0).stdout;
    assert!(schema.contains("\"field_type\": \"date\""), "{schema}");
    let ok = cli
        .run(&["set", "d1", "date", "2026-03-01", "--dry-run"])
        .exits(0);
    assert!(ok.stdout.contains("nothing to write"), "{}", ok.stdout);
    let before = std::fs::read_to_string(dir.root().join("spec/d.spec")).unwrap();
    let bad = cli.run(&["set", "d1", "date", "2026-02-31"]);
    assert_ne!(bad.code, 0);
    assert!(
        bad.stderr.contains("E080") && bad.stderr.contains("E061"),
        "{}",
        bad.stderr
    );
    assert_eq!(
        before,
        std::fs::read_to_string(dir.root().join("spec/d.spec")).unwrap()
    );
}

#[specforge_test_macros::test(
    behavior = "create_entity",
    verify = "new's help names a date field's value as YYYY-MM-DD"
)]
fn new_names_a_date_fields_value_as_a_calendar_day() {
    let dir = governed("spec/g.spec", "");
    let help = Cli::on(&dir).run(&["new", "decision", "--help"]).exits(0);
    assert!(
        help.stdout.contains("--date <YYYY-MM-DD>"),
        "{}",
        help.stdout
    );
}

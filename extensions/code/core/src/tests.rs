use super::*;
use specforge_extension_sdk::{EntityKindDescriptor, ExtensionDeclaration};

fn declaration() -> ExtensionDeclaration {
    specforge_extension_build().declaration()
}

fn kind<'d>(declaration: &'d ExtensionDeclaration, name: &str) -> &'d EntityKindDescriptor {
    declaration
        .entities
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("no kind {name}"))
}

#[test]
fn the_kinds_are_collected_with_their_schemes() {
    let declared = declaration();
    let test = kind(&declared, "test");
    assert_eq!(
        (
            test.writer.as_deref(),
            &test.schemes[..],
            test.result.as_deref()
        ),
        (Some("collector"), &["js".to_string()][..], Some("status"))
    );
    let run = kind(&declared, "test_run");
    assert_eq!(
        (
            run.writer.as_deref(),
            &run.schemes[..],
            run.result.as_deref()
        ),
        (Some("collector"), &["run".to_string()][..], None)
    );
    for k in [test, run] {
        assert_eq!(
            k.scoped_by, None,
            "{}: a scheme ref names the fact whole",
            k.name
        );
    }
    let edge = run.fields.iter().find(|f| f.name == "test").unwrap();
    assert_eq!(
        (edge.target_kind.as_deref(), edge.edge.as_deref()),
        (Some("test"), Some("run_of"))
    );
}

#[test]
fn a_status_is_a_test_status_name() {
    let names: Vec<&str> = TestStatus::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(STATUS.values(), &names[..]);
}

#[test]
fn it_catalogues_the_codes_it_reports() {
    let codes: Vec<String> = declaration().codes.into_iter().map(|c| c.code).collect();
    assert_eq!(codes, ["E101", "W102", "W401", "I402"]);
}

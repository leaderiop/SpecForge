use super::*;
use crate::containment::Containment;

#[specforge_test_macros::test(
    behavior = "discover_format_targets",
    verify = "a named path that names nothing refuses the run about paths before any file is read"
)]
#[ignore = "flipped by T2"]
fn a_named_path_that_names_nothing_is_refused_on_paths() {
    let dir = project_with(r#"{"spec_root": "spec"}"#, &[("spec/a.spec", MESSY)]);
    let bound = Containment::within(dir.path());
    let root = ProjectRoot::of(dir.path()).unwrap();
    for named in ["a.spec", "types", "nope", "spec/nope.spec", "nodir/"] {
        let paths = [PathBuf::from("spec/a.spec"), PathBuf::from(named)];
        let refusal = super::super::run(&Request {
            root: &root,
            paths: &paths,
            mode: Mode::Write,
            caller: crate::testing::bounded(&bound),
            forms: crate::authoring::format::Forms::none(),
        })
        .err()
        .unwrap_or_else(|| panic!("{named} was not refused"));
        assert_eq!(refusal.subject, Some(FormatField::Paths), "{named}");
        assert_eq!(
            (refusal.error.kind, &*refusal.error.code),
            (OpErrorKind::FileNotFound, "file_not_found")
        );
        assert_eq!(refusal.error.file(), Some(named), "{named}");
        assert_eq!(refusal.error.suggestion, None, "{named}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("spec/a.spec")).unwrap(),
            MESSY
        );
    }
}

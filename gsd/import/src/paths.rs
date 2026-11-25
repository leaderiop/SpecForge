//! The `spec/gsd/` file of each record (ADR 0391 D4): `project.spec` (the project and its decisions),
//! `requirements.spec`, `<milestone>.spec` (a milestone and its phases), `phases/<phase>.spec` (everything scoped under
//! one phase) and `quick.spec` (quick plans and their summaries).
//!
//! A name is built from ids the readers validated; `emit` refuses any record whose file `ids::file` does not take, so a
//! `/`, a space or `..` from a document never becomes a path.

/// The folder every imported entity is written under.
pub const ROOT: &str = "spec/gsd";

pub fn project() -> String {
    format!("{ROOT}/project.spec")
}

pub fn requirements() -> String {
    format!("{ROOT}/requirements.spec")
}

/// The file of the milestone `id` and its phases.
pub fn milestone(id: &str) -> String {
    format!("{ROOT}/{id}.spec")
}

/// The file of everything scoped under the phase `id`.
pub fn phase(id: &str) -> String {
    format!("{ROOT}/phases/{id}.spec")
}

pub fn quick() -> String {
    format!("{ROOT}/quick.spec")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids;

    #[test]
    fn each_record_has_its_file() {
        assert_eq!(phase("02.1"), "spec/gsd/phases/02.1.spec");
        assert_eq!(milestone("v1.0"), "spec/gsd/v1.0.spec");
        for file in [
            project(),
            requirements(),
            milestone("v1.0"),
            phase("02.1"),
            quick(),
        ] {
            assert!(ids::file(&file), "{file}");
        }
    }

    #[test]
    fn a_name_no_id_takes_makes_no_file_emit_writes() {
        for bad in [phase(".."), phase("a b"), milestone("../x"), phase("")] {
            assert!(!ids::file(&bad), "{bad}");
        }
    }
}

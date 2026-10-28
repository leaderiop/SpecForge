//! The recorded test report as a judged rule's `test(..)` input reads it.

use std::io::ErrorKind;
use std::path::Path;

use specforge_protocol_types::TestResults;
use specforge_registry::{Disk, EntitySnapshot};

use crate::coverage::{REPORT_FILE, TestReport};

pub(super) enum Report {
    /// No `specforge-report.json`: test inputs are not judged (I205).
    Absent,
    /// It is there and unusable (E045), with why.
    Unusable(String),
    Read(TestResults),
}

pub(super) fn read(root: &Path, entities: &EntitySnapshot, disk: &dyn Disk) -> Report {
    let path = root.join(REPORT_FILE);
    match disk.read(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Report::Absent,
        Err(e) => Report::Unusable(crate::coverage::ReportError::unreadable(&path, &e).to_string()),
        Ok(bytes) => match TestReport::parse(&path, &bytes) {
            Ok(report) => Report::Read(report.results(entities)),
            Err(e) => Report::Unusable(e.to_string()),
        },
    }
}

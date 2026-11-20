//! `record_security_verdict`: writes a judge's verdict on a judged `gsd.security/` rule, built by
//! `gsd_kit::judged::record` over the extension's own table (`crate::judged`). It has its own name: a command of the
//! same short and name as core's `record_verdict` would be shadowed, never run. It looks a rule up in this table only,
//! and the host refuses a verdict written by a command of another extension than the rule's.

use gsd_kit::judged::record::{Recorder, declare_record};
use specforge_extension_sdk::prelude::*;

use crate::judged::JUDGED;

static RECORDER: Recorder = Recorder {
    command: "record_security_verdict",
    namespace: "gsd.security",
    extension: "@gsd/security",
    table: JUDGED,
    checks: &[],
};

pub fn declare(c: &mut ContributionsBuilder) {
    declare_record(c, &RECORDER);
}

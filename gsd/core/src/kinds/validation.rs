//! The kinds of Nyquist validation (`spec/verify.md`): a phase's validation, its per-task rows, its wave-0 items and
//! its manual checks. Statuses are computed. `validation_row.threat` is `@gsd/security`'s.

use super::dsl::*;

const VALIDATION_STATUS: &[&str] = &["draft", "validated"];
const TEST_TYPE: &[&str] = &["unit", "integration", "smoke", "e2e", "manual"];
const ROW_STATUS: &[&str] = &["pending", "green", "red", "flaky"];
const WAVE0_KIND: &[&str] = &["test_stub", "fixture", "framework_install"];

const APPROVAL: &[Field] = &[str("by"), ts("at")];

/// `NN-VALIDATION.md`, named `01-VALIDATION` and scoped by its phase.
pub const VALIDATION: Kind = Kind::new(
    "validation",
    "How a phase's tasks are sampled by tests (GSD NN-VALIDATION.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase:`"),
        str("context_digest")
            .h()
            .doc("the digest of the phase's context when it was recorded (gsd/W406)"),
        str("framework").doc("`framework`"),
        // A `str`, not a `file`: the config may be a file the phase's wave 0 creates.
        str("config_file").doc("`config_file`"),
        str("quick_cmd").doc("`quick_cmd`"),
        str("full_cmd").doc("`full_cmd`"),
        int("max_latency_s").doc("`max_latency_s`"),
        block("approval", APPROVAL)
            .h()
            .doc("written by the command that approves it, with the host's clock"),
        en("status", VALIDATION_STATUS)
            .c()
            .doc("validated once `approval` is set"),
        bool("nyquist_compliant")
            .c()
            .doc("every row green and every task covered"),
        bool("wave_0_complete").c().doc("every wave-0 item done"),
    ],
)
.scoped_by("phase");

/// A line of the Per-Task Verification Map, named `VR1` and scoped by its validation.
pub const VALIDATION_ROW: Kind = Kind::new(
    "validation_row",
    "How one task is verified (GSD Per-Task Verification Map)",
    &[
        rf("validation", "validation").r().i().doc("parent"),
        rf("task", "task").r().doc("the task"),
        rf("requirement", "requirement").doc("the requirement it serves"),
        en("test_type", TEST_TYPE).r().doc("`Test Type`"),
        str("command").doc("`Automated Command`"),
        int("wave").c().doc("the task's wave"),
        bool("file_exists").c().doc("the test file exists"),
        en("status", ROW_STATUS)
            .c()
            .doc("green or red from the task's linked tests; `flaky` is never derived"),
    ],
)
.scoped_by("validation");

/// A Wave 0 requirement, named `W0-1` and scoped by its validation.
pub const WAVE0_ITEM: Kind = Kind::new(
    "wave0_item",
    "A file wave 0 must create before the tasks can be sampled (GSD Wave 0 Requirements)",
    &[
        rf("validation", "validation").r().i().doc("parent"),
        // A `str`: the file is the one wave 0 will create.
        str("path").r().doc("the file"),
        rf("for", "requirement").doc("the requirement it serves"),
        en("kind", WAVE0_KIND).r().doc("what it is"),
        bool("done")
            .c()
            .doc("the file exists and a linked test ran"),
    ],
)
.scoped_by("validation");

/// A behaviour only a person can check, named `MC1` and scoped by its validation.
pub const MANUAL_CHECK: Kind = Kind::new(
    "manual_check",
    "A behaviour with no automated check (GSD Manual-Only Verifications)",
    &[
        rf("validation", "validation").r().i().doc("parent"),
        str("behavior").r().doc("the behaviour"),
        rf("requirement", "requirement").doc("the requirement it serves"),
        str("why_manual").r().doc("why it cannot be automated"),
        str("instructions").r().doc("how to check it"),
    ],
)
.scoped_by("validation");

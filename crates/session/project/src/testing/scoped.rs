//! The GSD-shaped kinds the scope tests share (ADR 0284): a workstream holds milestones, a milestone phases and an
//! audit, a phase plans and decisions, a plan tasks. A scope field is optional where GSD leaves it out: a project
//! decision, a quick plan, a milestone of no workstream.

use specforge_extension_sdk::prelude::*;

/// The name the extension declaring [`scoped_kinds`] is served under.
pub const SCOPED_EXTENSION: &str = "@test/scoped";

/// The kinds, as the extension `@test/scoped` contributes them.
pub fn scoped_kinds(c: &mut ContributionsBuilder) {
    let one = |f: &mut FieldBuilder, kind: &str| {
        f.field_type(FieldType::Reference).target_kind(kind);
    };
    let many = |f: &mut FieldBuilder, kind: &str| {
        f.field_type(FieldType::ReferenceList).target_kind(kind);
    };
    c.kind("Workstream", |k| {
        k.keyword("workstream");
    });
    c.kind("Milestone", |k| {
        k.keyword("milestone");
        k.field("workstream", |f| one(f, "workstream"));
        k.field("audit", |f| one(f, "audit"));
    });
    c.kind("Phase", |k| {
        k.keyword("phase");
        k.field("milestone", |f| one(f, "milestone"));
    });
    c.kind("Plan", |k| {
        k.keyword("plan").testable(true).supports_verify(true);
        k.field("phase", |f| one(f, "phase"));
        k.field("depends_on", |f| many(f, "plan"));
        k.field("decisions", |f| many(f, "decision"));
    });
    c.kind("Decision", |k| {
        k.keyword("decision");
        k.field("phase", |f| one(f, "phase"));
        k.field("superseded_by", |f| one(f, "decision"));
    });
    c.kind("Task", |k| {
        k.keyword("task");
        k.field("plan", |f| one(f, "plan"));
    });
    c.kind("Audit", |k| {
        k.keyword("audit");
        k.field("milestone", |f| one(f, "milestone"));
    });
    c.kind("Requirement", |k| {
        k.keyword("requirement");
    });
    // A kind whose field types name entities, scoped by a phase.
    c.kind("Shape", |k| {
        k.keyword("shape").open_fields(true);
        k.field("phase", |f| one(f, "phase"));
        k.field("parts", |f| {
            many(f, "shape");
            f.derived_from("type_expressions");
        });
    });
}

/// The extension `@test/scoped`.
pub fn scoped_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(SCOPED_EXTENSION, "1.0.0"));
    scoped_kinds(&mut c);
    c
}

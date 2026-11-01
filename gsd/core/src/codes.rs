//! The code book: every `gsd/` code of the model (`spec/rules.md`), declared in the `codes` category so `specforge
//! explain gsd/E101` and the editor describe it. The host qualifies a local code with the namespace (`E101` is
//! `gsd/E101`, K10). A code whose rule lands in a later round is declared now and never reported until then: the
//! kernel needs a gate, a setting or a judged rule to name a catalogued code.

use specforge_extension_sdk::prelude::*;

/// A code's severity letter, and what a stricter profile raises it to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn letter(self) -> char {
        match self {
            Severity::Error => 'E',
            Severity::Warning => 'W',
            Severity::Info => 'I',
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Severity::Error => "an error",
            Severity::Warning => "a warning",
            Severity::Info => "information",
        }
    }
}

pub struct Code {
    /// The local code: `E101`.
    pub id: &'static str,
    pub title: &'static str,
    /// The rule, as `spec/rules.md` words it.
    pub rule: &'static str,
    /// The `rules.md` section the rule is in, as a Markdown anchor.
    pub section: &'static str,
    /// The code's letter: its severity under `gsd-compat` (for a SpecForge-only rule, its severity when on).
    pub severity: Severity,
    /// The severity under `gsd-strict` and `gsd-plus`; never below `severity`.
    pub strict: Severity,
}

impl Code {
    fn explanation(&self) -> String {
        let raised = if self.strict == self.severity {
            String::new()
        } else {
            format!(", and {} under gsd-strict", self.strict.noun())
        };
        format!("{}. It is {}{raised}.", self.rule, self.severity.noun())
    }

    fn docs(&self) -> String {
        format!("{RULES}#{}", self.section)
    }
}

/// The kernel refuses a `docs` that is not an absolute http(s) URL (E030), so this is the one place to change when
/// `@gsd/core` moves to its own repository.
const RULES: &str = "https://github.com/leaderiop/SpecForge/blob/main/gsd/core/spec/rules.md";

const fn code(
    id: &'static str,
    title: &'static str,
    rule: &'static str,
    section: &'static str,
    severity: Severity,
    strict: Severity,
) -> Code {
    Code {
        id,
        title,
        rule,
        section,
        severity,
        strict,
    }
}

/// The 63 codes of `spec/rules.md`, in the order of its tables.
pub static CODES: &[Code] = &[
    code(
        "E101",
        "Requirement not mapped to a phase",
        "Every v1 requirement maps to exactly one phase",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E102",
        "Dependency cycle or forward dependency",
        "Phase and plan depends_on are acyclic and point at earlier ones only",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W103",
        "Phase success criteria out of range",
        "A phase has 2–5 success criteria",
        "project-and-roadmap",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W104",
        "Phase numbers not continuous",
        "Phase numbers are continuous; an inserted phase sits between its neighbours",
        "project-and-roadmap",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "E105",
        "Out-of-scope requirement has no reason",
        "An out_of_scope requirement has a reason",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E106",
        "Milestone completes without a passing audit",
        "A milestone completes only with a passing (or accepted) audit",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E107",
        "Timestamp is not RFC 3339 UTC",
        "A timestamp field holds an RFC 3339 UTC time (every authored ts field is a str that matches)",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E108",
        "More than one project",
        "A repository has one project (the kernel keeps `singleton` as metadata only)",
        "project-and-roadmap",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E201",
        "Context has no canonical refs",
        "A context has canonical refs (missing_required_field and non_empty under one code)",
        "discuss-spec-research",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E202",
        "Phase spec exceeds its ambiguity gates",
        "A phase spec meets its ambiguity gates",
        "discuss-spec-research",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W203",
        "Locked requirement is not falsifiable",
        "A locked requirement has a falsifiable acceptance",
        "discuss-spec-research",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W204",
        "Open research question unresolved",
        "A research open question is resolved by a decision before planning",
        "discuss-spec-research",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W205",
        "Package without a legitimacy verdict",
        "A package in research has a legitimacy verdict",
        "discuss-spec-research",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E206",
        "More than one context, spec, log or research",
        "A phase has at most one context, phase spec, discussion log and research, a quick plan at most one context and research, and the project at most one research, named RESEARCH. A context names exactly one owner (`phase` or a quick plan in `plan`), a decision and a research at most one; a `plan` owner has no phase, and a quick plan's research is not named RESEARCH",
        "discuss-spec-research",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E301",
        "Phase requirement in no plan",
        "Every requirement of a phase is in at least one of its plans",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E302",
        "Plan names no requirement",
        "A plan of a phase names at least one requirement (a quick plan names none)",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E303",
        "Task is missing a required field",
        "An auto, tdd or tracer task has files and a verify (action and done are required fields)",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W304",
        "Plan has too many tasks",
        "A plan has at most 3 tasks (4 warns)",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W305",
        "Same-wave plans overlap on files",
        "Plans of one wave share no path in files_modified ∪ files_deleted; under compat that wave runs sequentially",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W306",
        "Task file outside the plan's files",
        "A task's files are among its plan's files_modified",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E307",
        "Checkpoint option or fields invalid",
        "Auto_select and answer.option name one of the checkpoint's options, option names are unique, and each checkpoint kind has its fields",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W308",
        "One-way task without a decision or a settled_by",
        "A one_way task follows a decision checkpoint of its plan, or names the decision that settles it in reversibility.settled_by",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W309",
        "Plan lacks a truth or an artifact",
        "A plan has a truth and an artifact",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E310",
        "Context decision cited by no plan",
        "Every trackable decision of a planned phase's context (origin context) is cited by one of its plans",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W311",
        "Plan does not achieve its phase goal",
        "The plan achieves its phase goal, keeps to scope and contradicts no settled decision",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E312",
        "Task action reduces scope",
        "No task action reduces scope (\"for now\", \"simplified\", \"v1\")",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W313",
        "Key links not wired",
        "Key links are wired between artifacts the plan names",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E314",
        "Task without an automated verify or a wave-0 item",
        "Each task has an automated verify or a wave-0 item (Nyquist)",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W315",
        "Data contracts between plans mismatch",
        "Data contracts between plans match (requires / provides)",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W316",
        "Plan ignores agent instructions or mapped patterns",
        "The plan follows the project's agent instructions and the mapped patterns",
        "plan",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "W317",
        "Post-planning gaps open",
        "The post-planning gap report finds nothing open",
        "plan",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "W318",
        "Plan-check verdict is stale",
        "The plan-check verdict is fresh before execution",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E319",
        "Prohibition names no test",
        "A prohibition with verification test names a test",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E320",
        "Verify command names a missing path",
        "A task's verify command names paths that exist",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "I321",
        "Justified coupling between plans",
        "A coupling_justified item names a peer of the same phase and wave: the applied exemption is reported as an advisory",
        "plan",
        Severity::Info,
        Severity::Info,
    ),
    code(
        "E322",
        "Plan has five or more tasks",
        "A plan has fewer than 5 tasks",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W323",
        "Plan not named after its phase",
        "A plan is named <phase>-<NN> in its phase, and by its quick id YYMMDD-xxx without one",
        "plan",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W324",
        "Verify command not checked",
        "A task's verify command runs in a dynamic or outside-the-project directory, or its package.json is unreadable or has no such script",
        "plan",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "E507",
        "Verify command names no failing direction",
        "Each task's verify commands name their failing direction (fails_when), one statement per automated",
        "plan",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E401",
        "Summary requirements differ from the plan's",
        "A summary's requirements_completed equals its plan's requirements",
        "execute",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E402",
        "Halted summary says nothing why",
        "A halted summary says why",
        "execute",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W403",
        "GREEN commit that descends from no RED commit (green_before_red)",
        "A tdd task's GREEN commit descends from a RED test commit, by its recorded ancestry, or by time when it was recorded without ancestry (the RED run's failure is not checked; a warning, as the order cannot be repaired after the fact)",
        "execute",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "W404",
        "TDD review checkpoint did not run",
        "The TDD review checkpoint ran after execution",
        "execute",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E405",
        "Schema changed with no database push recorded",
        "A phase that changed an ORM's schema files records its database push in a summary or a commit subject, once it has run",
        "execute",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W406",
        "Context drifted since the plan was written",
        "The context the plan was written against has not drifted (the context_digest a research, spec or validation recorded differs from the context's current digest)",
        "execute",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "W407",
        "Task has no commit",
        "Every task of a complete plan has a recorded commit (a manual task is exempt)",
        "execute",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "W408",
        "Deliverable checks neither pass nor are human-judged",
        "A deliverable's checks pass, or a person judged it and gave a rationale (a check no report speaks of is not judged)",
        "execute",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E501",
        "Verifier verdict is stale",
        "The verifier's verdict on each truth, artifact and key link is fresh",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E502",
        "Explicit truth verified without a passing test",
        "An explicit truth is verified only with a passing test recorded for it",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E503",
        "Override lacks its author or date",
        "An override names who accepted the truth and when (by and at)",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W504",
        "More than three overrides in a phase",
        "More than three overrides in a phase",
        "verify",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "E505",
        "Debt marker without an issue",
        "Debt markers (TBD, FIXME, XXX) in the files a plan modifies name an issue on the same line",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W506",
        "Settled decision maps to nothing shipped",
        "Every settled decision maps to something shipped",
        "verify",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "E509",
        "UAT evidence that cannot pass",
        "A skipped UAT test has a reason and an issue has the person's report",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W510",
        "Phase lacks a validation it needs",
        "A planned phase whose research asks for a validation has one, approved before its verification",
        "verify",
        Severity::Warning,
        Severity::Error,
    ),
    code(
        "E511",
        "Phase shipped before verification passed",
        "A phase ships only when its verification passed (a gate code of complete_milestone and /gsd-ship)",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E512",
        "Broken window open at ship",
        "No broken window is open at ship",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E513",
        "Resolved window changed again",
        "A resolved window cannot be waived or fixed again (waiver and fixed_at are never both set)",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "W514",
        "Refactor window recorded while enforcement is off",
        "No refactor deviation window is recorded while workflow.windows_enforce is off",
        "verify",
        Severity::Warning,
        Severity::Warning,
    ),
    code(
        "E515",
        "Complete requirement without a traced passing test",
        "A complete requirement traces to a task, a commit and a passing test",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E516",
        "Stale verdict blocks the workflow",
        "A stale verdict blocks every workflow step",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E517",
        "Active plans of different workstreams overlap",
        "Active plans of different workstreams share no path in files_modified ∪ files_deleted",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
    code(
        "E518",
        "Integration verdict is stale",
        "The integration checker's verdict on a milestone audit is fresh",
        "verify",
        Severity::Error,
        Severity::Error,
    ),
];

pub fn declare(c: &mut ContributionsBuilder) {
    for k in CODES {
        c.code(k.id, |d| {
            d.title(k.title)
                .explanation(&k.explanation())
                .docs(&k.docs());
        });
    }
}

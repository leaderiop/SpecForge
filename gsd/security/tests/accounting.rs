//! `injection-blocking-config.test.cjs` (#1577), accounted case by case: each of its five cases is ported, and
//! `tests/cli/injection.rs` runs it. Core's own suite skips the file with "@gsd/security's".

#[allow(
    dead_code,
    reason = "core's accounting serves every suite; this one uses `account`"
)]
#[path = "../../core/src/hooks/accounting.rs"]
mod accounting;

const BLOCKING_CONFIG: &str =
    include_str!("../../parity/upstream/v1.18.0/tests/injection-blocking-config.test.cjs");

/// `(upstream case, the test of tests/cli/injection.rs that runs it)`.
const PORTED: &[(&str, &str)] = &[
    (
        "isValidConfigKey accepts security.injection_blocking",
        "the_key_is_declared_and_defaults_to_false",
    ),
    (
        "bare security section is not a settable leaf key",
        "the_bare_security_section_is_not_a_setting",
    ),
    (
        "CONFIG_DEFAULTS ships injection_blocking = false (advisory by default)",
        "the_key_is_declared_and_defaults_to_false",
    ),
    (
        "config-set writes the nested shape the hook reads, and round-trips",
        "set_then_read_back_and_the_scanner_sees_it",
    ),
    (
        "a fresh project has no injection_blocking key \u{2014} hook sees absent \u{2192} advisory",
        "a_fresh_project_has_no_value_so_the_scanner_is_advisory",
    ),
];

#[test]
fn the_five_cases_of_1577_are_ported() {
    let ported: Vec<String> = PORTED.iter().map(|(case, _)| (*case).to_string()).collect();
    accounting::account(BLOCKING_CONFIG, &ported, &[]);
    let tests = include_str!("cli/injection.rs");
    for (case, test) in PORTED {
        assert!(
            tests.contains(&format!("fn {test}()")),
            "{case}: no test {test}"
        );
    }
}

// `secure-phase.test.cjs` (67 cases): each describe-block is run by one test below, over the ported assets, the
// descriptors and core's assets (a case about the planner, the VALIDATION template or `planning-config.md` is about the
// file `@gsd/core` ports). A case about upstream's installed layout or config template is skipped with its reason.

const SECURE_PHASE: &str =
    include_str!("../../parity/upstream/v1.18.0/tests/secure-phase.test.cjs");

fn asset(path: &str) -> &'static str {
    gsd_security::prompts::asset(path).unwrap_or_else(|| panic!("no asset {path}"))
}

fn workflow() -> &'static str {
    asset("gsd-core/workflows/secure-phase.md")
}

fn command() -> &'static str {
    asset("commands/gsd/secure-phase.md")
}

fn template() -> &'static str {
    asset("gsd-core/templates/SECURITY.md")
}

fn core_asset(path: &str) -> &'static str {
    gsd_core::prompts::asset(path).unwrap_or_else(|| panic!("no core asset {path}"))
}

fn core_agent(id: &str) -> &'static str {
    gsd_core::agents::roster::AGENTS
        .iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("no agent {id}"))
        .full
}

fn auditor() -> &'static str {
    gsd_security::agents::FULL
}

fn has_all(text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(text.contains(needle), "missing {needle}");
    }
}

fn described(category: &str) -> serde_json::Value {
    let host = specforge_extension_sdk::testing::MockHost::new(gsd_security::build());
    let text = host.describe_json(category).expect("a category");
    serde_json::from_str(&text).expect("JSON")
}

fn declared() -> serde_json::Value {
    serde_json::to_value(gsd_security::build().declaration().surfaces).expect("json")
}

#[test]
fn the_auditor_cases() {
    let surfaces = declared();
    let a = &surfaces["agents"][0];
    assert_eq!(a["id"], "gsd-security-auditor");
    for key in ["description", "tools", "color"] {
        assert!(!a[key].is_null(), "{key}");
    }
    let tools = a["tools"].to_string();
    for tool in ["read", "shell", "search"] {
        assert!(tools.contains(tool), "{tool}");
    }
    assert!(!tools.contains("write") && !tools.contains("edit"), "#2119");
    has_all(
        auditor(),
        &[
            "<role>",
            "</role>",
            "<execution_flow>",
            "</execution_flow>",
            "<structured_returns>",
            "## SECURED",
            "## OPEN_THREATS",
            "## ESCALATE",
            "<success_criteria>",
            "</success_criteria>",
            "READ-ONLY",
        ],
    );
}

#[test]
fn the_command_cases() {
    let surfaces = declared();
    let slash = surfaces["slash_commands"]
        .as_array()
        .expect("slash")
        .iter()
        .find(|s| s["prompt"] == "secure_phase")
        .expect("the slash command of secure_phase");
    assert_eq!(slash["argument_hint"], "[phase number]");
    assert_eq!(
        slash["tools"],
        serde_json::json!([
            "read", "write", "edit", "shell", "search", "agent", "ask_user"
        ])
    );
    assert_eq!(slash["attribution"]["path"], "commands/gsd/secure-phase.md");
    has_all(
        command(),
        &[
            "secure-phase.md",
            "<objective>",
            "(A)",
            "(B)",
            "(C)",
            "<execution_context>",
        ],
    );
}

#[test]
fn the_workflow_cases() {
    let w = workflow();
    has_all(
        w,
        &[
            "gsd-security-auditor",
            "threats_open",
            // the capability check: the hooks of every capability extension at `verify:post`
            "-hooks\" verify:post",
            "ref.skill == \"secure-phase\"",
            "SECURITY.md",
            "<success_criteria>",
            "</success_criteria>",
        ],
    );
    assert!(!w.contains("config-get workflow.security_enforcement"));
}

#[test]
fn the_template_cases() {
    let t = template();
    let front = t.split("---").nth(1).expect("frontmatter");
    for field in [
        "phase",
        "slug",
        "status",
        "threats_open",
        "asvs_level",
        "created",
    ] {
        assert!(front.contains(&format!("{field}:")), "{field}");
    }
    has_all(
        t,
        &[
            "## Trust Boundaries",
            "## Threat Register",
            "Threat ID",
            "Category",
            "Component",
            "Severity",
            "Disposition",
            "Mitigation",
            "Status",
            "## Accepted Risks Log",
            "## Security Audit Trail",
            "## Sign-Off",
            "- [ ]",
        ],
    );
}

#[test]
fn the_config_cases() {
    let json = described("settings");
    let default = |key: &str| {
        json["items"]
            .as_array()
            .expect("settings")
            .iter()
            .find(|s| s["kind"] == "setting" && s["name"] == key)
            .unwrap_or_else(|| panic!("no setting {key}"))["default"]
            .clone()
    };
    assert_eq!(default("workflow.security_enforcement"), true);
    assert_eq!(default("workflow.security_asvs_level"), 1);
    assert_eq!(default("workflow.security_block_on"), "high");
}

#[test]
fn the_validation_cases() {
    let v = core_asset("gsd-core/templates/VALIDATION.md");
    let header = v
        .lines()
        .find(|l| l.contains("Threat Ref") && l.contains("Secure Behavior"))
        .expect("both columns in one header");
    assert!(header.starts_with('|'));
    assert!(v.find("Threat Ref") > v.find("## Per-Task Verification Map"));
    // and the model: the row gains the two fields (`declaration.rs` pins them)
    let json = described("enhancements").to_string();
    assert!(json.contains("secure_behavior") && json.contains("\"threat\""));
}

fn row_of(text: &str, key: &str) -> String {
    let at = text.find(key).expect(key);
    text[at..].lines().next().expect("a line").to_string()
}

#[test]
fn the_severity_gate_cases() {
    let planner = core_agent("gsd-planner");
    has_all(
        planner,
        &[
            "| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |",
            "critical|high|medium|low",
            "Every threat has a Severity (critical|high|medium|low)",
        ],
    );
    let a = auditor();
    has_all(
        a,
        &[
            "block_on",
            "critical",
            "none",
            "critical > high > medium > low",
            "threats_open",
            "severity rank",
            "non-blocking",
            "below",
            "severity ≥ `block_on`",
            "OPEN-non-blocking (severity below block_on)",
            "Fail-closed",
            "missing",
        ],
    );
    for line in a.lines() {
        assert!(
            !line.contains("must not ship") || line.contains("severity"),
            "{line}"
        );
    }
    assert!(template().contains("Severity"));
    let row = row_of(
        core_asset("gsd-core/references/planning-config.md"),
        "security_block_on",
    );
    assert!(row.contains("critical") && row.contains("none"), "{row}");
    // upstream's docs/CONFIGURATION.md row: the declared setting lists the same values
    let json = described("settings").to_string();
    assert!(json.contains("critical") && json.contains("none"));
    let w = workflow();
    assert!(w.contains("blocking threats") || w.contains("block threshold"));
    assert!(
        w.contains("threat_id, category, component, severity, disposition, mitigation_pattern")
    );
}

#[test]
fn the_behaviour_cases() {
    let a = auditor().to_lowercase();
    assert!(!a.contains("scan for vulnerabilities") && !a.contains("find vulnerabilities"));
    has_all(
        auditor(),
        &["mitigate", "accept", "transfer", "OPEN", "CLOSED"],
    );
    let w = workflow();
    assert!(w.contains("threats_open") && (w.contains("BLOCKED") || w.contains("blocked")));
    assert!(w.contains("Do NOT emit next-phase routing"));
}

#[test]
fn the_config_resolution_cases() {
    let w = workflow();
    let injection = w
        .find("block_on: {SECURITY_BLOCK_ON}")
        .expect("the config line");
    for var in ["SECURITY_ASVS=", "SECURITY_BLOCK_ON="] {
        let at = w
            .find(var)
            .unwrap_or_else(|| panic!("{var} is not assigned"));
        assert!(at < injection, "{var} after the auditor's config");
    }
    // `config-get <key> ... || echo <default>` is `{{setting:<key>|<default>}}`: the key, the registry default, and the
    // value filled bare (`tests/cli/prompt.rs` renders it)
    has_all(
        w,
        &[
            "{{setting:workflow.security_asvs_level|1}}",
            "{{setting:workflow.security_block_on|high}}",
        ],
    );
}

#[test]
fn the_short_circuit_cases() {
    let w = workflow();
    assert!(w.contains("register_authored_at_plan_time"));
    assert!(w.contains("threats_open: 0 AND register_authored_at_plan_time"));
    assert!(w.contains("retroactive") || w.contains("Retroactive"));
    assert!(
        (w.contains("Verify mitigations") || w.contains("verify mitigations"))
            && w.contains("Retroactive")
    );
}

/// `(case, the test that runs it)`: a case that is a describe-block's is run by that block's test.
const SECURE_PORTED: &[(&str, &str)] = &[
    ("agent file exists", "the_auditor_cases"),
    (
        "has valid frontmatter with name, description, tools, color",
        "the_auditor_cases",
    ),
    ("name is gsd-security-auditor", "the_auditor_cases"),
    (
        "tools include Read, Bash, Glob, Grep but NOT Write or Edit (#2119)",
        "the_auditor_cases",
    ),
    ("has <role> section", "the_auditor_cases"),
    ("has <execution_flow> section", "the_auditor_cases"),
    (
        "has <structured_returns> with SECURED, OPEN_THREATS, ESCALATE",
        "the_auditor_cases",
    ),
    ("has <success_criteria> section", "the_auditor_cases"),
    (
        "has READ-ONLY rule \u{2014} does NOT modify implementation files",
        "the_auditor_cases",
    ),
    ("command file exists", "the_command_cases"),
    (
        "has valid frontmatter with name gsd:secure-phase",
        "the_command_cases",
    ),
    ("has allowed-tools list", "the_command_cases"),
    (
        "contains reference to secure-phase.md workflow",
        "the_command_cases",
    ),
    (
        "has <objective> section mentioning states A, B, C",
        "the_command_cases",
    ),
    ("workflow file exists", "the_workflow_cases"),
    (
        "contains gsd-security-auditor reference",
        "the_workflow_cases",
    ),
    (
        "contains threats_open enforcement logic",
        "the_workflow_cases",
    ),
    (
        "contains security capability hook check",
        "the_workflow_cases",
    ),
    (
        "contains SECURITY.md template reference",
        "the_workflow_cases",
    ),
    ("has success_criteria section", "the_workflow_cases"),
    ("template exists", "the_template_cases"),
    (
        "has YAML frontmatter with required fields",
        "the_template_cases",
    ),
    ("has ## Trust Boundaries section", "the_template_cases"),
    (
        "has ## Threat Register table with required columns",
        "the_template_cases",
    ),
    ("has ## Accepted Risks Log section", "the_template_cases"),
    ("has ## Security Audit Trail section", "the_template_cases"),
    ("has sign-off checklist", "the_template_cases"),
    (
        "threats_open field is present (terminal condition field)",
        "the_template_cases",
    ),
    ("config template exists", "the_config_cases"),
    (
        "has workflow.security_enforcement set to true",
        "the_config_cases",
    ),
    (
        "has workflow.security_asvs_level set to 1",
        "the_config_cases",
    ),
    (
        "has workflow.security_block_on set to \"high\"",
        "the_config_cases",
    ),
    ("VALIDATION.md template exists", "the_validation_cases"),
    ("contains Threat Ref column header", "the_validation_cases"),
    (
        "contains Secure Behavior column header",
        "the_validation_cases",
    ),
    (
        "both columns appear in the Per-Task Verification Map table",
        "the_validation_cases",
    ),
    (
        "gsd-planner.md threat_model register header has Severity column",
        "the_severity_gate_cases",
    ),
    (
        "gsd-planner.md security instruction assigns severity to each threat",
        "the_severity_gate_cases",
    ),
    (
        "gsd-planner.md checklist has Severity item",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md block_on domain is severity vocabulary",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md defines severity ordering critical > high > medium > low",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md threats_open counts only open threats at or above block_on",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md documents non-blocking below-threshold opens",
        "the_severity_gate_cases",
    ),
    (
        "SECURITY.md template Threat Register has Severity column",
        "the_severity_gate_cases",
    ),
    (
        "planning-config.md security_block_on row lists critical",
        "the_severity_gate_cases",
    ),
    (
        "planning-config.md security_block_on row lists none",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md BLOCKER classification conditions blocking on severity threshold (no unconditional all-open-blocks language)",
        "the_severity_gate_cases",
    ),
    (
        "gsd-security-auditor.md states fail-closed rule for missing/unranked severity",
        "the_severity_gate_cases",
    ),
    (
        "secure-phase.md prose reflects blocking-threshold semantics for threats_open",
        "the_severity_gate_cases",
    ),
    (
        "secure-phase.md Step 2c per-threat shape includes severity",
        "the_severity_gate_cases",
    ),
    (
        "docs/CONFIGURATION.md security_block_on mentions critical and none",
        "the_severity_gate_cases",
    ),
    (
        "agent does NOT contain \"scan for vulnerabilities\" (verifies, not scans)",
        "the_behaviour_cases",
    ),
    (
        "agent does NOT contain \"find vulnerabilities\" (verifies, not scans)",
        "the_behaviour_cases",
    ),
    (
        "agent contains mitigate, accept, transfer disposition types",
        "the_behaviour_cases",
    ),
    (
        "agent contains OPEN and CLOSED status values",
        "the_behaviour_cases",
    ),
    (
        "workflow contains enforcing gate (threats_open + block pattern)",
        "the_behaviour_cases",
    ),
    (
        "SECURITY_ASVS is assigned (not only used as placeholder)",
        "the_config_resolution_cases",
    ),
    (
        "SECURITY_BLOCK_ON is assigned (not only used as placeholder)",
        "the_config_resolution_cases",
    ),
    (
        "SECURITY_ASVS assignment appears before the auditor <config> injection line",
        "the_config_resolution_cases",
    ),
    (
        "SECURITY_BLOCK_ON assignment appears before the auditor <config> injection line",
        "the_config_resolution_cases",
    ),
    (
        "security config resolved via config-get with correct keys and defaults",
        "the_config_resolution_cases",
    ),
    (
        "security config-get uses --raw so the injected string value is unquoted",
        "the_settings_are_filled_bare_into_the_assignments",
    ),
    (
        "Step 2c tracks register_authored_at_plan_time",
        "the_short_circuit_cases",
    ),
    (
        "Step 3 short-circuit requires both conditions",
        "the_short_circuit_cases",
    ),
    (
        "retroactive-STRIDE mode is documented for legacy phases",
        "the_short_circuit_cases",
    ),
    (
        "Step 5 auditor constraint varies by mode",
        "the_short_circuit_cases",
    ),
];

const SECURE_SKIPPED: &[(&str, &str)] = &[(
    "SECURE: config.json security defaults > security_enforcement appears after nyquist_validation",
    "installer: the order of keys in upstream's config.json template; settings are declared, not ordered in a file",
)];

#[test]
fn the_67_cases_of_secure_phase_are_accounted() {
    let ported: Vec<String> = SECURE_PORTED
        .iter()
        .map(|(case, _)| (*case).to_string())
        .collect();
    accounting::account(SECURE_PHASE, &ported, SECURE_SKIPPED);
    let here = include_str!("accounting.rs");
    let render = include_str!("cli/prompt.rs");
    for (case, test) in SECURE_PORTED {
        let call = format!("fn {test}()");
        assert!(
            here.contains(&call) || render.contains(&call),
            "{case}: no test {test}"
        );
    }
}

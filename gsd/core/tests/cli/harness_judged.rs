//! The judged rules in the harness: which diagnostics are a judged rule's, what a project with no verdict reports,
//! and playing a judge (reading a rule's inputs and recording its verdict). A child of `harness.rs`, so it reaches
//! [`Project`]'s fields; `@gsd/security`'s tests compile it through their include of `harness.rs`.

use super::{Project, Run};

/// Whether a diagnostic code (`gsd/W311`) is one of the judged rules'.
pub fn is_judged(code: &str) -> bool {
    let local = code.strip_prefix("gsd/").unwrap_or(code);
    gsd_core::judged::JUDGED.iter().any(|j| j.code == local)
}

/// The `(code, entity)` pairs of the judged rules' diagnostics in a report from [`Project::report_all`], sorted. The
/// entity is the quoted id every judged message carries.
pub fn judged(report: &[(String, String)]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report
        .iter()
        .filter(|(code, _)| is_judged(code))
        .map(|(code, message)| {
            let id = message
                .split('\'')
                .nth(1)
                .unwrap_or_else(|| panic!("a judged message names its entity: {message}"));
            (code.clone(), id.to_string())
        })
        .collect();
    found.sort();
    found
}

/// What a report holds besides the judged rules' diagnostics.
pub fn unjudged(report: &[(String, String)]) -> Vec<(String, String)> {
    report
        .iter()
        .filter(|(code, _)| !is_judged(code))
        .cloned()
        .collect()
}

impl Project {
    /// The `(code, entity)` pairs every judged rule reports on a project that records no verdict, sorted: one for each
    /// entity of each kind the rule targets (the graph says which, not the judged diagnostics), except the rules in
    /// `off` (a gate that is off), an entity that writes a field the target says to skip and an entity that already
    /// carries a fresh passing verdict of the rule.
    pub fn expected_judged(&self, off: &[&str]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for rule in gsd_core::judged::JUDGED
            .iter()
            .filter(|r| !off.contains(&r.code))
        {
            for target in rule.targets {
                for node in self.nodes_of(target.kind) {
                    let skipped = target
                        .unless_set
                        .iter()
                        .any(|f| node["fields"].get(f).is_some_and(|v| !v.is_null()));
                    // A fresh passing verdict on the node (the twin carries its verifier's) answers the rule.
                    let judged = node["judgements"].as_array().is_some_and(|all| {
                        all.iter().any(|j| {
                            j["rule"] == format!("gsd/{}", rule.code).as_str()
                                && j["state"] == "fresh"
                                && j["passed"] == true
                        })
                    });
                    if !skipped && !judged {
                        let id = node["id"].as_str().expect("an id").to_string();
                        out.push((format!("gsd/{}", rule.code), id));
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// The clean-report invariant: `specforge check` finds nothing but the judged rules' `no verdict` on exactly the
    /// entities they target (see [`Project::expected_judged`]). Failable both ways: a missing judged diagnostic, an
    /// extra one, and any other diagnostic fail it.
    pub fn check_exact(&self, off: &[&str]) {
        let run = self.run(&["check"]);
        let report = self.report_all();
        assert_eq!(unjudged(&report), Vec::new(), "{report:#?}");
        let found = judged(&report);
        let expected = self.expected_judged(off);
        assert_eq!(found, expected, "{report:#?}");
        // `check` exits 1 exactly when a judged rule that is an Error (under gsd-compat) reports.
        let errors = expected.iter().any(|(code, _)| {
            let local = code.strip_prefix("gsd/").unwrap_or(code);
            gsd_core::codes::CODES
                .iter()
                .any(|k| k.id == local && k.severity == gsd_core::codes::Severity::Error)
        });
        assert_eq!(run.code, i32::from(errors), "check: {}", run.all());
        assert!(
            report
                .iter()
                .filter(|(code, _)| is_judged(code))
                .all(|(_, message)| message.ends_with(": no verdict")),
            "{report:#?}"
        );
    }

    /// The project as it was before its verifier ran: every `verdict gsd/E501` block of `spec/` and its ledger line
    /// removed. The taskboard twin carries those verdicts; a test that needs phase 01 unverified strips them.
    pub fn strip_verdicts(&self) {
        let dir = self.root.join("spec");
        for entry in std::fs::read_dir(&dir).expect("the spec dir") {
            let path = entry.expect("an entry").path();
            let text = std::fs::read_to_string(&path).expect("a spec file");
            let mut kept = Vec::new();
            let mut skipping: Option<String> = None;
            for line in text.lines() {
                match &skipping {
                    Some(close) if line == close => skipping = None,
                    Some(_) => {}
                    None if line.trim_start().starts_with("verdict gsd/E501 ") => {
                        let indent = line.len() - line.trim_start().len();
                        skipping = Some(format!("{}}}", " ".repeat(indent)));
                    }
                    None => kept.push(line),
                }
            }
            std::fs::write(&path, format!("{}\n", kept.join("\n"))).expect("a spec file");
        }
        let ledger = self.read("specforge-harness");
        let kept: Vec<&str> = ledger
            .lines()
            .filter(|l| !l.contains(" verdict:gsd/E501 "))
            .collect();
        self.write("specforge-harness", &format!("{}\n", kept.join("\n")));
    }

    /// `specforge query <entity> --inputs gsd/<code> --format json`: what the judge reads, and the basis to record.
    pub fn inputs(&self, entity: &str, code: &str) -> serde_json::Value {
        let run = self.run(&[
            "query",
            entity,
            "--inputs",
            &format!("gsd/{code}"),
            "--format",
            "json",
        ]);
        serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("inputs are JSON ({e}): {}", run.all()))
    }

    /// Plays the judge: reads the inputs of `gsd/<code>` on `entity`, then records `outcome` over their basis with
    /// `specforge gsd record-verdict`. `extra` are further options (`--findings`, `--cited`, `--values`).
    pub fn record(&self, entity: &str, code: &str, outcome: &str, extra: &[&str]) -> Run {
        let basis = self.inputs(entity, code)["basis"].to_string();
        self.record_over(entity, code, outcome, &basis, extra)
    }

    /// Plays the verifier on every item of `phase` (`v1.0/01`): a `verified` verdict on each truth, artifact and key
    /// link that `gsd/E501` reports. Answers how many it recorded.
    pub fn verify_phase(&self, phase: &str) -> usize {
        let items: Vec<String> = judged(&self.report_all())
            .into_iter()
            .filter(|(code, id)| code == "gsd/E501" && id.starts_with(&format!("{phase}/")))
            .map(|(_, id)| id)
            .collect();
        for id in &items {
            let run = self.record(id, "E501", "verified", &[]);
            assert_eq!(run.code, 0, "{id}: {}", run.all());
        }
        items.len()
    }

    /// [`Project::record`] over a `basis` the caller read earlier.
    pub fn record_over(
        &self,
        entity: &str,
        code: &str,
        outcome: &str,
        basis: &str,
        extra: &[&str],
    ) -> Run {
        let mut args = vec![
            "gsd",
            "record-verdict",
            entity,
            code,
            outcome,
            "test-model",
            basis,
        ];
        args.extend_from_slice(extra);
        self.run(&args)
    }
}

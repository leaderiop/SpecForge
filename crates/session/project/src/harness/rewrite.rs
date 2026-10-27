//! What every host write and `specforge harness gc` do to the ledger (ADR 0287): the one function,
//! [`Ledger::rewritten`].

use std::collections::BTreeMap;

use specforge_graph::{FieldForms, Graph};

use super::Harness;
use super::format::{Ledger, Line, digest};

/// A project as the ledger reads it: its graph, the forms that resolve its values, the harness fields, and whether a
/// line whose entity is missing is kept (a source did not parse).
pub struct View<'a> {
    pub graph: &'a Graph,
    pub forms: &'a FieldForms,
    pub harness: &'a Harness,
    pub keeps_gone: bool,
}

impl View<'_> {
    fn value(&self, path: &str, field: &str) -> Option<Vec<u8>> {
        self.harness.value(self.graph, self.forms, path, field)
    }
}

/// What a host write does to the ledger beyond keeping it true to the project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerChange {
    /// The harness values the write sets: `(path, field)` in the project after it. Each is recorded by exactly one
    /// line.
    pub set: Vec<(String, String)>,
    /// The entities the write moves: `(old path, new path)`, each with the entities under it.
    pub moves: Vec<(String, String)>,
}

impl Ledger {
    /// The ledger after a write that took the project from `before` to `after`:
    ///
    /// 1. a value `change` sets has exactly one line, with its new digest;
    /// 2. a move re-keys every line of the entity and of those under it that matched its value before, adds the
    ///    record `new moved:old`, and carries an existing record `old moved:x` to `new moved:x`;
    /// 3. a value the write respelled (a reference whose target moved) is re-recorded when it matched before, so a
    ///    value recorded before the write stays recorded after it;
    /// 4. a line whose entity or field is gone in `after` is dropped, unless the project has a source that did not
    ///    parse; a move record is dropped only when its new entity is gone;
    /// 5. a key with a line that matches keeps only that line;
    /// 6. the lines are sorted and unique.
    pub fn rewritten(&self, before: &View, after: &View, change: &LedgerChange) -> Ledger {
        let relocate = |path: &str| -> String {
            for (old, new) in &change.moves {
                if path == old {
                    return new.clone();
                }
                if let Some(rest) = path
                    .strip_prefix(old.as_str())
                    .and_then(|r| r.strip_prefix('/'))
                {
                    return format!("{new}/{rest}");
                }
            }
            path.to_string()
        };
        let mut lines: Vec<Line> = Vec::new();
        for line in self.lines() {
            let path = relocate(&line.path);
            if let Some(from) = line.moved_from() {
                // `old moved:x` is now `new moved:x`.
                lines.push(Line::moved(&path, from));
            } else if !line.is_field() {
                lines.push(line.clone());
            } else {
                let matched = before
                    .value(&line.path, &line.key)
                    .is_some_and(|v| digest(&line.path, &line.key, &v) == line.digest);
                match (matched, after.value(&path, &line.key)) {
                    (true, Some(v)) => lines.push(Line {
                        digest: digest(&path, &line.key, &v),
                        path,
                        key: line.key.clone(),
                    }),
                    // Not recorded before, or no longer written: the line stands as it was.
                    _ => lines.push(line.clone()),
                }
            }
        }
        for (old, new) in &change.moves {
            lines.push(Line::moved(new, old));
        }
        for (path, field) in &change.set {
            lines.retain(|l| !(l.path == *path && l.key == *field));
            if let Some(value) = after.value(path, field) {
                lines.push(Line {
                    path: path.clone(),
                    key: field.clone(),
                    digest: digest(path, field, &value),
                });
            }
        }
        // Drop what the project no longer holds.
        lines.retain(|l| {
            if after.keeps_gone {
                return true;
            }
            match l.moved_from() {
                Some(_) => after.graph.node(&l.path).is_some(),
                None if !l.is_field() => true,
                None => after.value(&l.path, &l.key).is_some(),
            }
        });
        // A key with a matching line keeps only that line.
        let mut by_key: BTreeMap<(String, String), Vec<Line>> = BTreeMap::new();
        for line in lines {
            by_key
                .entry((line.path.clone(), line.key.clone()))
                .or_default()
                .push(line);
        }
        let mut out = Ledger::default();
        for ((path, key), group) in by_key {
            let current = after.value(&path, &key).map(|v| digest(&path, &key, &v));
            let kept: Vec<Line> = match current {
                Some(current) if group.iter().any(|l| l.digest == current) => {
                    group.into_iter().filter(|l| l.digest == current).collect()
                }
                _ => group,
            };
            kept.into_iter().for_each(|l| out.insert(l));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use proptest::prelude::*;
    use specforge_graph::{
        FieldForm, FieldForms, GraphConfig, GraphInputs, build_graph_with_config,
    };
    use specforge_parser::parse;
    use specforge_protocol_types::FieldType;

    use super::*;
    use crate::harness::format::digest;

    fn line(path: u8, key: u8, seed: u8) -> Line {
        let path = format!("v1.0/0{}", path % 4);
        let key = ["a", "b", "moved:v1.0/09"][usize::from(key % 3)].to_string();
        Line {
            digest: digest(&path, &key, &[seed % 3]),
            path,
            key,
        }
    }

    fn ledger(lines: &[(u8, u8, u8)]) -> Ledger {
        let mut ledger = Ledger::default();
        lines
            .iter()
            .for_each(|&(p, k, s)| ledger.insert(line(p, k, s)));
        ledger
    }

    proptest! {
        #[test]
        fn canonical_is_idempotent(lines in prop::collection::vec((0u8..8, 0u8..8, 0u8..8), 0..12)) {
            let text = ledger(&lines).canonical();
            let parsed = Ledger::parse(&text);
            prop_assert_eq!(parsed.ledger.canonical(), text);
        }

        #[test]
        fn parse_after_write_is_the_identity(lines in prop::collection::vec((0u8..8, 0u8..8, 0u8..8), 0..12)) {
            let written = ledger(&lines);
            let parsed = Ledger::parse(&written.canonical());
            prop_assert_eq!(parsed.ledger, written);
            prop_assert!(parsed.malformed.is_empty() && !parsed.uncanonical);
        }

        #[test]
        fn a_union_of_two_canonical_ledgers_canonicalizes_to_their_sorted_union(
            a in prop::collection::vec((0u8..8, 0u8..8, 0u8..8), 0..8),
            b in prop::collection::vec((0u8..8, 0u8..8, 0u8..8), 0..8),
        ) {
            let (a, b) = (ledger(&a), ledger(&b));
            // `merge=union` keeps every line of both sides, headers included.
            let merged = Ledger::parse(&format!("{}{}", a.canonical(), b.canonical()));
            let mut union = a.clone();
            b.lines().for_each(|l| union.insert(l.clone()));
            prop_assert!(merged.malformed.is_empty());
            prop_assert_eq!(merged.ledger.canonical(), union.canonical());
        }
    }

    fn forms() -> FieldForms {
        let reference =
            |kind: &str| FieldForm::new(FieldType::Reference, Vec::new()).targeting(kind);
        FieldForms::new(
            [
                (("phase", "milestone"), reference("milestone")),
                (("uat", "phase"), reference("phase")),
                (
                    ("uat", "result"),
                    FieldForm::new(FieldType::String, Vec::new()),
                ),
            ]
            .into_iter()
            .map(|((kind, field), form)| ((kind.to_string(), field.to_string()), form))
            .collect(),
            BTreeSet::new(),
            ["milestone", "phase", "uat"].map(String::from).into(),
        )
    }

    fn graph(source: &str) -> specforge_graph::Graph {
        let config = GraphConfig {
            inputs: GraphInputs {
                field_forms: forms(),
                scopes: BTreeMap::from([
                    ("phase".to_string(), "milestone".to_string()),
                    ("uat".to_string(), "phase".to_string()),
                ]),
                ..Default::default()
            },
            ..Default::default()
        };
        build_graph_with_config(&[parse(source, "a.spec")], &config).0
    }

    fn source(phase: &str, result: &str) -> String {
        format!(
            "milestone v1.0 \"M\" {{}}\nphase {phase} \"P\" {{\n  milestone v1.0\n}}\nuat U1 \"U\" {{\n  phase {phase}\n  result {result}\n}}\n"
        )
    }

    #[test]
    fn a_move_rekeys_the_lines_under_it_and_records_the_move() {
        let harness = Harness(BTreeMap::from([(
            "uat".to_string(),
            BTreeSet::from(["result".to_string()]),
        )]));
        let forms = forms();
        let (before, after) = (graph(&source("03", "pass")), graph(&source("04", "pass")));
        let view = |graph| View {
            graph,
            forms: &forms,
            harness: &harness,
            keeps_gone: false,
        };
        let mut ledger = Ledger::default();
        let recorded = digest("v1.0/03/U1", "result", b"\"pass\"");
        ledger.insert(Line {
            path: "v1.0/03/U1".into(),
            key: "result".into(),
            digest: recorded,
        });
        let change = LedgerChange {
            set: Vec::new(),
            moves: vec![("v1.0/03".into(), "v1.0/04".into())],
        };
        let after_ledger = ledger.rewritten(&view(&before), &view(&after), &change);
        assert!(after_ledger.records(
            "v1.0/04/U1",
            "result",
            &digest("v1.0/04/U1", "result", b"\"pass\"")
        ));
        assert!(after_ledger.records(
            "v1.0/04",
            "moved:v1.0/03",
            &digest("v1.0/04", "moved:v1.0/03", b"")
        ));
        assert_eq!(
            after_ledger.lines().count(),
            2,
            "{}",
            after_ledger.canonical()
        );
    }
}

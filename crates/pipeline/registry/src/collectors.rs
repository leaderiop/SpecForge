//! The order a collect runs its collectors in (ADR 0345 D3): a collector whose `facts` input names a kind another
//! answers runs after it. The registry build reports a cycle over the loaded collectors (E119), and `collect` refuses
//! one over its selection before anything runs.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::{CollectorDescriptor, ExtensionDeclaration, Input, Inputs};

/// What the order reads of one collector: the kinds it answers, and its inputs, whose `facts` input names the kinds it
/// reads.
#[derive(Debug, Clone, Copy)]
pub struct Edges<'a> {
    pub answers: &'a [String],
    pub inputs: &'a [Input],
}

impl<'a> From<&'a CollectorDescriptor> for Edges<'a> {
    fn from(collector: &'a CollectorDescriptor) -> Self {
        Edges {
            answers: &collector.answers,
            inputs: &collector.inputs,
        }
    }
}

/// A stable topological order of `collectors`, as indices into it: on the edges producer to consumer, where the
/// producer `answers` a kind the consumer's `facts` input names, and declaration order between collectors no edge
/// orders. A collector that reads a kind it answers itself is not ordered by it: it is served that kind from the
/// recorded stores. Err: the indices of the collectors on a cycle, ascending.
pub fn order(collectors: &[Edges<'_>]) -> Result<Vec<usize>, Vec<usize>> {
    let producers = |consumer: usize| -> BTreeSet<usize> {
        let reads = Inputs(collectors[consumer].inputs).facts();
        (0..collectors.len())
            .filter(|&producer| producer != consumer)
            .filter(|&producer| {
                collectors[producer]
                    .answers
                    .iter()
                    .any(|kind| reads.contains(&kind.as_str()))
            })
            .collect()
    };
    let waits: Vec<BTreeSet<usize>> = (0..collectors.len()).map(producers).collect();
    let mut ordered: Vec<usize> = Vec::with_capacity(collectors.len());
    let mut done = vec![false; collectors.len()];
    while let Some(next) =
        (0..collectors.len()).find(|&at| !done[at] && waits[at].iter().all(|&p| done[p]))
    {
        done[next] = true;
        ordered.push(next);
    }
    if ordered.len() == collectors.len() {
        return Ok(ordered);
    }
    let edges: BTreeMap<usize, BTreeSet<usize>> = (0..collectors.len())
        .map(|consumer| (consumer, waits[consumer].clone()))
        .collect();
    let mut on_cycle: Vec<usize> = specforge_cycles::cycles(&edges)
        .into_iter()
        .flat_map(|cycle| cycle.members)
        .collect();
    on_cycle.sort_unstable();
    Err(on_cycle)
}

/// E119 for `names`, the collectors of a cycle.
pub fn cycle(names: &[String]) -> Diagnostic {
    Diagnostic::new(
        codes::E119,
        format!(
            "collectors {} read one another's facts: their facts inputs and answers form a cycle, so none of them \
             can run first",
            names
                .iter()
                .map(|name| format!("'{name}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
    .with_suggestion("drop a kind from one collector's facts input, or from what another answers")
}

/// E119 for a cycle among the collectors `declarations` declare, in load order.
pub(crate) fn check(declarations: &[ExtensionDeclaration]) -> Option<Diagnostic> {
    let collectors: Vec<&CollectorDescriptor> =
        declarations.iter().flat_map(|d| &d.collectors).collect();
    let edges: Vec<Edges<'_>> = collectors.iter().map(|&c| Edges::from(c)).collect();
    order(&edges).err().map(|on_cycle| {
        let names: Vec<String> = on_cycle
            .into_iter()
            .map(|at| collectors[at].name.clone())
            .collect();
        cycle(&names)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collector(name: &str, reads: &[&str], answers: &[&str]) -> CollectorDescriptor {
        CollectorDescriptor {
            name: name.to_string(),
            input_formats: Vec::new(),
            export: format!("collect__{name}"),
            auto_detect: None,
            capability: None,
            report: None,
            capture: None,
            writes: false,
            inputs: if reads.is_empty() {
                Vec::new()
            } else {
                vec![Input::facts(reads)]
            },
            answers: answers.iter().map(ToString::to_string).collect(),
            install: None,
        }
    }

    fn of(collectors: &[CollectorDescriptor]) -> Result<Vec<usize>, Vec<String>> {
        let edges: Vec<Edges<'_>> = collectors.iter().map(Edges::from).collect();
        order(&edges).map_err(|on_cycle| {
            on_cycle
                .into_iter()
                .map(|at| collectors[at].name.clone())
                .collect()
        })
    }

    #[specforge_test_macros::test(
        behavior = "auto_detect_collector",
        verify = "collectors run in declaration order unless a facts input orders a producer first, and a cycle is E119"
    )]
    fn the_order_is_stable_and_a_cycle_names_its_collectors() {
        let items = collector("items", &["package"], &["item"]);
        let packages = collector("packages", &[], &["package"]);
        let tests = collector("tests", &[], &["test"]);
        let own = collector("own", &["test"], &["test"]);
        assert_eq!(
            of(&[items.clone(), tests.clone(), packages.clone()]),
            Ok(vec![1, 2, 0])
        );
        assert_eq!(
            of(&[packages.clone(), items.clone(), tests]),
            Ok(vec![0, 1, 2])
        );
        // Reading what it answers itself orders nothing.
        assert_eq!(of(&[own]), Ok(vec![0]));
        let back = collector("back", &["item"], &["package"]);
        let after = collector("after", &["item"], &[]);
        assert_eq!(
            of(&[after, items, packages, back]),
            Err(vec!["items".to_string(), "back".to_string()])
        );
    }
}

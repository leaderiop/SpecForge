//! The compute pass: every `C` field of the model is listed in `computes`, as the kernel needs a compute pass for
//! each computed field it declares (W021 otherwise). It answers `context.digest`; the other computed fields stay
//! absent until the round that derives them (r38 for the roadmap's fields and `context.status`).

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    let computes: Vec<String> = crate::kinds::computed()
        .into_iter()
        .filter(|field| !super::CLAIMED.contains(&field.as_str()))
        .collect();
    c.pass("derive", |p| {
        let names: Vec<&str> = computes.iter().map(String::as_str).collect();
        p.compute().computes(&names).run(derive);
    });
}

fn derive(input: &PassInput) -> PassOutput {
    PassOutput {
        values: input
            .entities
            .iter()
            .filter(|e| e.kind == "context")
            .filter_map(|e| {
                Some(PassValue {
                    entity: e.id.clone(),
                    field: "digest".into(),
                    value: crate::digest::context(input, &e.id)?.into(),
                })
            })
            .collect(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derive_pass_gives_each_context_its_digest() {
        let entity = |id: &str, kind: &str| PassEntity {
            id: id.into(),
            kind: kind.into(),
            ..Default::default()
        };
        let input = PassInput {
            entities: vec![
                entity("v1.0/01/01-CONTEXT", "context"),
                entity("v1.0/02/02-CONTEXT", "context"),
                entity("v1.0/02/02-RESEARCH", "research"),
            ],
            ..Default::default()
        };
        let output = derive(&input);
        let given: Vec<(&str, &str)> = output
            .values
            .iter()
            .map(|v| (v.entity.as_str(), v.field.as_str()))
            .collect();
        assert_eq!(
            given,
            [
                ("v1.0/01/01-CONTEXT", "digest"),
                ("v1.0/02/02-CONTEXT", "digest")
            ]
        );
        assert_ne!(output.values[0].value, output.values[1].value);
    }
}

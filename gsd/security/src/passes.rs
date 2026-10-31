//! The passes of `@gsd/security`: the compute pass for the model's computed fields.

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    let computes = crate::kinds::computed();
    c.pass("security", |p| {
        let names: Vec<&str> = computes.iter().map(String::as_str).collect();
        p.compute().computes(&names).run(compute);
    });
}

fn compute(_input: &PassInput) -> PassOutput {
    PassOutput::default()
}

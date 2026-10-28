//! The compute pass: every `C` field of the model is listed in `computes`, as the kernel needs a compute pass for
//! each computed field it declares (W021 otherwise). It answers no value yet, so a computed field is absent until the
//! round that derives it (r38 for the roadmap's fields).

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    let computes = crate::kinds::computed();
    c.pass("derive", |p| {
        let names: Vec<&str> = computes.iter().map(String::as_str).collect();
        p.compute()
            .computes(&names)
            .run(|_: &PassInput| PassOutput::default());
    });
}

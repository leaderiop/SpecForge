//! The kinds of `@gsd/core`, as tables (`dsl`): one file per area of the model.

mod dsl;
mod project;

use dsl::Kind;
use specforge_extension_sdk::prelude::*;

/// Every kind declared so far, in the order of the model's files.
const KINDS: &[&Kind] = &[&project::PROJECT, &project::REQUIREMENT];

pub fn declare(c: &mut ContributionsBuilder) {
    dsl::declare(c, KINDS);
}

/// `kind.field` of every computed field: what the derive pass answers.
pub fn computed() -> Vec<String> {
    dsl::computed(KINDS)
}

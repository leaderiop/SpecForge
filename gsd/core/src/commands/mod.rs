//! The commands of `@gsd/core` (K5): the writing commands the workflows call. r40 builds `record_verdict`; r41
//! builds the others.

mod verdict;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    verdict::declare(c);
}

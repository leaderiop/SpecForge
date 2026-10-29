//! The commands of `@gsd/core` (K5): the writing commands the workflows call. r40 builds `record_verdict`; r41
//! builds the others, each file the commands of one family (D-r41-2).

mod execute;
mod support;
mod verdict;
mod verify;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    verdict::declare(c);
    execute::declare(c);
    verify::declare(c);
}

//! The passes of `@gsd/core`.

mod derive;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    derive::declare(c);
}

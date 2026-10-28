//! The passes of `@gsd/core`.

mod derive;
mod roadmap;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    derive::declare(c);
    roadmap::declare(c);
}

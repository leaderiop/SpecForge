//! The passes of `@gsd/core`.

mod derive;
mod drift;
mod research;
mod roadmap;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    derive::declare(c);
    roadmap::declare(c);
    research::declare(c);
    drift::declare(c);
}

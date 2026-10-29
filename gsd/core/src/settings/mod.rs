//! The typed settings of `@gsd/core` (K6): GSD's `.planning/config.json` keys as `specforge.json` settings.

mod gates;
mod keys;
pub mod models;
mod profiles;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    for key in keys::KEYS {
        c.setting(key.name, |s| {
            key.apply(s);
            if let Some(codes) = gates::of(key.name) {
                s.gates(codes);
            }
        });
    }
    for family in keys::FAMILIES {
        c.setting(family.name, |s| family.apply(s));
    }
    gates::declare(c);
    profiles::declare(c);
    models::declare(c);
}

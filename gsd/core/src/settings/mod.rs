//! The typed settings of `@gsd/core` (K6): GSD's `.planning/config.json` keys as `specforge.json` settings.

mod keys;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    for key in keys::KEYS {
        c.setting(key.name, |s| key.apply(s));
    }
    for family in keys::FAMILIES {
        c.setting(family.name, |s| family.apply(s));
    }
}

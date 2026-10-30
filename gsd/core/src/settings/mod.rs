//! The typed settings of `@gsd/core` (K6): GSD's `.planning/config.json` keys as `specforge.json` settings.
//!
//! Also ports gsd-core bin/lib modules: config-loader.cjs, config-types.cjs, configuration.cjs,
//! federated-config.cjs.

mod gates;
mod keys;
pub mod models;
mod profiles;
pub mod surface;

use specforge_extension_sdk::prelude::*;

/// The settings a workflow reads in place of `config-get`: every key of the roster that has a value, as a JSON object
/// (the host's value, else the declared default), in the roster's order.
pub fn block() -> String {
    let current = Settings::current();
    block_of(|name| current.get(name).cloned())
}

/// [`block`] over any source of values.
pub fn block_of(value: impl Fn(&str) -> Option<serde_json::Value>) -> String {
    let rows: Vec<String> = keys::KEYS
        .iter()
        .filter_map(|key| Some(format!("  \"{}\": {}", key.name, value(key.name)?)))
        .collect();
    format!("{{\n{}\n}}", rows.join(",\n"))
}

/// The names of the roster's keys.
pub fn key_names() -> impl Iterator<Item = &'static str> {
    keys::KEYS.iter().map(|key| key.name)
}

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
    surface::declare(c);
}

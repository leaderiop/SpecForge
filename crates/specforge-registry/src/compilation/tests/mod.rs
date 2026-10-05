//! The registry's specification tests. They are unit tests so they can
//! reach the steps `build_registries` keeps private; they name the crate
//! `specforge_registry` (an alias of `crate` in test builds) as the
//! integration tests do.

pub(crate) mod support;
mod zero_entity_registries;
mod zero_entity_validation;

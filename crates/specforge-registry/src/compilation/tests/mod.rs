//! The rule engine's tests, until plan 02 moves them to tests/. They
//! reach the registries through `support::registries`, which is
//! `build_registries`; they name the crate `specforge_registry` (an alias
//! of `crate` in test builds) as the integration tests do.

pub(crate) mod support;
mod zero_entity_validation;

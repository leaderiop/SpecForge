//! The kind DSL is public API of the kit: a capability extension declares its kinds with it.
use gsd_kit::dsl::*;

const THING: Kind = Kind::new(
    "thing",
    "a kind declared outside @gsd/core",
    &[str("title").r(), int("n")],
);

#[test]
fn a_kind_is_declared_from_outside_core() {
    assert_eq!(THING.name, "thing");
    assert_eq!(THING.fields.len(), 2);
}

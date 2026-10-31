//! `gsd-kit`: what `@gsd/core` and the capability extensions share (ADR gsd/0002): the kind DSL, the prompt render
//! pipeline and the attribution of a ported agent. No guest: each extension exports its own.

pub mod agent;
pub mod capability;
pub mod dsl;
pub mod prompt;

/// The gsd-core release the `@gsd` extensions track, the one place it is written: `TRACKS`, every attribution and
/// every path into the vendored snapshot read it. `Cargo.toml`'s `tracks` repeats it, held equal by a test per crate.
#[macro_export]
macro_rules! tracked_version {
    () => {
        "1.18.0"
    };
}

/// The vendored snapshot of the tracked release (`gsd/parity/upstream/`), or a file in it, as an absolute path from
/// the calling crate: `env!` expands there, and every `@gsd` crate sits at `gsd/<crate>/`. The literal is split so
/// the snapshot-name scan of `gsd/core/tests/declaration.rs` does not read it as a snapshot.
#[macro_export]
macro_rules! snapshot {
    () => {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../parity/upstream/",
            "v",
            $crate::tracked_version!()
        )
    };
    ($path:literal) => {
        concat!($crate::snapshot!(), "/", $path)
    };
}

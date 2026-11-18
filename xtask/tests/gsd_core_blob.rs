//! The committed `@gsd/core` blob is the stager's build of its sources, on the pinned rustc.
//!
//! `build-blobs --verify --only gsd/core` rebuilds `gsd/core` in a staged workspace on `BLOB_RUSTC` and compares the
//! bytes with `gsd/core/wasm/gsd_core.wasm`, not with the hash its record holds. A blob another rustc built, or other
//! bytes, fails; nothing skips (ADR 0366 D4, ADR 0373 D2). The bootstrap job runs it after `cargo gate bootstrap`:
//! `cargo nextest run -p xtask --run-ignored only -E 'test(committed_gsd_core_blob_is_the_stagers_build)'`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::process::{Command, Stdio};

/// Proves the committed `@gsd/core` bytes are the stager's build of the sources the record names, on the pinned rustc:
/// the spawned `--verify --only gsd/core` exits 0 and prints `[same]   gsd/core`. A `[skip]`, a `[fail]` (another rustc,
/// a missing toolchain), a `[differs]` or any other exit fails it; it cannot pass by skipping (ADR 0366 D4, ADR 0373 D2).
#[test]
#[ignore = "builds @gsd/core on the pinned rustc (about a minute); the bootstrap job runs it"]
fn committed_gsd_core_blob_is_the_stagers_build() {
    let out = Command::new(env!("CARGO_BIN_EXE_build-blobs"))
        .args(["--verify", "--only", "gsd/core"])
        .stdin(Stdio::null())
        .output()
        .expect("build-blobs runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let same = stdout.lines().any(|line| line == "[same]   gsd/core");
    assert!(
        out.status.success() && same,
        "the committed gsd/core blob is not the stager's build (exit {:?}).\n\
         Fix: install the pinned rustc (`rustup toolchain install <BLOB_RUSTC> --target wasm32-wasip2`, the \
         `[fail]` line names the command), then run `cargo run -p xtask --bin revendor` and commit what it \
         prints (docs/agents/revendor.md).\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        out.status.code()
    );
}

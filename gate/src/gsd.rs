//! The `gsd` check: the `@gsd` workspace (`gsd/`) lints, builds both extensions (`@gsd/core`, `@gsd/security`) to
//! `wasm32-wasip2` and passes its tests against the `specforge` binary (ADR gsd/0001 D6, ADR gsd/0002). The workspace
//! patches the in-tree SDK, so a kernel change that breaks an `@gsd` extension fails in the pull request that made it.

use crate::checks::Selection;
use crate::run::Ctx;

/// What can break the workspace: itself, the root manifest and lockfile (a version bump must reach
/// `gsd/core/Cargo.toml`) and every crate: the tests run through the `specforge` binary, which is built from all of
/// them, and the SDK is one of them.
pub const TRIGGER: &[&str] = &["gsd/**", "crates/**", "Cargo.toml", "Cargo.lock"];

/// Stops at the first failing step: clippy for native and wasm, the release wasm build, the `specforge` CLI build,
/// then `cargo nextest run` (every test binary at once; `cargo test` runs them one after another) with
/// `SPECFORGE_BIN`, `GSD_CORE_WASM` and `GSD_SECURITY_WASM` set to the three artifacts. Cargo runs in `gsd/`: it reads
/// `.cargo/config.toml` (the SDK patch) from its working directory, not from `--manifest-path`.
pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let target = ctx.repo.target_dir.join("gsd");
    let wasm = |name: &str| target.join("wasm32-wasip2/release").join(name);
    let cargo = |args: &[&str]| {
        let mut cmd = ctx.cmd("cargo");
        cmd.current_dir(ctx.repo.root.join("gsd"))
            .env("CARGO_TARGET_DIR", &target)
            .args(args);
        cmd
    };
    ctx.sh(&mut cargo(&[
        "clippy",
        "--locked",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]))?;
    ctx.sh(&mut cargo(&[
        "clippy",
        "--locked",
        "--target",
        "wasm32-wasip2",
        "--lib",
        "--",
        "-D",
        "warnings",
    ]))?;
    ctx.sh(&mut cargo(&[
        "build",
        "--locked",
        "-p",
        "gsd-core",
        "-p",
        "gsd-security",
        "--release",
        "--target",
        "wasm32-wasip2",
    ]))?;
    let specforge = ctx.specforge()?;
    ctx.sh(cargo(&["nextest", "run", "--locked", "--no-fail-fast"])
        .env("SPECFORGE_BIN", specforge)
        .env("GSD_CORE_WASM", wasm("gsd_core.wasm"))
        .env("GSD_SECURITY_WASM", wasm("gsd_security.wasm")))
}

#[cfg(test)]
mod tests {
    use crate::change_set::ChangeSet;
    use crate::checks::{CHECKS, Selection};
    use crate::repo::this_repo;
    use crate::run::Mode;

    fn selection(paths: &[&str]) -> Selection {
        let check = CHECKS
            .iter()
            .find(|c| c.name == "gsd")
            .expect("the list has a gsd check");
        check
            .selection(
                Mode::Quick,
                &this_repo(),
                &ChangeSet::of(paths.iter().copied()),
            )
            .expect("a path trigger cannot fail")
    }

    #[test]
    fn the_gsd_check_triggers_on_what_the_binary_is_built_from_and_on_gsd() {
        for path in [
            "gsd/core/src/lib.rs",
            "crates/runtime/extension-sdk/src/lib.rs",
            "crates/leaf/protocol-types/src/calls.rs",
            "crates/pipeline/formatter/src/lib.rs",
            "crates/session/src/lib.rs",
            "crates/runtime/component/src/lib.rs",
            "crates/app/ops/src/lib.rs",
            "crates/surfaces/cli/src/main.rs",
            "Cargo.toml",
        ] {
            assert_eq!(selection(&[path]), Selection::Everything, "{path}");
        }
        assert_eq!(selection(&["docs/guide.md"]), Selection::Skip);
    }
}

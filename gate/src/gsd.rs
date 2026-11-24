//! The `gsd` check: the `@gsd` workspace (`gsd/`) lints, builds `@gsd/security` and `@gsd/import` to `wasm32-wasip2`
//! and passes its tests against the `specforge` binary (ADR gsd/0001 D6, ADR gsd/0002). The workspace patches the
//! in-tree SDK, so a kernel change that breaks an `@gsd` extension fails in the pull request that made it.
//!
//! Who builds what (ADR 0373 D5): `@gsd/core` is a builtin the CLI embeds, vendored by `revendor` into
//! `gsd/core/wasm/`, and nothing builds its wasm to load it; the suite loads it by name. `@gsd/security` is not
//! vendored: this step builds its wasm from `gsd/` and the suite reads it through `GSD_SECURITY_WASM`. `@gsd/import`
//! is built here too, and read through `GSD_IMPORT_WASM`, until its blob is vendored (ADR 0391 D12).

use std::path::PathBuf;
use std::process::Command;

use crate::checks::Selection;
use crate::run::Ctx;

/// What can break the workspace: itself, the root manifest and lockfile (a version bump must reach
/// `gsd/core/Cargo.toml`), every crate and every builtin's committed blob: the tests run through the `specforge`
/// binary, which is built from all the crates and embeds the blobs, and the SDK is one of the crates. A builtin's
/// sources are the `builtins` check's: a source change without a re-vendor fails there, so only the blob triggers.
/// The GSD A/B protocol's files too: the suite seeds its tasks with `seed.py` and runs both scripts' self-tests.
pub const TRIGGER: &[&str] = &[
    "gsd/**",
    "evaluation/gsd-ab/**",
    "crates/**",
    "Cargo.toml",
    "Cargo.lock",
    "extensions/**/*.wasm",
];

/// Stops at the first failing step: clippy for native and wasm, the release builds of `@gsd/security`'s and
/// `@gsd/import`'s wasm (`@gsd/core` is the builtin the CLI embeds), the `specforge` CLI build, then
/// `cargo nextest run` (every test binary at once; `cargo test` runs them one after another) with `SPECFORGE_BIN`,
/// `GSD_SECURITY_WASM` and `GSD_IMPORT_WASM` set to the three artifacts. Cargo runs in `gsd/`: it reads
/// `.cargo/config.toml` (the SDK patch) from its working directory, not from `--manifest-path`.
pub fn run(ctx: &Ctx, _: &Selection) -> Result<(), String> {
    let cargo = |args: &[&str]| cargo(ctx, args);
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
    let security = build_wasm(ctx, "gsd-security")?;
    let import = build_wasm(ctx, "gsd-import")?;
    let specforge = ctx.specforge()?;
    ctx.sh(cargo(&["nextest", "run", "--locked", "--no-fail-fast"])
        .env("SPECFORGE_BIN", specforge)
        .env("GSD_SECURITY_WASM", security)
        .env("GSD_IMPORT_WASM", import))
}

/// Build `package` for `wasm32-wasip2` in release, a component the suite reads by path rather than as a vendored
/// builtin (ADR 0373 D5); answers its path, `<package with - as _>.wasm`.
fn build_wasm(ctx: &Ctx, package: &str) -> Result<PathBuf, String> {
    ctx.sh(&mut cargo(
        ctx,
        &[
            "build",
            "--locked",
            "-p",
            package,
            "--release",
            "--target",
            "wasm32-wasip2",
        ],
    ))?;
    Ok(target(ctx).join(format!(
        "wasm32-wasip2/release/{}.wasm",
        package.replace('-', "_")
    )))
}

/// `<target_dir>/gsd`: the workspace's own target directory.
fn target(ctx: &Ctx) -> PathBuf {
    ctx.repo.target_dir.join("gsd")
}

/// Cargo in `gsd/`, on the workspace's target directory.
fn cargo(ctx: &Ctx, args: &[&str]) -> Command {
    let mut cmd = ctx.cmd("cargo");
    // An incremental guest is another blob than CI's, and the suite's work budget was calibrated on CI's.
    cmd.current_dir(ctx.repo.root.join("gsd"))
        .env("CARGO_TARGET_DIR", target(ctx))
        .env_remove("CARGO_INCREMENTAL")
        .args(args);
    cmd
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
            "extensions/software/wasm/specforge_ext_software.wasm",
            "extensions/code/core/wasm/x.wasm",
            "evaluation/gsd-ab/tasks.json",
            "evaluation/gsd-ab/seed.py",
        ] {
            assert_eq!(selection(&[path]), Selection::Everything, "{path}");
        }
        assert_eq!(selection(&["docs/guide.md"]), Selection::Skip);
        assert_eq!(selection(&["evaluation/tasks.json"]), Selection::Skip);
        assert_eq!(
            selection(&["extensions/software/src/lib.rs"]),
            Selection::Skip,
            "a builtin's sources are the builtins check's"
        );
    }
}

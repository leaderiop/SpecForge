//! Gives the binary the commit it is built from, `SPECFORGE_COMMIT` (empty when unknown), for `--version`.

#[path = "build/commit.rs"]
mod commit;

fn main() {
    println!("cargo:rerun-if-env-changed=SPECFORGE_COMMIT");
    // CI and a source tarball (no `.git`) say it; else the checkout's `.git` files do.
    let told = std::env::var("SPECFORGE_COMMIT").unwrap_or_default();
    let told = told.trim();
    let short = if told.is_empty() {
        // The workspace root holds `.git`: crates/surfaces/cli is three levels below it. Canonical, so each watched
        // path is absolute and outside the package: cargo keeps a path under the package relative to it, and a copy
        // of the workspace without `.git` (the gate's sample) would find it missing and rebuild the binary.
        let dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&dir).join("../../..");
        let found = commit::of(&root.canonicalize().unwrap_or(root));
        for path in &found.watched {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        found.short.unwrap_or_default()
    } else {
        told.get(..9).unwrap_or(told).to_string()
    };
    println!("cargo:rustc-env=SPECFORGE_COMMIT={short}");
}

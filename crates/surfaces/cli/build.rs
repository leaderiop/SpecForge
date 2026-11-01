//! Gives the binary the commit it is built from, `SPECFORGE_COMMIT` (empty when unknown), for `--version`.

#[path = "build/commit.rs"]
mod commit;

fn main() {
    println!("cargo:rerun-if-env-changed=SPECFORGE_COMMIT");
    // CI and a source tarball (no `.git`) say it; else the checkout's `.git` files do.
    let told = std::env::var("SPECFORGE_COMMIT").unwrap_or_default();
    let told = told.trim();
    let short = if told.is_empty() {
        let dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default();
        let found = commit::of(std::path::Path::new(&dir));
        for path in &found.watched {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        found.short.unwrap_or_default()
    } else {
        told.get(..9).unwrap_or(told).to_string()
    };
    println!("cargo:rustc-env=SPECFORGE_COMMIT={short}");
}

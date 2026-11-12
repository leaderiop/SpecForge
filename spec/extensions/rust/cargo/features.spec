// @rust/cargo features

use "extensions/rust/cargo/behaviors"

feature rc_package_facts "Cargo Package Facts" {
  status   in_progress
  problem  """
    A spec could cite a Rust test but not the package or target that holds
    it: nothing read a project's Cargo.toml files into facts, so a package,
    its version, its targets and the project packages it depends on were
    names no check could resolve.
  """
  solution """
    The extension library specforge-rust-manifest reads every Cargo.toml
    the host serves with cargo_toml, cargo's own manifest model, over the
    file listing: workspace inheritance and target autodiscovery as cargo
    applies them. It answers one package per manifest that declares one,
    with its targets and its path dependencies inside the project, minted
    by specforge-code-facts (ADR 0348).
  """
}

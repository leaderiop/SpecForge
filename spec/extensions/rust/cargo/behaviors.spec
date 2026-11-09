// @rust/cargo behaviors

use "extensions/code/core/features"

behavior rc_read_manifests "Read Every Cargo.toml as Cargo Reads It" {
  features [rc_package_facts, cc_code_facts]
  category computation
  contract """
    specforge-rust-manifest MUST complete each manifest it is given with
    cargo_toml over the host's file listing: fields a member inherits from
    the nearest workspace above it (or the one package.workspace names),
    the targets cargo discovers (src/lib.rs, src/main.rs, src/bin, tests,
    examples, benches, build.rs) and those the manifest declares, under
    autotests and its kin. A virtual manifest MUST be no package. A
    package's depends_on MUST name the project packages its path
    dependencies (normal, dev, build, per platform) reach, never a path
    outside the project. A manifest that does not parse or complete MUST be
    an error naming its path, never a dropped package (ADR 0348 D5).
  """
  ensures {
    inherits     "a member's inherited fields are its workspace's"
    discovers    "every target cargo builds is a target, its root project-relative"
    virtual_none "a virtual manifest is no package"
    project_deps "depends_on names only the project's packages"
    named_error  "an unreadable manifest is an error naming it"
  }
  verify unit "a member inherits its version, edition and rust-version from its workspace"
  verify unit "targets are discovered as cargo discovers them, autotests and a build script included"
  verify unit "each target has the nextest binary id its tests run under"
  verify unit "a target round-trips through the text its package fact writes"
  verify unit "path dependencies name the project packages they reach, never one outside it"
  verify unit "a manifest that does not parse or complete is an error naming it"
  verify unit "a virtual manifest is no package"
}

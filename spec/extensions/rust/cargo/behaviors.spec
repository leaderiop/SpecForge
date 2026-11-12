// @rust/cargo behaviors

use "extensions/code/core/features"
use "types/wasm"

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

behavior rc_declare_cargo_collector "Declare the rust-cargo Collector" {
  features [rc_package_facts, cc_code_facts]
  category query
  types    [CollectorDescriptor]
  contract """
    @rust/cargo MUST declare one collector, `rust-cargo`, selected by a
    Cargo.toml at the project root, with no command, no report and no
    capability: it reads every `**/Cargo.toml` with its text and, without
    text, the eleven globs cargo's target autodiscovery looks at (one read
    per glob). It answers the package kind and requires @code/core ^1.1,
    whose kind it writes (ADR 0348 D5).
  """
  ensures {
    detected_by_cargo_toml "a Cargo.toml at the project root selects rust-cargo"
    no_command             "it runs nothing, so it asks no approval"
    requires_code_core     "@code/core ^1.1 is a required peer"
  }
  verify unit "rust-cargo declares its collector, its reads and its peer"
}

behavior rc_map_manifests_to_package_facts "Map Every Manifest to Package Facts" {
  features [rc_package_facts, cc_code_facts]
  category query
  types    [CollectInput, CollectOutput, Fact]
  contract """
    `collect__rust_cargo` MUST answer one package fact per package the
    served manifests declare, minted by specforge-code-facts: the id
    pkg:cargo/<name>, or pkg:cargo/<name>#<dir> for every package but the
    shallowest of a name two workspaces share; its version, edition,
    features, targets and path; depends_on naming the project packages
    only; located at its [package] table. A manifest the host withheld, one
    cargo's reader cannot read and a read that says truncated MUST be the
    collector's error, never a dropped package (ADR 0348 D5).
  """
  ensures {
    one_fact_each "every package is one package fact"
    project_deps  "depends_on names project packages only"
    shared_name   "a shared name keeps one plain id"
    loud_failure  "a withheld, unreadable or truncated read is the collector's error"
    located       "a package fact is at its [package] table"
  }
  verify unit "every package becomes a package fact"
  verify unit "depends_on names the project packages only"
  verify unit "a name two workspaces share keeps the nearest plain and the other its directory"
  verify unit "a withheld or truncated read is the collector's error"
  verify unit "a package fact is located at its package table"
  verify integration "the rust packs run in fact order over a fixture crate"
}

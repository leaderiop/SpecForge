// Rust code generation features

use behaviors/rust-codegen

feature rust_type_and_port_generation "Rust Type and Port Code Generation" {
  behaviors [generate_rust_structs_from_types, generate_rust_traits_from_ports, slugify_verify_descriptions]

  problem """
    Rust projects using SpecForge must manually translate spec type and port
    definitions to Rust structs and traits. This duplication leads to drift
    between spec and implementation.
  """

  solution """
    specforge gen rust produces Rust structs from type blocks and traits from
    port blocks. Supports thiserror/anyhow/raw Result styles, optional async
    support, and serde derive annotations.
  """
}

feature rust_test_stub_generation "Rust Test Stub Generation and Drift Detection" {
  behaviors [generate_rust_test_stubs, generate_rust_bench_stubs, generate_rust_module_tree, detect_rust_code_drift, safe_rust_regeneration]

  problem """
    Rust developers need boilerplate test stubs for each behavior's verify
    statements and each capability's scenario blocks. Generated code must
    stay in sync with spec changes and be safe to regenerate.
  """

  solution """
    Test stub generation creates one module per entity with #[test] fns per
    verify statement. SHA256 checksum headers enable drift detection in CI.
    Safe regeneration skips files with implementations unless --force is used.
    --merge preserves existing bodies while updating signatures.
  """
}

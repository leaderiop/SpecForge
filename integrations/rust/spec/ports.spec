// specforge-test crate ports
// Boundaries between the crate and external systems.

use "types"

port ProcessExit {
  direction outbound
  category  "system/lifecycle"

  method register_atexit(handler: fn()) -> Result<(), string>
  method get_binary_name() -> Result<string, string>

  requires {
    single_registration "atexit handler is registered at most once via std::sync::Once"
  }
}

port FileSystem {
  direction outbound
  category  "io/filesystem"

  method write_report(path: string, report: BinaryReport) -> Result<(), string>
  method read_graph_export(path: string) -> Result<GraphExport, string>
  method file_exists(path: string) -> Result<boolean, string>

  requires {
    target_dir_writable "target/specforge/ directory is writable"
  }
}

port SpecforgeCli {
  direction outbound
  category  "tool/specforge"

  method export_graph(output_path: string) -> Result<(), string>

  requires {
    specforge_on_path "specforge binary is discoverable on PATH"
  }

  ensures {
    graph_json_written "output_path contains valid GraphExport JSON"
  }
}

port TestRegistry {
  direction inbound
  category  "testing/registry"

  method record(entry: TestRecordEntry) -> Result<(), string>
  method drain() -> Result<TestRecordEntry[], string>

  requires {
    thread_safe "registry operations are safe under concurrent test execution"
  }
}

port Stderr {
  direction outbound
  category  "io/output"

  method print_summary(diffs: CoverageDiff[]) -> Result<(), string>
}

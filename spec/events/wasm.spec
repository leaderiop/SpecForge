// Wasm/Extism runtime events

use types/wasm
use behaviors/wasm

event plugin_loaded "Plugin Loaded" {
  trigger   load_wasm_module
  channel   "wasm.plugin_loaded"

  payload {
    pluginName  string
    wasmSize    integer
    fromCache   boolean
    loadTimeMs  integer
  }

  consumers [initialize_wasm_plugin]

  verify integration "emits plugin_loaded with correct pluginName, wasmSize, fromCache, and loadTimeMs"
  verify integration "consumer initialize_wasm_plugin receives event and begins initialization"
}

event plugin_initialized "Plugin Initialized" {
  trigger   initialize_wasm_plugin
  channel   "wasm.plugin_initialized"

  payload {
    pluginName     string
    entityCount    integer
    edgeTypeCount  integer
  }

  consumers [call_wasm_validate]

  verify integration "emits plugin_initialized with correct entityCount and edgeTypeCount"
  verify integration "consumer call_wasm_validate receives event and begins validation"
}

event plugin_validated "Plugin Validated" {
  trigger   call_wasm_validate
  channel   "wasm.plugin_validated"

  payload {
    pluginName      string
    diagnosticCount integer
    durationMs      integer
  }

  consumers [call_wasm_generate]

  verify integration "emits plugin_validated with correct diagnosticCount and durationMs"
  verify integration "consumer call_wasm_generate receives event when generation is requested"
}

event plugin_unloaded "Plugin Unloaded" {
  trigger   warm_wasm_engine_instance
  channel   "wasm.plugin_unloaded"

  payload {
    pluginName  string
    reason      string
  }

  consumers []

  verify integration "emits plugin_unloaded with correct pluginName and reason"
}

event wasm_aot_compiled "Wasm AOT Compiled" {
  trigger   aot_compile_wasm_module
  channel   "wasm.aot_compiled"

  payload {
    pluginName    string
    wasmHash      string
    aotSize       integer
    compileTimeMs integer
  }

  consumers [cache_aot_artifacts]

  verify integration "emits wasm_aot_compiled with correct wasmHash, aotSize, and compileTimeMs"
  verify integration "consumer cache_aot_artifacts receives event and stores compiled artifact"
}

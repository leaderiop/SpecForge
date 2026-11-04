//! The way into an instance: what one call of an export is held to, and what it answers.

use std::sync::Arc;

use specforge_wasm::PortSession;
use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
use specforge_wasm::sandbox::Limits;

use crate::{ComponentRuntime, PluginInstance, ms_to_ticks};

impl ComponentRuntime {
    /// One call into `name`'s shared instance, its port session made from the extension's limits. The session lasts
    /// the call: a later pure call is denied whatever this one was lent.
    pub(crate) fn call_in_session(
        &self,
        name: &str,
        export: &str,
        input: &[u8],
        max_ms: u32,
        session: impl FnOnce(Limits) -> PortSession,
    ) -> WasmCallResult {
        // Look up the plugin and release the map lock immediately: holding
        // it across the guest call would serialize every extension behind
        // one mutex (the C7-10 finding). Only the target plugin's own lock
        // is held for the duration of the call.
        let plugin = {
            let plugins = match self.plugins.lock() {
                Ok(p) => p,
                Err(e) => {
                    return WasmCallResult::Trap(WasmTrapInfo {
                        kind: "lock_poisoned".to_string(),
                        message: e.to_string(),
                        export_name: export.to_string(),
                    });
                }
            };
            match plugins.get(name) {
                Some(plugin) => Arc::clone(plugin),
                None => {
                    return WasmCallResult::Trap(WasmTrapInfo {
                        kind: "extension_not_found".to_string(),
                        message: format!("Extension '{name}' not loaded"),
                        export_name: export.to_string(),
                    });
                }
            }
        };
        let mut instance = match plugin.lock() {
            Ok(instance) => instance,
            Err(e) => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "lock_poisoned".to_string(),
                    message: e.to_string(),
                    export_name: export.to_string(),
                });
            }
        };
        let PluginInstance {
            store,
            bindings,
            limits,
            component,
            fuel,
            ..
        } = &mut *instance;
        // Every call gets the whole fuel budget and the whole wall-clock
        // budget: a long session never spends one call's allowance on the
        // next.
        let (call_fuel, deadline_ms) = self.budget(*fuel, *limits, max_ms);
        if let Err(e) = store.set_fuel(call_fuel) {
            return WasmCallResult::Trap(WasmTrapInfo {
                kind: "call_failed".to_string(),
                message: format!("failed to set fuel for {name}: {e}"),
                export_name: export.to_string(),
            });
        }
        store.set_epoch_deadline(ms_to_ticks(u64::from(deadline_ms)));
        store.data_mut().ports = session(*limits);
        let called = bindings.call_call(&mut *store, name, export, input);
        store.data_mut().ports = PortSession::pure();
        match called {
            Ok(Ok(bytes)) => WasmCallResult::Ok(bytes),
            Ok(Err(message)) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message,
                export_name: export.to_string(),
            }),
            Err(e) => {
                // What stopped the call is read before the instance is
                // replaced: a limit of the sandbox is named, any other
                // fault keeps wasmtime's message.
                let memory_refused = store.data_mut().memory.refused.take();
                let kind = trap_kind(&e, memory_refused);
                let message = match kind {
                    "memory_limit_exceeded" => format!(
                        "grew past its {} MB memory limit (sandbox_policy.max_memory_mb)",
                        limits.memory_mb
                    ),
                    "fuel_exhausted" if call_fuel < *fuel => format!(
                        "spent the work of its {} ms execution limit ({call_fuel} instructions \
                         under a work budget; sandbox_policy.max_execution_ms)",
                        limits.execution_ms
                    ),
                    "fuel_exhausted" => {
                        format!("spent its fuel budget ({fuel} instructions) in one call")
                    }
                    "deadline_exceeded" if max_ms < limits.execution_ms => {
                        format!("ran past the call's {max_ms} ms budget")
                    }
                    "deadline_exceeded" => format!(
                        "ran past its {} ms execution limit (sandbox_policy.max_execution_ms)",
                        limits.execution_ms
                    ),
                    _ => e.to_string(),
                };
                // An instance that trapped cannot be entered again: the
                // extension's next call gets a fresh one under the same
                // limits, as a guest that panics in one call must not take
                // the extension down for the rest of the process (an MCP
                // session).
                // One that cannot start (its own limits refuse it) is
                // unloaded, so later calls say it is gone instead of failing
                // on the dead instance.
                let message = match self.fresh_instance(component, *fuel, *limits) {
                    Ok((fresh_store, fresh_bindings)) => {
                        *store = fresh_store;
                        *bindings = fresh_bindings;
                        message
                    }
                    Err(why) => {
                        // The plugin's lock goes first: `apply_limits` takes
                        // the map's, then a plugin's.
                        drop(instance);
                        if let Ok(mut plugins) = self.plugins.lock()
                            && plugins.get(name).is_some_and(|p| Arc::ptr_eq(p, &plugin))
                        {
                            plugins.remove(name);
                        }
                        format!("{message}; it could not be restarted ({why}) and is unloaded")
                    }
                };
                WasmCallResult::Trap(WasmTrapInfo {
                    kind: kind.to_string(),
                    message,
                    export_name: export.to_string(),
                })
            }
        }
    }
}

/// What stopped a call: a limit the sandbox holds it to, or another fault.
fn trap_kind(error: &wasmtime::Error, memory_refused: Option<usize>) -> &'static str {
    if memory_refused.is_some() {
        return "memory_limit_exceeded";
    }
    match error.downcast_ref::<wasmtime::Trap>() {
        Some(wasmtime::Trap::Interrupt) => "deadline_exceeded",
        Some(wasmtime::Trap::OutOfFuel) => "fuel_exhausted",
        _ => "call_failed",
    }
}

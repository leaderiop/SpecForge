//! The way into an instance: what one call of an export is held to, and what it answers.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specforge_wasm::PortSession;
use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
use specforge_wasm::sandbox::Limits;

use wasmtime::UpdateDeadline;
use wasmtime::component::Component;

use crate::{ComponentRuntime, PluginInstance, ms_to_ticks};

/// The time a guest may run, read at each epoch deadline: what has passed, less what its ports took (ADR 0037 D3).
pub(crate) struct GuestClock {
    budget: Duration,
    started: Instant,
}

impl GuestClock {
    pub(crate) fn of(deadline_ms: u32) -> Self {
        GuestClock {
            budget: Duration::from_millis(u64::from(deadline_ms)),
            started: Instant::now(),
        }
    }

    /// At an epoch deadline: trap a guest that spent its budget, or check again when what is left of it has passed.
    pub(crate) fn at_deadline(&self, in_ports: Duration) -> UpdateDeadline {
        let guest = self.started.elapsed().saturating_sub(in_ports);
        match self.budget.checked_sub(guest) {
            Some(left) if !left.is_zero() => {
                let left_ms = u64::try_from(left.as_millis()).unwrap_or(u64::MAX);
                UpdateDeadline::Continue(ms_to_ticks(left_ms).max(1))
            }
            _ => UpdateDeadline::Interrupt,
        }
    }
}

impl ComponentRuntime {
    /// `name`'s plugin, or the trap of a call of `export` that finds none.
    pub(crate) fn plugin(
        &self,
        name: &str,
        export: &str,
    ) -> Result<Arc<Mutex<PluginInstance>>, WasmCallResult> {
        // Release the map lock immediately: holding it across the guest call would serialize every extension behind
        // one mutex (the C7-10 finding). Only the target plugin's own lock is held for the duration of the call.
        let plugins = self.plugins.lock().map_err(|e| {
            WasmCallResult::Trap(WasmTrapInfo {
                kind: "lock_poisoned".to_string(),
                message: e.to_string(),
                export_name: export.to_string(),
            })
        })?;
        plugins.get(name).map(Arc::clone).ok_or_else(|| {
            WasmCallResult::Trap(WasmTrapInfo {
                kind: "extension_not_found".to_string(),
                message: format!("Extension '{name}' not loaded"),
                export_name: export.to_string(),
            })
        })
    }

    /// One effectful call of a guest that imports a port, on an instance of its own, outside the extension's lock: a
    /// trap takes only this instance with it, and nothing of the guest outlives the call (ADR 0335 D4).
    #[expect(
        clippy::too_many_arguments,
        reason = "what the shared path reads off the plugin under its lock"
    )]
    pub(crate) fn call_on_own_instance(
        &self,
        name: &str,
        export: &str,
        input: &[u8],
        component: &Component,
        limits: Limits,
        fuel: u64,
        session: impl FnOnce(Limits) -> PortSession,
    ) -> WasmCallResult {
        let failed = |message: String| {
            WasmCallResult::Trap(WasmTrapInfo {
                kind: "call_failed".to_string(),
                message,
                export_name: export.to_string(),
            })
        };
        let (mut store, bindings) = match self.fresh_instance(component, fuel, limits) {
            Ok(instance) => instance,
            Err(why) => return failed(format!("failed to start {name}: {why}")),
        };
        // What starting the instance spent is not the call's.
        let (call_fuel, deadline_ms) = self.budget(fuel, limits, u32::MAX);
        if let Err(e) = store.set_fuel(call_fuel) {
            return failed(format!("failed to set fuel for {name}: {e}"));
        }
        store.set_epoch_deadline(ms_to_ticks(u64::from(deadline_ms)));
        store.data_mut().clock = GuestClock::of(deadline_ms);
        store.data_mut().ports = session(limits);
        match bindings.call_call(&mut store, name, export, input) {
            Ok(Ok(bytes)) => WasmCallResult::Ok(bytes),
            Ok(Err(message)) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message,
                export_name: export.to_string(),
            }),
            Err(e) => {
                let refused = store.data_mut().memory.refused.take();
                let (kind, message) = stopped(&e, refused, limits, fuel, call_fuel, u32::MAX);
                WasmCallResult::Trap(WasmTrapInfo {
                    kind: kind.to_string(),
                    message,
                    export_name: export.to_string(),
                })
            }
        }
    }

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
        let plugin = match self.plugin(name, export) {
            Ok(plugin) => plugin,
            Err(trap) => return trap,
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
        store.data_mut().clock = GuestClock::of(deadline_ms);
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
                let (kind, message) =
                    stopped(&e, memory_refused, *limits, *fuel, call_fuel, max_ms);
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

/// What stopped a call, named, and said as the limit it ran into: `fuel` of the plugin, `call_fuel` of this call,
/// `max_ms` its caller's time budget.
fn stopped(
    error: &wasmtime::Error,
    memory_refused: Option<usize>,
    limits: Limits,
    fuel: u64,
    call_fuel: u64,
    max_ms: u32,
) -> (&'static str, String) {
    let kind = trap_kind(error, memory_refused);
    let message = match kind {
        "memory_limit_exceeded" => format!(
            "grew past its {} MB memory limit (sandbox_policy.max_memory_mb)",
            limits.memory_mb
        ),
        "fuel_exhausted" if call_fuel < fuel => format!(
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
        _ => error.to_string(),
    };
    (kind, message)
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

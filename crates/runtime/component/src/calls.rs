//! The way into an instance: what one call of an export is held to, and what it answers.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specforge_wasm::PortSession;
use specforge_wasm::runtime::{WasmCallResult, WasmTrapInfo};
use specforge_wasm::sandbox::Limits;

use wasmtime::component::Component;
use wasmtime::{Store, UpdateDeadline};

use crate::{Bridge, ComponentRuntime, HostState, PluginInstance, ms_to_ticks};

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

/// What holds one call: the plugin's limits and fuel, its caller's time budget, and what those leave it to spend.
#[derive(Clone, Copy)]
pub(crate) struct Allowance {
    limits: Limits,
    fuel: u64,
    max_ms: u32,
    call_fuel: u64,
    deadline_ms: u32,
}

/// What a call's trap answers.
fn trap(kind: &str, message: impl Into<String>, export: &str) -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: kind.to_string(),
        message: message.into(),
        export_name: export.to_string(),
    })
}

/// One call into `store`'s instance, its session made from the limits and lasting the call. `Err`: what stopped it, for
/// the caller to do with the instance what it must.
fn enter(
    store: &mut Store<HostState>,
    bindings: &Bridge,
    (name, export): (&str, &str),
    input: &[u8],
    held: Allowance,
    session: impl FnOnce(Limits) -> PortSession,
) -> Result<WasmCallResult, (&'static str, String)> {
    if let Err(e) = store.set_fuel(held.call_fuel) {
        let why = format!("failed to set fuel for {name}: {e}");
        return Ok(trap("call_failed", why, export));
    }
    store.set_epoch_deadline(ms_to_ticks(u64::from(held.deadline_ms)));
    store.data_mut().clock = GuestClock::of(held.deadline_ms);
    store.data_mut().ports = session(held.limits);
    let called = bindings.call_call(&mut *store, name, export, input);
    store.data_mut().ports = PortSession::pure();
    match called {
        Ok(Ok(bytes)) => Ok(WasmCallResult::Ok(bytes)),
        Ok(Err(message)) => Ok(trap("guest_error", message, export)),
        // What stopped the call is read before the instance is replaced: a limit of the sandbox is named, any other
        // fault keeps wasmtime's message.
        Err(e) => {
            let memory_refused = store.data_mut().memory.refused.take();
            Err(stopped(&e, memory_refused, held))
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
        let plugins = self
            .plugins
            .lock()
            .map_err(|e| trap("lock_poisoned", e.to_string(), export))?;
        plugins.get(name).map(Arc::clone).ok_or_else(|| {
            let why = format!("Extension '{name}' not loaded");
            trap("extension_not_found", why, export)
        })
    }

    /// What a call of a plugin of `limits` and `fuel`, its caller's budget `max_ms`, may spend.
    pub(crate) fn allowance(&self, limits: Limits, fuel: u64, max_ms: u32) -> Allowance {
        let (call_fuel, deadline_ms) = self.budget(fuel, limits, max_ms);
        Allowance {
            limits,
            fuel,
            max_ms,
            call_fuel,
            deadline_ms,
        }
    }

    /// One effectful call of a guest that imports a port, on an instance of its own, outside the extension's lock: a
    /// trap takes only this instance with it, and nothing of the guest outlives the call (ADR 0335 D4).
    pub(crate) fn call_on_own_instance(
        &self,
        name: &str,
        export: &str,
        input: &[u8],
        component: &Component,
        held: Allowance,
        session: impl FnOnce(Limits) -> PortSession,
    ) -> WasmCallResult {
        let (mut store, bindings) = match self.fresh_instance(component, held.fuel, held.limits) {
            Ok(instance) => instance,
            Err(why) => {
                let why = format!("failed to start {name}: {why}");
                return trap("call_failed", why, export);
            }
        };
        // What starting the instance spent is not the call's: `enter` sets the fuel again.
        enter(&mut store, &bindings, (name, export), input, held, session)
            .unwrap_or_else(|(kind, message)| trap(kind, message, export))
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
            Err(e) => return trap("lock_poisoned", e.to_string(), export),
        };
        let PluginInstance {
            store,
            bindings,
            limits,
            component,
            fuel,
            ..
        } = &mut *instance;
        // Every call gets the whole fuel budget and the whole wall-clock budget: a long session never spends one call's
        // allowance on the next.
        let held = self.allowance(*limits, *fuel, max_ms);
        let (kind, message) = match enter(store, bindings, (name, export), input, held, session) {
            Ok(answer) => return answer,
            Err(stopped) => stopped,
        };
        // An instance that trapped cannot be entered again: the extension's next call gets a fresh one under the same
        // limits, as a guest that panics in one call must not take the extension down for the rest of the process (an
        // MCP session). One that cannot start (its own limits refuse it) is unloaded, so later calls say it is gone
        // instead of failing on the dead instance.
        let message = match self.fresh_instance(component, *fuel, *limits) {
            Ok((fresh_store, fresh_bindings)) => {
                *store = fresh_store;
                *bindings = fresh_bindings;
                message
            }
            Err(why) => {
                // The plugin's lock goes first: `apply_limits` takes the map's, then a plugin's.
                drop(instance);
                if let Ok(mut plugins) = self.plugins.lock()
                    && plugins.get(name).is_some_and(|p| Arc::ptr_eq(p, &plugin))
                {
                    plugins.remove(name);
                }
                format!("{message}; it could not be restarted ({why}) and is unloaded")
            }
        };
        trap(kind, message, export)
    }
}

/// What stopped a call, named, and said as the limit of `held` it ran into.
fn stopped(
    error: &wasmtime::Error,
    memory_refused: Option<usize>,
    held: Allowance,
) -> (&'static str, String) {
    let Allowance {
        limits,
        fuel,
        max_ms,
        call_fuel,
        ..
    } = held;
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

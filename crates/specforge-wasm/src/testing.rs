//! An in-process adapter of the [`WasmRuntime`] seam, for tests: it serves
//! an extension straight from its SDK [`ContributionsBuilder`], routed by
//! the guest's own [`guest_call`] — the function `component_guest!` calls in
//! a component — so a test declares an extension with the same builders an
//! extension author uses, and the host loads it exactly as it loads a
//! component.
//!
//! It is a test adapter: the "guest" runs in the host process, with no
//! sandbox, no fuel and no deadline. Sandbox obligations stay proven only
//! through the component runtime.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};

use specforge_extension_sdk::{ContributionsBuilder, ExportHandler, guest_call, no_other_exports};

use crate::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};

/// What builds an extension's contributions, per call (as its guest does).
type Build = Arc<dyn Fn() -> ContributionsBuilder + Send + Sync>;

/// One call the runtime answered.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedCall {
    pub extension: String,
    pub export: String,
    /// The input as JSON (a string of the bytes when they are not JSON).
    pub input: serde_json::Value,
}

/// Serves SDK-declared extensions in process; see the module docs.
#[derive(Default)]
pub struct InProcessRuntime {
    extensions: BTreeMap<String, (Build, ExportHandler)>,
    overrides: Mutex<HashMap<(String, String), WasmCallResult>>,
    calls: Mutex<Vec<RecordedCall>>,
}

impl InProcessRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serve the extension `build` declares (named by its `meta.name`),
    /// routed as its guest is, with no exports beyond its declarations.
    pub fn with(self, build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static) -> Self {
        self.with_handler(build, no_other_exports)
    }

    /// Serve the extension `build` declares, with `handler` answering the
    /// exports no declaration answers (`component_guest!`'s `handler`).
    pub fn with_handler(
        mut self,
        build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
        handler: ExportHandler,
    ) -> Self {
        let name = build().meta.name.clone();
        self.extensions.insert(name, (Arc::new(build), handler));
        self
    }

    /// Answer `export` of `extension` with `result` (raw bytes or a trap)
    /// instead of routing it: for failure-mapping tests. The extension
    /// need not be served otherwise.
    pub fn answer_raw(self, extension: &str, export: &str, result: WasmCallResult) -> Self {
        self.overrides
            .lock()
            .expect("overrides lock")
            .insert((extension.to_string(), export.to_string()), result);
        self
    }

    /// Every call answered so far, in order.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("calls lock").clone()
    }
}

fn trap(kind: &str, message: String, export: &str) -> WasmCallResult {
    WasmCallResult::Trap(WasmTrapInfo {
        kind: kind.to_string(),
        message,
        export_name: export.to_string(),
    })
}

impl WasmRuntime for InProcessRuntime {
    fn load_module(&self, wasm_path: &Path) -> Result<(), String> {
        Err(format!(
            "the in-process runtime serves SDK builders, not binaries ({})",
            wasm_path.display()
        ))
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        self.calls.lock().expect("calls lock").push(RecordedCall {
            extension: extension.to_string(),
            export: export.to_string(),
            input: serde_json::from_slice(input)
                .unwrap_or_else(|_| String::from_utf8_lossy(input).into_owned().into()),
        });
        let key = (extension.to_string(), export.to_string());
        if let Some(result) = self.overrides.lock().expect("overrides lock").get(&key) {
            return result.clone();
        }
        let Some((build, handler)) = self.extensions.get(extension) else {
            return trap(
                "extension_not_found",
                format!("Extension '{extension}' not loaded"),
                export,
            );
        };
        // A guest panic is a trap, as `unreachable` is in a component.
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            guest_call(&build(), *handler, export, input)
        }));
        match answer {
            Ok(Ok(bytes)) => WasmCallResult::Ok(bytes),
            Ok(Err(message)) => trap("guest_error", message, export),
            Err(panic) => {
                let message = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                trap("call_failed", format!("unreachable: {message}"), export)
            }
        }
    }
}

/// The contract every adapter of the [`WasmRuntime`] port keeps, asserted
/// over `runtime`, which must serve `extension` (an SDK-built guest) with
/// `panicking` an export whose handler panics:
///
/// - a call to an extension the runtime did not load is the trap
///   `extension_not_found`;
/// - an export the guest does not route is the trap `guest_error`
///   `unknown export '<name>'`, as the guest's own routing answers it;
/// - a handler's error is the trap `guest_error` carrying its message (the
///   guest's `__describe` of a category the protocol does not have);
/// - a guest that panics is a trap (`call_failed`), never a host crash;
/// - the runtime still answers the extension afterwards (its handshake).
///
/// The component runtime and [`InProcessRuntime`] run it in their tests,
/// so the two adapters cannot disagree on what a failure looks like.
pub fn assert_runtime_contract(runtime: &dyn WasmRuntime, extension: &str, panicking: &str) {
    let trap = |result: WasmCallResult, what: &str| match result {
        WasmCallResult::Trap(trap) => trap,
        WasmCallResult::Ok(bytes) => panic!(
            "{what}: answered {} instead of trapping",
            String::from_utf8_lossy(&bytes)
        ),
    };

    let missing = trap(
        runtime.call_export("@contract/not-loaded", "__handshake", b"{}"),
        "an extension that is not loaded",
    );
    assert_eq!(missing.kind, "extension_not_found", "{missing:?}");
    assert_eq!(missing.export_name, "__handshake");

    let unrouted = trap(
        runtime.call_export(extension, "contract__no_such_export", b"{}"),
        "an export the guest does not route",
    );
    assert_eq!(unrouted.kind, "guest_error", "{unrouted:?}");
    assert_eq!(
        unrouted.message,
        "unknown export 'contract__no_such_export'"
    );
    assert_eq!(unrouted.export_name, "contract__no_such_export");

    let refused = trap(
        runtime.call_export(
            extension,
            "__describe",
            br#"{"category":"no_such_category"}"#,
        ),
        "a handler's error",
    );
    assert_eq!(refused.kind, "guest_error", "{refused:?}");
    assert_eq!(refused.message, "unsupported category: no_such_category");

    let panicked = trap(
        runtime.call_export(extension, panicking, b"{}"),
        "a guest that panics",
    );
    assert_eq!(panicked.kind, "call_failed", "{panicked:?}");
    assert_eq!(panicked.export_name, panicking);

    match runtime.call_export(extension, "__handshake", b"{}") {
        WasmCallResult::Ok(bytes) => {
            let handshake: serde_json::Value =
                serde_json::from_slice(&bytes).expect("a handshake is JSON");
            assert_eq!(handshake["name"], extension);
        }
        WasmCallResult::Trap(trap) => panic!("the extension stopped answering: {trap:?}"),
    }
}

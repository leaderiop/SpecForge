//! An in-process adapter of the [`WasmRuntime`] seam, for tests: it serves
//! an extension straight from its SDK [`ContributionsBuilder`], routed by
//! the guest's own [`GuestServed`](specforge_extension_sdk::Served) — the function `component_guest!` calls in
//! a component — so a test declares an extension with the same builders an
//! extension author uses, and the host loads it exactly as it loads a
//! component.
//!
//! It is a test adapter: the "guest" runs in the host process, with no
//! sandbox, no fuel and no deadline (it records the limits the host applies).
//! Sandbox obligations stay proven only through the component runtime.
//!
//! Beside another runtime ([`InProcessRuntime::beside`]), what it cannot serve (a builtin's real component) loads
//! there, so one project mixes extensions served in process with components.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use specforge_extension_sdk::host::in_process::{InProcessHost, serve};
use specforge_extension_sdk::{
    ContributionsBuilder, ExportHandler, ExtensionMeta, PeerDependency, Served as GuestServed,
    no_other_exports,
};
use specforge_protocol_types::{
    FileSize, FsContent, FsEntry, FsListing, GitBlobs, GitRepo, GitStatus, GitTreeEntry, HostError,
    HttpRequest, HttpResponse, Operation, ProcessOutput, ProcessRequest,
};

use crate::ports::{PortCall, PortSession};
use crate::runtime::{ExtensionSettings, LoadError, WasmCallResult, WasmRuntime, WasmTrapInfo};
use crate::sandbox::Limits;
use specforge_effects::Provenance;

/// What builds an extension's contributions, per call (as its guest does).
type Build = Arc<dyn Fn() -> ContributionsBuilder + Send + Sync>;

/// An extension served under a name: what builds it, and what answers the
/// exports its declarations do not.
type Served = (Build, ExportHandler);

/// What starts the bytes [`spelled_module`] makes.
const SPELLED: &[u8] = b"\0asm in-process-spelled ";

/// Bytes that spell a declaration: [`InProcessRuntime`] serves them, under whatever name they are
/// loaded as, as a component declaring `name` at `version` with `peers` and nothing else (after the
/// binaries and names it was given). A module a test installs by hand declares what its lock entry
/// records without the test registering it.
pub fn spelled_module(name: &str, version: &str, peers: &[PeerDependency]) -> Vec<u8> {
    let spelling = serde_json::json!({"name": name, "version": version, "peers": peers});
    [SPELLED, spelling.to_string().as_bytes()].concat()
}

/// The declaration `bytes` spell, if they are [`spelled_module`] bytes.
fn spelled(bytes: &[u8]) -> Option<ExtensionMeta> {
    let spelling: serde_json::Value = serde_json::from_slice(bytes.strip_prefix(SPELLED)?).ok()?;
    let mut meta = ExtensionMeta::new(spelling["name"].as_str()?, spelling["version"].as_str()?);
    meta.peer_dependencies = serde_json::from_value(spelling["peers"].clone()).ok()?;
    Some(meta)
}

/// One call the runtime answered.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedCall {
    pub extension: String,
    pub export: String,
    /// The input as JSON (a string of the bytes when they are not JSON).
    pub input: serde_json::Value,
}

/// An answer given instead of routing a call: to one export, or to one
/// export called with one input.
type Overrides = Vec<(String, String, Option<serde_json::Value>, WasmCallResult)>;

/// Serves SDK-declared extensions in process; see the module docs.
#[derive(Default)]
pub struct InProcessRuntime {
    /// The extensions served, by the name each is loaded as. Behind a lock:
    /// loading, renaming and unloading change it through the port.
    extensions: Mutex<BTreeMap<String, Served>>,
    /// Binaries that serve an extension under whatever name they are
    /// loaded as ([`InProcessRuntime::binary`]).
    binaries: Vec<(Vec<u8>, Served)>,
    /// Bytes whose load answers a refusal ([`InProcessRuntime::refusing`]).
    refusals: Vec<(Vec<u8>, LoadError)>,
    overrides: Mutex<Overrides>,
    faults: Vec<(String, String)>,
    calls: Mutex<Vec<RecordedCall>>,
    limits: Mutex<Vec<(String, Limits)>>,
    settings: Mutex<std::collections::HashMap<String, ExtensionSettings>>,
    /// The runtime a load this one cannot serve goes to ([`InProcessRuntime::beside`]).
    beside: Option<Beside>,
    /// Each loaded name's provenance, as [`WasmRuntime::load`] was given it.
    provenances: Mutex<BTreeMap<String, Provenance>>,
    /// The `specforge:host` interfaces a served extension imports ([`InProcessRuntime::importing`]).
    imports: BTreeMap<String, Vec<String>>,
}

/// The runtime beside an in-process one, and the names loaded in it, whose calls, limits, settings, renames and
/// unloads go there.
struct Beside {
    runtime: Arc<dyn WasmRuntime>,
    loaded: Mutex<BTreeSet<String>>,
}

impl Beside {
    fn loaded(&self) -> std::sync::MutexGuard<'_, BTreeSet<String>> {
        self.loaded.lock().expect("beside lock")
    }

    /// `name` loads in process now: what was loaded beside under it is unloaded there.
    fn forget(&self, name: &str) {
        if self.loaded().remove(name) {
            self.runtime.unload(name);
        }
    }
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
        self,
        build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
        handler: ExportHandler,
    ) -> Self {
        let name = build().meta.name.clone();
        self.serving(&name, build, handler)
    }

    /// Serve the extension `build` declares under `name`, the name the
    /// host loads it by, which need not be the one it declares (a binary
    /// installed from a path), with `handler` as in
    /// [`InProcessRuntime::with_handler`].
    pub fn serving(
        mut self,
        name: &str,
        build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
        handler: ExportHandler,
    ) -> Self {
        self.extensions
            .get_mut()
            .expect("extensions lock")
            .insert(name.to_string(), (Arc::new(build), handler));
        self
    }

    /// Serve `build` under whatever name a module of exactly `bytes` is
    /// loaded as: a `.wasm` file whose content a test chooses, installed
    /// and loaded as any extension is.
    pub fn binary(
        mut self,
        bytes: &[u8],
        build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
    ) -> Self {
        self.binaries
            .push((bytes.to_vec(), (Arc::new(build), no_other_exports)));
        self
    }

    /// A load of exactly `bytes` answers `error`: the in-process seam for a refusal the component runtime gives
    /// (an unknown host port, a foreign import), as [`Self::binary`] is for a declaration.
    pub fn refusing(mut self, bytes: &[u8], error: LoadError) -> Self {
        self.refusals.push((bytes.to_vec(), error));
        self
    }

    /// `name`'s guest imports `interfaces` of `specforge:host`: what the component runtime reads off a component.
    pub fn importing(mut self, name: &str, interfaces: &[&str]) -> Self {
        self.imports.insert(
            name.to_string(),
            interfaces.iter().map(|i| i.to_string()).collect(),
        );
        self
    }

    /// Load in `runtime` what this one cannot serve (bytes that are no binary, served name or spelled module it
    /// knows: a builtin's component), and route that name there from then on. Its calls are recorded here too.
    pub fn beside(mut self, runtime: Arc<dyn WasmRuntime>) -> Self {
        self.beside = Some(Beside {
            runtime,
            loaded: Mutex::default(),
        });
        self
    }

    /// The runtime beside this one, when `name` was loaded in it.
    fn provenances(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Provenance>> {
        self.provenances.lock().expect("provenances lock")
    }

    fn beside_of(&self, name: &str) -> Option<&Beside> {
        self.beside.as_ref().filter(|b| b.loaded().contains(name))
    }

    /// The runtime `name` was loaded in beside this one, if it was.
    fn delegate(&self, name: &str) -> Option<&dyn WasmRuntime> {
        self.beside_of(name).map(|b| b.runtime.as_ref())
    }

    /// Serve `name` in process, if `bytes` are a binary it was given or spell a module, or it serves or answers that
    /// name already ([`WasmRuntime::load`]); false when only the runtime beside can.
    fn load_here(&self, name: &str, bytes: &[u8]) -> bool {
        let mut extensions = self.extensions.lock().expect("extensions lock");
        if let Some((_, served)) = self.binaries.iter().find(|(known, _)| known == bytes) {
            extensions.insert(name.to_string(), served.clone());
            return true;
        }
        let answered = self
            .overrides
            .lock()
            .expect("overrides lock")
            .iter()
            .any(|(extension, ..)| extension == name);
        if extensions.contains_key(name) || answered {
            return true;
        }
        if let Some(meta) = spelled(bytes) {
            let build: Build = Arc::new(move || ContributionsBuilder::new(meta.clone()));
            extensions.insert(name.to_string(), (build, no_other_exports));
            return true;
        }
        false
    }

    /// `runtime` as the in-process runtime, if it is one.
    pub fn of(runtime: &dyn WasmRuntime) -> Option<&InProcessRuntime> {
        (runtime as &dyn std::any::Any).downcast_ref()
    }

    /// The version and peers the extension declares once a module of `bytes` is loaded as `name`: a
    /// binary it was given, else the name it serves, else the declaration `bytes` spell
    /// ([`spelled_module`]). Nothing is loaded and no call is recorded, so a test can write the lock
    /// entry a real install would.
    pub fn declared_by(&self, name: &str, bytes: &[u8]) -> Option<(String, Vec<PeerDependency>)> {
        let built = |build: &Build| build().meta;
        let meta = match self.binaries.iter().find(|(known, _)| known == bytes) {
            Some((_, (build, _))) => built(build),
            None => match self.extensions.lock().expect("extensions lock").get(name) {
                Some((build, _)) => built(build),
                None => spelled(bytes)?,
            },
        };
        Some((meta.version, meta.peer_dependencies))
    }

    /// Answer `export` of `extension` with `result` (raw bytes or a trap)
    /// instead of routing it: for failure-mapping tests. The extension
    /// need not be served otherwise.
    pub fn answer_raw(self, extension: &str, export: &str, result: WasmCallResult) -> Self {
        self.overriding(extension, export, None, result)
    }

    /// Answer `export` of `extension` with `result` when it is called with
    /// `input` (compared as JSON), and route its other calls: one describe
    /// category answering what its declaration could not hold.
    pub fn answer_raw_to(
        self,
        extension: &str,
        export: &str,
        input: serde_json::Value,
        result: WasmCallResult,
    ) -> Self {
        self.overriding(extension, export, Some(input), result)
    }

    fn overriding(
        self,
        extension: &str,
        export: &str,
        input: Option<serde_json::Value>,
        result: WasmCallResult,
    ) -> Self {
        self.overrides.lock().expect("overrides lock").push((
            extension.to_string(),
            export.to_string(),
            input,
            result,
        ));
        self
    }

    /// Make calling `export` of `extension` panic in the host, outside the
    /// guest: a fault of the runtime itself (a broken host function), not a
    /// guest trap, for tests of what the host does when its runtime fails.
    pub fn fault(mut self, extension: &str, export: &str) -> Self {
        self.faults
            .push((extension.to_string(), export.to_string()));
        self
    }

    /// Every call answered so far, in order.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("calls lock").clone()
    }

    /// Forget the calls answered so far.
    pub fn clear_calls(&self) {
        self.calls.lock().expect("calls lock").clear();
    }

    /// Every limit the host applied (`(extension, limits)`), in order. The
    /// in-process runtime records them; it enforces none.
    pub fn limits(&self) -> Vec<(String, Limits)> {
        self.limits.lock().expect("limits lock").clone()
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
    /// A binary this runtime was given ([`InProcessRuntime::binary`]) is
    /// served under `name`; a name it serves, or answers raw
    /// ([`InProcessRuntime::answer_raw`]), loads as it is (an extension
    /// served in process is whatever binary is installed under its name);
    /// any other bytes are no component it serves.
    fn load(&self, name: &str, bytes: &[u8], provenance: Provenance) -> Result<(), LoadError> {
        if let Some((_, error)) = self.refusals.iter().find(|(known, _)| known == bytes) {
            return Err(error.clone());
        }
        if self.load_here(name, bytes) {
            if let Some(beside) = &self.beside {
                beside.forget(name);
            }
            self.provenances().insert(name.to_string(), provenance);
            return Ok(());
        }
        let Some(beside) = &self.beside else {
            return Err(LoadError::Unloadable(
                "not a component the in-process runtime serves".to_string(),
            ));
        };
        beside.runtime.load(name, bytes, provenance.clone())?;
        beside.loaded().insert(name.to_string());
        self.provenances().insert(name.to_string(), provenance);
        Ok(())
    }

    fn provenance(&self, name: &str) -> Option<Provenance> {
        self.provenances().get(name).cloned()
    }

    fn rename(&self, from: &str, to: &str) -> bool {
        let mut provenances = self.provenances();
        if let Some(provenance) = provenances.remove(from) {
            provenances.insert(to.to_string(), provenance);
        }
        drop(provenances);
        if let Some(beside) = self.beside_of(from) {
            let mut loaded = beside.loaded();
            loaded.remove(from);
            loaded.insert(to.to_string());
            return beside.runtime.rename(from, to);
        }
        let mut extensions = self.extensions.lock().expect("extensions lock");
        match extensions.remove(from) {
            Some(served) => {
                extensions.insert(to.to_string(), served);
                true
            }
            None => false,
        }
    }

    fn unload(&self, name: &str) -> bool {
        self.provenances().remove(name);
        if let Some(beside) = self.beside_of(name) {
            beside.loaded().remove(name);
            return beside.runtime.unload(name);
        }
        self.extensions
            .lock()
            .expect("extensions lock")
            .remove(name)
            .is_some()
    }

    fn apply_limits(&self, extension: &str, limits: Limits) {
        if let Some(beside) = self.delegate(extension) {
            beside.apply_limits(extension, limits);
        }
        self.limits
            .lock()
            .expect("limits lock")
            .push((extension.to_string(), limits));
    }

    fn apply_settings(&self, extension: &str, settings: ExtensionSettings) {
        if let Some(beside) = self.delegate(extension) {
            beside.apply_settings(extension, settings.clone());
        }
        self.settings
            .lock()
            .expect("settings lock")
            .insert(extension.to_string(), settings);
    }

    fn settings_of(&self, extension: &str) -> Option<ExtensionSettings> {
        self.settings
            .lock()
            .expect("settings lock")
            .get(extension)
            .cloned()
    }

    fn call_export(&self, extension: &str, export: &str, input: &[u8]) -> WasmCallResult {
        self.answer(extension, export, input, None)
    }

    fn call_with_ports(
        &self,
        extension: &str,
        export: &str,
        input: &[u8],
        kind: Operation,
        lent: Option<PortCall>,
    ) -> WasmCallResult {
        self.answer(extension, export, input, Some((kind, lent)))
    }

    fn host_imports(&self, extension: &str) -> Vec<String> {
        match self.delegate(extension) {
            Some(beside) => beside.host_imports(extension),
            None => self.imports.get(extension).cloned().unwrap_or_default(),
        }
    }
}

/// A call's [`PortSession`] as the guest's seam on the thread reads it.
struct ServedPorts(RefCell<PortSession>);

impl InProcessHost for ServedPorts {
    fn http_send(&self, request: &HttpRequest) -> Result<HttpResponse, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.http(request.clone()))
    }
    fn fs_stat(&self, root: &str, path: &str) -> Result<Option<FsEntry>, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.fs_stat(root, path))
    }
    fn fs_read(&self, root: &str, path: &str, max_bytes: u64) -> Result<FsContent, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.fs_read(root, path, max_bytes))
    }
    fn fs_walk(
        &self,
        root: &str,
        path: &str,
        glob: Option<&str>,
        max_entries: u32,
    ) -> Result<FsListing, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.fs_walk(root, path, glob, max_entries))
    }
    fn process_run(&self, request: &ProcessRequest) -> Result<ProcessOutput, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.process_run(request.clone()))
    }
    fn git_locate(&self) -> Result<Option<GitRepo>, HostError> {
        self.0.borrow_mut().timed(|session| session.git_locate())
    }
    fn git_commit(&self, revision: &str) -> Result<Option<String>, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.git_commit(revision))
    }
    fn git_merge_base(&self, a: &str, b: &str) -> Result<Option<String>, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.git_merge_base(a, b))
    }
    fn git_tree(&self, commit: &str, dir: &str) -> Result<Vec<GitTreeEntry>, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.git_tree(commit, dir))
    }
    fn git_sizes(&self, commit: &str, paths: &[String]) -> Result<Vec<FileSize>, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.git_sizes(commit, paths))
    }
    fn git_blobs(&self, commit: &str, paths: &[String]) -> Result<GitBlobs, HostError> {
        self.0
            .borrow_mut()
            .timed(|session| session.git_blobs(commit, paths))
    }
    fn git_branch(&self) -> Result<String, HostError> {
        self.0.borrow_mut().timed(|session| session.git_branch())
    }
    fn git_status(&self) -> Result<GitStatus, HostError> {
        self.0.borrow_mut().timed(|session| session.git_status())
    }
}

impl InProcessRuntime {
    /// The limits the host last applied to `extension`: what a session holds a port's answer to.
    fn limits_of(&self, extension: &str) -> Limits {
        self.limits()
            .into_iter()
            .rev()
            .find(|(name, _)| name == extension)
            .map_or(Limits::CEILING, |(_, limits)| limits)
    }

    /// One call, recorded and routed; `ports` is its kind and what it was lent, for a call that may use a port.
    fn answer(
        &self,
        extension: &str,
        export: &str,
        input: &[u8],
        ports: Option<(Operation, Option<PortCall>)>,
    ) -> WasmCallResult {
        let input_json: serde_json::Value = serde_json::from_slice(input)
            .unwrap_or_else(|_| String::from_utf8_lossy(input).into_owned().into());
        self.calls.lock().expect("calls lock").push(RecordedCall {
            extension: extension.to_string(),
            export: export.to_string(),
            input: input_json.clone(),
        });
        if self
            .faults
            .iter()
            .any(|(e, x)| e == extension && x == export)
        {
            panic!("the runtime failed calling {export} of {extension}");
        }
        let overridden = self
            .overrides
            .lock()
            .expect("overrides lock")
            .iter()
            .find(|(e, x, when, _)| {
                e == extension && x == export && when.as_ref().is_none_or(|w| *w == input_json)
            })
            .map(|(_, _, _, result)| result.clone());
        if let Some(result) = overridden {
            return result;
        }
        if let Some(beside) = self.delegate(extension) {
            return match ports {
                Some((kind, lent)) => beside.call_with_ports(extension, export, input, kind, lent),
                None => beside.call_export(extension, export, input),
            };
        }
        let served = self
            .extensions
            .lock()
            .expect("extensions lock")
            .get(extension)
            .cloned();
        let Some((build, handler)) = served else {
            return trap(
                "extension_not_found",
                format!("Extension '{extension}' not loaded"),
                export,
            );
        };
        // A guest panic is a trap, as `unreachable` is in a component.
        let session = ports.map(|(kind, lent)| {
            PortSession::effectful(extension, kind, lent, self.limits_of(extension))
        });
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let guest = || GuestServed::new(build()).call(handler, export, input);
            match session {
                Some(session) => serve(Rc::new(ServedPorts(RefCell::new(session))), guest),
                None => guest(),
            }
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

/// The contract every adapter keeps for a port call, asserted over `runtime`, which must serve `extension`, a guest
/// whose `export` calls the HTTP port on its input's `target` and answers `{"ok": status}`, `{"denied": kind}` or
/// `{"failed": class}` (the sandbox probe's `ports` command, and its in-process twin). `lent` lends a grant that
/// permits `inside` and not `outside`:
///
/// - a command inside its grant reaches the port;
/// - outside it, it is answered `denied(scope)` and one E090 is kept for the caller;
/// - with nothing lent, it is answered `denied(scope)`;
/// - a call kind no port is for, and a plain call, are answered `denied(call-kind)`.
pub fn assert_port_contract(
    runtime: &dyn WasmRuntime,
    extension: &str,
    export: &str,
    lent: &dyn Fn() -> PortCall,
    inside: &str,
    outside: &str,
) {
    let input = |url: &str| serde_json::json!({"port": "http", "target": url}).to_string();
    let answer = |result: WasmCallResult, what: &str| match result {
        WasmCallResult::Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap_or_else(|e| panic!("{what}: {e}")),
        WasmCallResult::Trap(trap) => panic!("{what}: {trap:?}"),
    };

    let call = |url: &str, kind, lent| {
        runtime.call_with_ports(extension, export, input(url).as_bytes(), kind, lent)
    };
    let reached = answer(
        call(inside, Operation::Command, Some(lent())),
        "inside the grant",
    );
    assert_eq!(reached["ok"], 200, "inside the grant: {reached}");

    let port_call = lent();
    let refused = answer(
        call(outside, Operation::Command, Some(port_call.clone())),
        "outside the grant",
    );
    assert_eq!(refused["denied"], "scope", "outside the grant: {refused}");
    let kept = port_call.take_denials();
    assert_eq!(kept.len(), 1, "one E090 is kept: {kept:?}");
    assert_eq!(kept[0].code, specforge_common::codes::E090.id());

    let unlent = answer(call(inside, Operation::Command, None), "nothing lent");
    assert_eq!(unlent["denied"], "scope", "nothing lent: {unlent}");

    let pure = answer(call(inside, Operation::Pass, Some(lent())), "a pure kind");
    assert_eq!(pure["denied"], "call_kind", "a pure kind: {pure}");

    let plain = answer(
        runtime.call_export(extension, export, input(inside).as_bytes()),
        "a plain call",
    );
    assert_eq!(plain["denied"], "call_kind", "a plain call: {plain}");
}

/// The contract every adapter of the [`WasmRuntime`] port keeps for the
/// modules it holds, asserted over `runtime`, which must load `bytes` as a
/// component declaring `declared`:
///
/// - loading `bytes` under a name registers them there, and the extension
///   answers its handshake under that name;
/// - a renamed module answers under its new name and no longer under the
///   old one; renaming what is not loaded is false;
/// - an unloaded module is not loaded any more; unloading twice is false.
pub fn assert_module_contract(runtime: &dyn WasmRuntime, bytes: &[u8], declared: &str) {
    let answers = |name: &str| match runtime.call_export(name, "__handshake", b"{}") {
        WasmCallResult::Ok(answer) => {
            let handshake: serde_json::Value =
                serde_json::from_slice(&answer).expect("a handshake is JSON");
            assert_eq!(handshake["name"], declared);
            true
        }
        WasmCallResult::Trap(trap) => {
            assert_eq!(trap.kind, "extension_not_found", "{trap:?}");
            false
        }
    };

    assert!(!answers("loaded-as"), "nothing is loaded yet");
    let provenance = Provenance::third_party(bytes);
    runtime
        .load("loaded-as", bytes, provenance.clone())
        .expect("the bytes are a component");
    assert!(answers("loaded-as"), "loading registers the bytes by name");
    assert_eq!(runtime.provenance("loaded-as"), Some(provenance.clone()));

    assert!(!runtime.rename("not-loaded", "elsewhere"));
    assert!(runtime.rename("loaded-as", "renamed"));
    assert!(
        answers("renamed"),
        "a renamed module answers as its new name"
    );
    assert!(!answers("loaded-as"), "and no longer as the old one");
    assert_eq!(
        runtime.provenance("renamed"),
        Some(provenance),
        "a rename carries the provenance"
    );
    assert_eq!(runtime.provenance("loaded-as"), None);

    assert!(runtime.unload("renamed"));
    assert!(!answers("renamed"), "an unloaded module is not loaded");
    assert!(!runtime.unload("renamed"), "nothing is left to unload");
    assert_eq!(runtime.provenance("renamed"), None, "an unload drops it");
}

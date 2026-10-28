use crate::sandbox::Limits;

/// Result of calling a Wasm export function.
#[derive(Debug, Clone)]
pub enum WasmCallResult {
    Ok(Vec<u8>),
    Trap(WasmTrapInfo),
}

/// Details of a Wasm trap.
#[derive(Debug, Clone)]
pub struct WasmTrapInfo {
    pub kind: String,
    pub message: String,
    pub export_name: String,
}

/// What the host sends an extension on every call but its handshake and describe (ADR 0288): its effective
/// settings, and which of its secrets are set in the environment, never the values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionSettings {
    /// Every setting with a value, by name (a family's keys written out).
    pub values: serde_json::Map<String, serde_json::Value>,
    /// Each declared secret: its name and the environment variable it is read from.
    pub secrets: Vec<(String, String)>,
}

impl ExtensionSettings {
    /// Whether the extension has nothing to be sent.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.secrets.is_empty()
    }

    /// Whether the environment variable `var` a secret is read from is set and not empty. The value is read
    /// nowhere else.
    pub fn secret_is_set(var: &str) -> bool {
        std::env::var_os(var).is_some_and(|v| !v.is_empty())
    }

    /// The members the host adds to a call's input object: `settings`, and with `secrets` the presence of each
    /// declared secret (name -> set), the environment read now. An extension that declares no secret has no
    /// `secrets` member.
    ///
    /// Presence is the machine's, not the project's: CI has a token a laptop lacks. Nothing the host hashes,
    /// records or baselines holds it, so an input that carries it cannot be reproduced from the project. The
    /// caller adds it only to what a person runs on a machine (commands, tools, resources, prompts), never to a pass,
    /// validator, collector, scanner or migration, whose results must be the same on every machine.
    pub fn members(&self, secrets: bool) -> serde_json::Map<String, serde_json::Value> {
        let mut members = serde_json::Map::new();
        if !self.values.is_empty() {
            members.insert("settings".into(), self.values.clone().into());
        }
        if secrets && !self.secrets.is_empty() {
            let set = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
            let secrets = self
                .secrets
                .iter()
                .map(|(name, var)| (name.clone(), set(var).into()));
            members.insert(
                "secrets".into(),
                serde_json::Value::Object(secrets.collect()),
            );
        }
        members
    }
}

/// Testable abstraction over a Wasm runtime (wasmtime component engine).
///
/// Compilation caching is a host-engine concern (wasmtime's native on-disk
/// cache, configured at `ComponentRuntime` construction) — it is not part of
/// the load contract.
///
/// `Any` lets a test ask a shared runtime whether it is the in-process one
/// ([`crate::testing::InProcessRuntime::of`]).
pub trait WasmRuntime: std::any::Any + Send + Sync {
    /// Compile the component `bytes` and register it under `name`,
    /// replacing what was loaded under it. The bytes are the ones the
    /// caller checked: the runtime reads nothing else. A refusal's reason
    /// names no key: the caller names its subject.
    fn load(&self, name: &str, bytes: &[u8]) -> Result<(), String>;

    /// Register what is loaded as `from` under `to` instead, without
    /// compiling it again. False when `from` is not loaded.
    fn rename(&self, from: &str, to: &str) -> bool;

    /// Drop what is loaded as `name`. False when nothing was.
    fn unload(&self, name: &str) -> bool;

    /// Call an export function on a loaded module.
    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult;

    /// [`Self::call_export`] held to at most `max_ms` of wall-clock time, below the extension's own limit (a hook's
    /// timeout, ADR 0292). The in-process runtime runs the guest in the host process and holds it to nothing.
    fn call_export_within(
        &self,
        extension_name: &str,
        export_name: &str,
        input: &[u8],
        _max_ms: u32,
    ) -> WasmCallResult {
        self.call_export(extension_name, export_name, input)
    }

    /// Hold `extension_name`'s later calls to `limits` (its handshake's
    /// sandbox, ADR 0037): the component runtime enforces them, the
    /// in-process runtime records them (its guest runs in the host process
    /// and is held to nothing). An extension that is not loaded is ignored.
    fn apply_limits(&self, extension_name: &str, limits: Limits);

    /// Hold `settings` as what `extension_name`'s later calls carry (ADR 0288). Held beside the limits, not in
    /// the instance, so a restarted instance carries them too. An extension that is not loaded is ignored.
    fn apply_settings(&self, extension_name: &str, settings: ExtensionSettings);

    /// What [`Self::apply_settings`] gave `extension_name`; `None` when nothing was applied.
    fn settings_of(&self, extension_name: &str) -> Option<ExtensionSettings>;
}
